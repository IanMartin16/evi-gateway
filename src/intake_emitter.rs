//! Emisor de telemetría hacia evi-intake.
//!
//! Versión para **actix-web** + `log` (el stack real de evi-gateway).
//!
//! # Principio: jamás afectar el request path
//!
//! Si evi-intake está caído, lento o saturado, evi-gateway sigue operando igual:
//!   - `emit()` no es async y nunca bloquea: `try_send` a un canal interno
//!   - si el canal está lleno, se descarta y se cuenta (no se acumula)
//!   - el POST vive en una task de fondo con timeout
//!   - un fallo de red se registra y se sigue; sin reintentos infinitos
//!
//! # Batching
//!
//! Un POST por request sería desperdicio. Junta hasta `batch_size` envelopes
//! (o espera `flush_interval`) y manda un solo POST con el array.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use log::{debug, info, warn};
use serde::Serialize;
use serde_json::Value as JsonValue;
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use uuid::Uuid;

/// Clase del dato. Determina la durabilidad DEL LADO DE INTAKE:
/// `Event` se escribe a WAL antes del ACK; `Metric` va directo al buffer.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DataClass {
    /// Métricas de salud, latencia, contadores. Pérdida tolerable.
    Metric,
    /// Eventos que alimentarán la memoria de MCPOne. No deben perderse.
    Event,
}

/// Lo que construye el llamador. El `source` lo inyecta el emisor.
#[derive(Debug, Clone)]
pub struct TelemetryEvent {
    pub class: DataClass,
    pub source_id: String,
    pub user_id: Option<Uuid>,
    pub capability: Option<String>,
    pub intent_family: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub payload: JsonValue,
}

impl TelemetryEvent {
    pub fn new(class: DataClass, source_id: impl Into<String>, payload: JsonValue) -> Self {
        Self {
            class,
            source_id: source_id.into(),
            user_id: None,
            capability: None,
            intent_family: None,
            timestamp: Utc::now(),
            payload,
        }
    }

    pub fn with_user(mut self, user_id: Option<Uuid>) -> Self {
        self.user_id = user_id;
        self
    }

    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capability = Some(capability.into());
        self
    }

    pub fn with_intent(mut self, intent: impl Into<String>) -> Self {
        self.intent_family = Some(intent.into());
        self
    }
}

/// Shape exacto que espera evi-intake (espejo de `intake_contract::Envelope`).
#[derive(Debug, Clone, Serialize)]
struct Envelope {
    class: DataClass,
    source: String,
    source_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    capability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    intent_family: Option<String>,
    timestamp: DateTime<Utc>,
    payload: JsonValue,
}

#[derive(Debug, Clone)]
pub struct EmitterConfig {
    /// URL completa, ej: http://evi-intake.railway.internal:8090/api/v1/telemetry
    pub intake_url: String,
    /// module_id en el registry de MCPOne. Debe coincidir con la política de
    /// redacción de evi-intake o el payload se descarta completo.
    pub source: String,
    pub queue_capacity: usize,
    pub batch_size: usize,
    pub flush_interval: Duration,
    /// Corto a propósito: intake lento no debe acumular tasks aquí.
    pub request_timeout: Duration,
    /// Kill switch. En false, `emit()` es no-op sin costo.
    pub enabled: bool,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            intake_url: "http://localhost:8090/api/v1/telemetry".into(),
            source: "evi-gateway".into(),
            queue_capacity: 4096,
            batch_size: 100,
            flush_interval: Duration::from_secs(5),
            request_timeout: Duration::from_secs(3),
            enabled: true,
        }
    }
}

impl EmitterConfig {
    /// Variables de entorno:
    ///   INTAKE_URL      (default: localhost:8090)
    ///   INTAKE_SOURCE   (default: evi-gateway)
    ///   INTAKE_ENABLED  (default: true; "false"/"0"/"no" lo apaga)
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(url) = std::env::var("INTAKE_URL") {
            cfg.intake_url = url;
        }
        if let Ok(src) = std::env::var("INTAKE_SOURCE") {
            cfg.source = src;
        }
        if let Ok(enabled) = std::env::var("INTAKE_ENABLED") {
            cfg.enabled = !matches!(enabled.to_lowercase().as_str(), "false" | "0" | "no");
        }
        cfg
    }
}

