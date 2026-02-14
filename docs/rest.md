# REST API (Resource Parity)

Waypoint exposes a read-only REST API with resource parity to MCP `waypoint://` resources.

## What This Provides

- Resource parity for core MCP resources over standard HTTP GET endpoints.
- JSON responses that match MCP resource payload structure where applicable.
- HTTP-native status semantics (`400`, `404`, `500`, and `200` with empty collections).
- Auto-generated OpenAPI spec and interactive Swagger UI.

## Configuration

```toml
[rest]
enabled = true
bind_address = "0.0.0.0"
port = 8081
max_limit = 100
swagger_ui_enabled = false
```

Environment variables:

```bash
WAYPOINT_REST__ENABLED=true
WAYPOINT_REST__BIND_ADDRESS=0.0.0.0
WAYPOINT_REST__PORT=8081
WAYPOINT_REST__MAX_LIMIT=100
WAYPOINT_REST__SWAGGER_UI_ENABLED=false
```

## Endpoints

Base URL examples in this document use `http://localhost:8081`.

### OpenAPI and Swagger

- `GET /api/v1/openapi.json` - Generated OpenAPI specification.
- `GET /swagger-ui` - Interactive Swagger UI backed by the generated spec (when `swagger_ui_enabled=true`).

### Users
- `GET /api/v1/users/{fid}`
- `GET /api/v1/users/by-username/{username}`

### Verifications
- `GET /api/v1/verifications/{fid}`
- `GET /api/v1/verifications/{fid}/{address}`
- `GET /api/v1/verifications/all-by-fid/{fid}?limit=10&start_time=<unix>&end_time=<unix>`

### Casts
- `GET /api/v1/casts/{fid}/{hash}`
- `GET /api/v1/casts/by-fid/{fid}?limit=10`
- `GET /api/v1/casts/by-mention/{fid}?limit=10`
- `GET /api/v1/casts/by-parent/{fid}/{hash}?limit=10`
- `GET /api/v1/casts/by-parent-url?url=<encoded-url>&limit=10`

### Conversations
- `GET /api/v1/conversations/{fid}/{hash}?recursive=true&max_depth=5&limit=10`

### Reactions
- `GET /api/v1/reactions/by-fid/{fid}?limit=10`
- `GET /api/v1/reactions/by-target-cast/{fid}/{hash}?limit=10`
- `GET /api/v1/reactions/by-target-url?url=<encoded-url>&limit=10`

### Links
- `GET /api/v1/links/by-fid/{fid}?limit=10`
- `GET /api/v1/links/by-target/{fid}?limit=10`
- `GET /api/v1/links/compact-state/{fid}`

### Username Proofs
- `GET /api/v1/username-proofs/{fid}`
- `GET /api/v1/username-proofs/by-name/{name}`

## MCP Resource Mapping

| MCP Resource URI | REST Endpoint |
|---|---|
| `waypoint://users/{fid}` | `GET /api/v1/users/{fid}` |
| `waypoint://users/by-username/{username}` | `GET /api/v1/users/by-username/{username}` |
| `waypoint://verifications/{fid}` | `GET /api/v1/verifications/{fid}` |
| `waypoint://verifications/{fid}/{address}` | `GET /api/v1/verifications/{fid}/{address}` |
| `waypoint://verifications/all-by-fid/{fid}` | `GET /api/v1/verifications/all-by-fid/{fid}` |
| `waypoint://casts/{fid}/{hash}` | `GET /api/v1/casts/{fid}/{hash}` |
| `waypoint://casts/by-fid/{fid}` | `GET /api/v1/casts/by-fid/{fid}` |
| `waypoint://casts/by-mention/{fid}` | `GET /api/v1/casts/by-mention/{fid}` |
| `waypoint://casts/by-parent/{fid}/{hash}` | `GET /api/v1/casts/by-parent/{fid}/{hash}` |
| `waypoint://casts/by-parent-url?url=...` | `GET /api/v1/casts/by-parent-url?url=...` |
| `waypoint://conversations/{fid}/{hash}` | `GET /api/v1/conversations/{fid}/{hash}` |
| `waypoint://reactions/by-fid/{fid}` | `GET /api/v1/reactions/by-fid/{fid}` |
| `waypoint://reactions/by-target-cast/{fid}/{hash}` | `GET /api/v1/reactions/by-target-cast/{fid}/{hash}` |
| `waypoint://reactions/by-target-url?url=...` | `GET /api/v1/reactions/by-target-url?url=...` |
| `waypoint://links/by-fid/{fid}` | `GET /api/v1/links/by-fid/{fid}` |
| `waypoint://links/by-target/{fid}` | `GET /api/v1/links/by-target/{fid}` |
| `waypoint://links/compact-state/{fid}` | `GET /api/v1/links/compact-state/{fid}` |
| `waypoint://username-proofs/by-name/{name}` | `GET /api/v1/username-proofs/by-name/{name}` |
| `waypoint://username-proofs/{fid}` | `GET /api/v1/username-proofs/{fid}` |

## Defaults and validation
- `limit` defaults to `10` and is clamped by `rest.max_limit`.
- `limit=0` is rejected with `400`.
- URL-based endpoints require `url` query parameter.
- Hash params accept `0x`-prefixed and non-prefixed hex.
- Address params accept `0x`-prefixed and non-prefixed hex.
- `start_time` must be less than or equal to `end_time`.

## Request and Response Examples

### Get User by FID

```bash
curl "http://localhost:8081/api/v1/users/3"
```

Example response:

