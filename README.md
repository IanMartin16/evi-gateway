# evi-gate

Secure API Gateway with API key authentication and scope validation. Structs: RouteConfig { service_name: String, path_prefix: String, target_url: String, required_scopes: Vec<String>, auth_required: bool, timeout_ms: u64 }, ApiClient { client_id: String, api_key: String, scopes: Vec<String> }, GatewayError { code: String, message: String, request_id: String }, ProxyResponse { status: u16, body: String }. Parse EVIGATE_API_KEYS env var with format client_id:api_key:scope1,scope2 semicolon-separated. Endpoints: GET /api/routes returns registered RouteConfig list, POST /api/proxy requires X-API-Key header validates against route allowlist and required scopes returns GatewayError if unauthorized or route not found, GET /api/health returns each route service_name and status up or down. Default route: service_name mcpone path_prefix /mcpone target_url http://localhost:8000 required_scopes mcpone.execute auth_required true timeout_ms 5000. Reject arbitrary target URLs only proxy to registered routes. Return standardized GatewayError JSON for all error cases.

## Stack

- **Framework**: actix-web 4
- **Language**: Rust (edition 2021)
- **Author**: Martin
- **Version**: 0.1.0

## Getting Started

```bash
cp .env.example .env
cargo run
```

## Build

```bash
cargo build --release
./target/release/evi-gate
```

## Endpoints

| Method | Path | Description |
|---|---|---|
| GET | `/api/health` | Health check |

## API Docs

Swagger UI available at `http://localhost:8080/swagger-ui/`

## Docker

```bash
docker build -t evi-gate .
docker run -p 8080:8080 --env-file .env evi-gate
```
