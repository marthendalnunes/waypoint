# Architecture

Four main components:
1. **Streaming** - Real-time Snapchain events via gRPC → Redis → PostgreSQL
2. **Backfill** - Historical data via queue/worker pattern
3. **MCP** - AI assistant data access
4. **REST API** - Read-only HTTP resource access

## External API Surfaces

| Surface | Protocol | Default Port | Base Path | Primary Clients |
|---|---|---|---|---|
| MCP | Streamable HTTP (MCP) | `8000` | `/mcp` | AI assistants and MCP clients |
| REST API | HTTP/JSON | `8081` | `/api/v1` | Services, dashboards, scripts |
| Health | HTTP | `8080` (`PORT`) | `/health` | Orchestrators and uptime probes |

## Service Modes

Waypoint supports three service modes for horizontal scaling:

```bash
waypoint start              # Both producer and consumer (default)
waypoint start producer     # Producer only: Hub → Redis
waypoint start consumer     # Consumer only: Redis → PostgreSQL
```

This enables independent scaling of producers and consumers via HPA or similar.

MCP and REST services are started only in `consumer` or `both` mode because they require database access.

## Streaming

```mermaid
sequenceDiagram
    participant Hub as Snapchain
    participant Sub as Subscriber
    participant Redis
    participant Consumer
    participant DB as PostgreSQL

    Hub->>Sub: gRPC stream
    Sub->>Redis: Publish by type (casts, reactions, etc.)
    Consumer->>Redis: XREADGROUP
    Consumer->>DB: Store
    Consumer->>Redis: XACK
```

**Flow:**
- Subscriber connects to Snapchain gRPC, filters spam, groups by type
- Redis streams provide durability and backpressure
- Consumer groups enable parallel processing
- Stale messages get reclaimed via XCLAIM

## Backfill

```mermaid
sequenceDiagram
    participant Queue as Redis Queue
    participant Worker
    participant Hub as Snapchain
    participant DB as PostgreSQL

    Queue->>Worker: Job with FIDs
    Worker->>Hub: Fetch messages per FID
    Worker->>DB: Store
    Worker->>Queue: Complete
```

**Flow:**
- Queue service populates Redis with FID batches
- Workers pull jobs atomically (BRPOP)
- Each job reconciles all message types for its FIDs
- Multiple workers scale horizontally

## MCP

```mermaid
sequenceDiagram
    participant AI
    participant MCP
    participant Hub as Snapchain
    participant DB as PostgreSQL

    AI->>MCP: Tool call
    MCP->>Hub: Fetch (primary)
    MCP->>DB: Fallback
    MCP->>AI: JSON response
```

See [mcp.md](mcp.md) for tool details.

## REST API

```mermaid
sequenceDiagram
    participant Client
    participant REST
    participant Hub as Snapchain
    participant DB as PostgreSQL

    Client->>REST: GET /api/v1/...
    REST->>Hub: Fetch (primary)
    REST->>DB: Fallback
    REST->>Client: JSON response
```

See [rest.md](rest.md) for endpoint details.

## Data Access

Uses DataContext pattern with Hub-primary, DB-fallback strategy:

```
DataContext<DB, HC>
  ├── database: Option<DB>    # PostgreSQL
  └── hub_client: Option<HC>  # Snapchain gRPC
```

See [data-architecture.md](data-architecture.md) for schema.
