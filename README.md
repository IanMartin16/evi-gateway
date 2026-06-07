# evi-gateway

## evi-gateway is the Rust-based validation, routing and observability gateway for the Evilink ecosystem.

## It acts as a controlled entry layer between Nexus, MCPOne, internal APIs, MCP-compatible services and future IO/SSC components.

## The goal of evi-gateway is not to replace MCPOne or become the orchestration brain. Its role is to validate, route, trace and protect internal service calls.

## Nexus
  ↓
evi-gateway
  ↓
MCPOne
  ↓
Providers / MCPs / APIs / Internal Services
---

## Current status

## evi-gateway is currently in an early operational stage.

## Validated capabilities:

* Rust + Actix Web base
* Docker-ready build
* Swagger / OpenAPI support
* Health endpoint aligned with Status-Hub
* Controlled route registry
* API key validation
* Scope validation
* Proxy endpoint for MCPOne
* GET / POST upstream support
* Request ID propagation
* Structured proxy logs
* Upstream status code propagation
* MCPOne route mapping
---

## Role inside Evilink

## evi-gateway is part of the IO-oriented evolution of Evilink.

## Its purpose is to provide a safe and observable gateway layer before requests reach MCPOne or other internal services.

Nexus conversa.
evi-gateway valida y enruta.
MCPOne interpreta y orquesta.
Status-Hub observa.
SSC consume señales operativas.
--- 

## Main responsibilities

evi-gateway is responsible for:

* validating internal API keys
* validating route-level scopes
* rejecting unknown or unauthorized routes
* forwarding requests only to registered upstreams
* generating or propagating request IDs
* standardizing gateway errors
* exposing operational health
* producing structured logs for future observability
* preparing gateway events for Status-Hub, IO Module and SSC

## evi-gateway is not responsible for:

* deciding orchestration strategy
* replacing MCPOne
* executing business logic
* storing secrets permanently
* acting as a full enterprise API Gateway
* managing billing or tenant lifecycle
--- 

## Architecture

### Client / Nexus
     ↓
POST /api/proxy
     ↓
API Key validation
     ↓
Scope validation
     ↓
Route registry lookup
     ↓
Controlled upstream request
     ↓
MCPOne / Internal service
     ↓
Wrapped gateway response


## Endpoints

## GET /api/health

## Returns the operational status of evi-gateway and its critical upstreams.

## Example response when MCPOne is available:
``` json 
{
  "ok": true,
  "status": "operational",
  "service": "evi-gate",
  "version": "0.1.0",
  "env": "local",
  "routes_registered": 9,
  "upstreams": [
    {
      "service_name": "mcpone",
      "status": "operational",
      "target": "http://localhost:8000/health",
      "latency_ms": 684
    }
  ]
}
``` 
## Example response when MCPOne is unavailable:
``` json
{
  "ok": false,
  "status": "degraded",
  "service": "evi-gate",
  "version": "0.1.0",
  "env": "local",
  "routes_registered": 9,
  "upstreams": [
    {
      "service_name": "mcpone",
      "status": "degraded",
      "target": "http://localhost:8000/health",
      "latency_ms": null
    }
  ]
}
```
## This format is designed to be consumable by Status-Hub.


## GET /api/routes

## Returns the registered route catalog.

## Example response:
``` json
{
  "routes": [
    {
      "service_name": "mcpone",
      "route": "mcpone.health",
      "method": "GET",
      "target_url": "http://localhost:8000/health",
      "required_scopes": ["mcpone.health"],
      "auth_required": true,
      "timeout_ms": 5000
    },
    {
      "service_name": "mcpone",
      "route": "mcpone.execute",
      "method": "POST",
      "target_url": "http://localhost:8000/orchestrate",
      "required_scopes": ["mcpone.execute"],
      "auth_required": true,
      "timeout_ms": 5000
    }
  ]
}
```

## POST /api/proxy

## Validates and forwards a request to a registered upstream route.

## Headers:

Content-Type: application/json
X-API-Key: <internal-api-key>
X-Request-ID: <optional-request-id>

Request body:
``` json
{
  "route": "mcpone.execute",
  "payload": {
    "user_input": "Help me validate a CURP in Mexico",
    "client_context": {
      "source": "nexus",
      "channel": "widget"
    }
  }
}
```
## If payload.request_id is missing, evi-gateway injects the current gateway request ID.

Example response:
``` json 
{
  "request_id": "test-route-execute-001",
  "route": "mcpone.execute",
  "status": 200,
  "data": {
    "recommended_module": "curpify",
    "product_name": "Curpify",
    "status": "resolved",
    "summary": "MCP-One resolvió la solicitud con Curpify usando la capability CURP Validation."
  }
}
```


## Registered MCPOne routes

## Current normalized route registry:

mcpone.health                   GET   /health
mcpone.providers.active         GET   /providers/active
mcpone.registry.modules         GET   /registry/modules
mcpone.meta.reason_codes        GET   /meta/reason-codes
mcpone.meta.recent_resolutions  GET   /meta/recent-resolutions
mcpone.meta.metrics             GET   /meta/metrics
mcpone.metrics.summary          GET   /metrics/summary
mcpone.execute                  POST  /orchestrate
---

## Scopes

## Current recommended internal scopes:

mcpone.health
mcpone.providers.read
mcpone.registry.read
mcpone.meta.read
mcpone.metrics.read
mcpone.execute

Example:

EVIGATE_API_KEYS=nexus:nexus_dev_key:mcpone.health,mcpone.providers.read,mcpone.registry.read,mcpone.meta.read,mcpone.metrics.read,mcpone.execute