```json
{
  "fid": 3,
  "display_name": "Dan Romero",
  "username": "dwr",
  "bio": "Building Farcaster"
}
```

### Get Verifications by FID

```bash
curl "http://localhost:8081/api/v1/verifications/3?limit=10"
```

Example response:

```json
{
  "fid": 3,
  "count": 1,
  "verifications": [
    {
      "fid": 3,
      "address": "0x1234...",
      "protocol": "ethereum",
      "type": "eoa",
      "action": "add",
      "timestamp": 1710000000
    }
  ]
}
```

### Get Verification by FID and Address

```bash
curl "http://localhost:8081/api/v1/verifications/3/0x1234"
```

Example response:

```json
{
  "fid": 3,
  "address": "0x1234",
  "found": true,
  "verification": {
    "fid": 3,
    "address": "0x1234",
    "protocol": "ethereum",
    "type": "eoa",
    "action": "add",
    "timestamp": 1710000000
  }
}
```

### Get All Verification Messages by FID

```bash
curl "http://localhost:8081/api/v1/verifications/all-by-fid/3?limit=25&start_time=1700000000&end_time=1710000000"
```

Example response:

```json
{
  "fid": 3,
  "count": 2,
  "start_time": 1700000000,
  "end_time": 1710000000,
  "verifications": [
    {
      "fid": 3,
      "address": "0x1234...",
      "protocol": "ethereum",
      "type": "eoa",
      "action": "add",
      "timestamp": 1710000000
    }
  ]
}
```

### Get Cast by FID and Hash

```bash
curl "http://localhost:8081/api/v1/casts/3/0xabc123"
```

### Get Cast Replies by Parent URL

```bash
curl "http://localhost:8081/api/v1/casts/by-parent-url?url=https%3A%2F%2Fexample.com%2Fpost%2F1"
```

### Get Conversation Thread

```bash
curl "http://localhost:8081/api/v1/conversations/3/0xabc123?recursive=true&max_depth=5&limit=25"
```

### Get Reactions by Target Cast

```bash
curl "http://localhost:8081/api/v1/reactions/by-target-cast/3/0xabc123?limit=10"
```

### Get Links by Target

```bash
curl "http://localhost:8081/api/v1/links/by-target/3?limit=20"
```

### Get Username Proof by Name

```bash
curl "http://localhost:8081/api/v1/username-proofs/by-name/vitalik.eth"
```

Example response:

```json
{
  "name": "vitalik.eth",
  "found": true,
  "type": "ens_l1",
  "fid": 5650,
  "timestamp": 1710000000,
  "owner": "0x1234..."
}
```

### Get Username Proofs by FID

```bash
curl "http://localhost:8081/api/v1/username-proofs/5650"
```

Example response:

```json
{
  "fid": 5650,
  "count": 1,
  "proofs": [
    {
      "name": "vitalik.eth",
      "type": "ens_l1",
      "fid": 5650,
      "timestamp": 1710000000,
      "owner": "0x1234..."
    }
  ]
}
```

## Error format

All REST errors use a consistent JSON schema:

```json
{
  "error": {
    "code": "invalid_params",
    "message": "Invalid parameters: ..."
  }
}
```

### Error Examples

`400` invalid params:

```json
{
  "error": {
    "code": "invalid_params",
    "message": "Invalid parameters: Invalid fid: not-a-number"
  }
}
```

`404` singular resource not found:

```json
{
  "error": {
    "code": "not_found",
    "message": "Resource not found: No cast found for FID 3 with hash 0xabc123"
  }
}
```

`500` internal error:

```json
{
  "error": {
    "code": "internal_error",
    "message": "Internal error: Error fetching user data: Hub client not available"
  }
}
```

`200` empty list payload:

```json
{
  "fid": 3,
  "count": 0,
  "casts": []
}
```

## HTTP semantics
- `400` invalid parameters
- `404` singular resource not found
- `200` empty list payload for list-style resources with no results
- `500` internal/upstream failures

## Examples

```bash
# User by FID
curl "http://localhost:8081/api/v1/users/3"

# Cast replies by URL
curl "http://localhost:8081/api/v1/casts/by-parent-url?url=https%3A%2F%2Fexample.com%2Fpost%2F1"

# Conversation with deeper traversal
curl "http://localhost:8081/api/v1/conversations/3/0xabc123?recursive=true&max_depth=7&limit=25"
```

## OpenAPI and Swagger Usage

```bash
# Fetch generated OpenAPI spec
curl "http://localhost:8081/api/v1/openapi.json"
```

Open Swagger UI in a browser:

```text
http://localhost:8081/swagger-ui
```

If disabled, enable it via `rest.swagger_ui_enabled=true` (or `WAYPOINT_REST__SWAGGER_UI_ENABLED=true`).

## Rollout and compatibility notes
- REST runs on a dedicated port (`8081` by default).
- MCP remains available and unchanged for tool invocations.
- REST responses mirror MCP resource payloads where applicable, with HTTP-native status handling.

## Troubleshooting

If the REST API is not reachable:

1. Verify REST is enabled:
   - `rest.enabled = true` in config, or
   - `WAYPOINT_REST__ENABLED=true` in env.
2. Verify bind address and port:
   - `rest.bind_address`, `rest.port`
3. Confirm Docker port mappings include REST port when containerized.
4. Confirm `waypoint` is running in `consumer` or `both` mode (REST requires database access).
5. Check service logs for startup errors and connection failures.
6. Test health and REST endpoints directly:
   - `GET /health`
   - `GET /api/v1/openapi.json`
   - `GET /api/v1/users/{fid}`