/// Handle clonable. Se registra en actix con `web::Data::new(...)`.
#[derive(Clone)]
pub struct IntakeEmitter {
    tx: Option<mpsc::Sender<Envelope>>,
    source: String,
    dropped: Arc<AtomicU64>,
}

impl IntakeEmitter {
    /// Arranca el emisor y su task de fondo.
    ///
    /// Debe llamarse DENTRO del runtime (o sea, dentro de `#[actix_web::main]`),
    /// no antes. actix-rt está construido sobre tokio, así que `tokio::spawn`
    /// encuentra el runtime correctamente.
    pub fn start(config: EmitterConfig) -> Self {
        let dropped = Arc::new(AtomicU64::new(0));

        if !config.enabled {
            info!("emisor de telemetría deshabilitado (INTAKE_ENABLED=false)");
            return Self {
                tx: None,
                source: config.source,
                dropped,
            };
        }

        let (tx, rx) = mpsc::channel(config.queue_capacity);
        let source = config.source.clone();
        let url = config.intake_url.clone();

        tokio::spawn(run_emitter(rx, config, dropped.clone()));
        info!("emisor de telemetría arrancado: source={} url={}", source, url);

        Self {
            tx: Some(tx),
            source,
            dropped,
        }
    }

    /// Encola un evento. NO bloquea, NO es async, NO falla hacia el llamador.
    pub fn emit(&self, event: TelemetryEvent) {
        let Some(tx) = &self.tx else {
            return;
        };

        let envelope = Envelope {
            class: event.class,
            source: self.source.clone(),
            source_id: event.source_id,
            user_id: event.user_id,
            capability: event.capability,
            intent_family: event.intent_family,
            timestamp: event.timestamp,
            payload: event.payload,
        };

        if tx.try_send(envelope).is_err() {
            let n = self.dropped.fetch_add(1, Ordering::Relaxed) + 1;
            if n % 100 == 1 {
                warn!("telemetría descartada (cola llena o emisor caído); total={}", n);
            }
        }
    }

    /// Exponer en el /health del gateway: si crece, intake no da abasto.
    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// Task de fondo
// ---------------------------------------------------------------------------

async fn run_emitter(
    mut rx: mpsc::Receiver<Envelope>,
    config: EmitterConfig,
    dropped: Arc<AtomicU64>,
) {
    let client = match reqwest::Client::builder()
        .timeout(config.request_timeout)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("no se pudo construir el cliente HTTP; emisor inactivo: {}", e);
            return;
        }
    };

    let mut batch: Vec<Envelope> = Vec::with_capacity(config.batch_size);
    let mut ticker = tokio::time::interval(config.flush_interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            received = rx.recv() => {
                match received {
                    Some(env) => {
                        batch.push(env);

                        // Oportunista: vaciar lo que ya esté listo
                        while batch.len() < config.batch_size {
                            match rx.try_recv() {
                                Ok(e) => batch.push(e),
                                Err(_) => break,
                            }
                        }

                        if batch.len() >= config.batch_size {
                            send_batch(&client, &config, &mut batch, &dropped).await;
                        }
                    }
                    None => {
                        if !batch.is_empty() {
                            send_batch(&client, &config, &mut batch, &dropped).await;
                        }
                        debug!("emisor de telemetría terminado");
                        break;
                    }
                }
            }

            _ = ticker.tick() => {
                if !batch.is_empty() {
                    send_batch(&client, &config, &mut batch, &dropped).await;
                }
            }
        }
    }
}

async fn send_batch(
    client: &reqwest::Client,
    config: &EmitterConfig,
    batch: &mut Vec<Envelope>,
    dropped: &Arc<AtomicU64>,
) {
    if batch.is_empty() {
        return;
    }
    let size = batch.len();

    let result = client.post(&config.intake_url).json(&*batch).send().await;

    match result {
        Ok(resp) if resp.status().is_success() => {
            debug!("batch de telemetría enviado: size={} status={}", size, resp.status());
        }
        Ok(resp) => {
            // 4xx = contrato mal formado (bug nuestro). 5xx/503 = intake saturado.
            dropped.fetch_add(size as u64, Ordering::Relaxed);
            warn!("evi-intake rechazó el batch: size={} status={}", size, resp.status());
        }
        Err(e) => {
            // Sin reintentos: la observabilidad no debe presionar a un servicio caído.
            dropped.fetch_add(size as u64, Ordering::Relaxed);
            warn!("no se pudo enviar telemetría a evi-intake: size={} error={}", size, e);
        }
    }

    batch.clear();
}