Format:

client_id:api_key:scope1,scope2,scope3

Multiple clients can be separated with semicolons:

EVIGATE_API_KEYS=nexus:nexus_dev_key:mcpone.execute;mcpone_bot:bot_key:mcpone.health,mcpone.registry.read
---

## Environment variables

HOST=0.0.0.0
PORT=8080
APP_ENV=local
MCPONE_URL=http://localhost:8000
MCPONE_HEALTH_PATH=/health
MCPONE_PROVIDERS_PATH=/providers/active
MCPONE_REGISTRY_PATH=/registry/modules
MCPONE_ORCHESTRATE_PATH=/orchestrate
DEFAULT_TIMEOUT_MS=5000
EVIGATE_API_KEYS=nexus:nexus_dev_key:mcpone.health,mcpone.providers.read,mcpone.registry.read,mcpone.meta.read,mcpone.metrics.read,mcpone.execute

When running inside Docker and MCPOne is running on the host machine, use:

MCPONE_URL=http://host.docker.internal:8000
--- 

## Local development

## Run locally:

## cargo run

## Health check:

curl http://localhost:8080/api/health

## Routes:

curl http://localhost:8080/api/routes

Proxy test:
``` json
curl -X POST http://localhost:8080/api/proxy ^
  -H "Content-Type: application/json" ^
  -H "X-API-Key: nexus_dev_key" ^
  -H "X-Request-ID: test-route-execute-001" ^
  -d "{\"route\":\"mcpone.execute\",\"payload\":{\"user_input\":\"Help me validate a CURP in Mexico\",\"client_context\":{\"source\":\"nexus\",\"channel\":\"widget\"}}}"
```

## Docker

 Build:

 docker build -t evi-gate .

## Run:

docker run --rm -p 8080:8080 ^
  -e PORT=8080 ^
  -e MCPONE_URL=http://host.docker.internal:8000 ^
  -e EVIGATE_API_KEYS=nexus:nexus_dev_key:mcpone.health,mcpone.providers.read,mcpone.registry.read,mcpone.meta.read,mcpone.metrics.read,mcpone.execute ^
  evi-gate
---

## Swagger / OpenAPI

OpenAPI JSON:

/api-docs/openapi.json

Swagger UI:

/swagger-ui/
--- 

### Error model

### Gateway-level errors use a standardized response shape:
``` json
{
  "status": 401,
  "code": "UNAUTHORIZED",
  "message": "Unauthorized: Missing X-API-Key header",
  "request_id": "test-id"
}
```
### Common error codes:

UNAUTHORIZED
FORBIDDEN
ROUTE_NOT_FOUND
UPSTREAM_ERROR
UPSTREAM_TIMEOUT
BAD_REQUEST
INTERNAL_ERROR
--- 

### Upstream status propagation

### When an upstream responds, evi-gateway propagates the upstream HTTP status code.

### Example:

MCPOne returns 200 → evi-gateway returns HTTP 200
MCPOne returns 404 → evi-gateway returns HTTP 404

The response body still keeps the gateway wrapper:
``` json
{
  "request_id": "test-status-404-001",
  "route": "mcpone.meta.reason_codes",
  "status": 404,
  "data": {
    "error": {
      "code": "NOT_FOUND",
      "message": "The requested resource was not found."
    }
  }
}
```

## Structured proxy logs

### evi-gateway emits structured logs for proxy operations.

### Event types:

proxy_rejected
proxy_completed
proxy_failed

### Example success log:

proxy_completed request_id=test-logs-success-001 upstream_request_id=test-logs-success-001 client_id=nexus route=mcpone.execute method=POST service=mcpone target_url=http://localhost:8000/orchestrate upstream_status=200 latency_ms=617 result=success
---
### Example rejected log:

proxy_rejected request_id=test-logs-invalid-key-001 client_id=unknown route=mcpone.execute reason=invalid_api_key result=error
---
### Example upstream failure log:

proxy_failed request_id=test-logs-upstream-error-001 upstream_request_id=test-logs-upstream-error-001 client_id=nexus route=mcpone.execute method=POST service=mcpone target_url=http://localhost:8000/orchestrate latency_ms=2326 result=upstream_error
--- 
## These logs are designed to become future signals for Status-Hub, IO Module and SSC.

### Current validated checks

[done] Rust base compiles locally
[done] Docker build works
[done] Swagger / OpenAPI works
[done] /api/health works
[done] Status-Hub compatible health response
[done] Route registry
[done] API Key validation
[done] Scope validation
[done] Controlled proxy to MCPOne
[done] Request ID propagation
[done] Request ID injection into MCPOne payload
[done] GET / POST route support
[done] Structured proxy logs
[done] Normalized MCPOne route names
[done] Upstream HTTP status propagation
--- 

## Roadmap

### Near-term:

[ ] Add route groups / route metadata
[ ] Add JSON structured logs
[ ] Add Status-Hub integration
[ ] Add optional V-Secrets-backed API key loading
[ ] Add Secure_Link risk checks
[ ] Add upstream health by route group
[ ] Add integration tests

### Future:

[ ] SSC event stream integration
[ ] Policy-based routing
[ ] Provider-aware routing
[ ] Failover-aware routing
[ ] Audit event persistence
--- 

### Design principle

evi-gateway should remain small, strict and observable.

It should not become the brain of the ecosystem.

Nexus interacts.
evi-gateway validates and routes.
MCPOne orchestrates.
Status-Hub observes.
SSC learns from operational signals.
---