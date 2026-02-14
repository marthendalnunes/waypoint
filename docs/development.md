# Development

## Prerequisites

- Rust 1.92+
- Docker & Docker Compose
- PostgreSQL 17+ with pgvector (or use Docker)

## Setup

```bash
docker compose up -d          # start postgres + redis
make env-setup                # create .env
make build
make run
```

## Verify Local APIs

After startup, verify health and REST endpoints:

```bash
curl "http://localhost:8080/health"
curl "http://localhost:8081/api/v1/openapi.json"
curl "http://localhost:8081/api/v1/users/3"
```

If REST is not enabled, set `WAYPOINT_REST__ENABLED=true` in `.env` and restart Waypoint.

## Migrations

Migrations run automatically on startup. To add new ones:

```bash
# Create migration
touch migrations/NNN_description.sql

# Update SQLx cache
export DATABASE_URL=postgresql://postgres:postgres@localhost:5432/waypoint
cargo sqlx prepare
```

## Backfill

```bash
# Local
make backfill-queue
make backfill-worker

# Docker (scales to 4 workers)
docker compose --profile backfill up --scale backfill-worker=4
```

Worker flags:
- `--exit-on-complete` - exit when done
- `--idle-timeout <secs>` - wait time before exit (default: 30)

## Metrics

```bash
make metrics-start
./run-with-metrics.sh make backfill-worker
make metrics-open              # opens Grafana
make metrics-stop
```

## Testing

```bash
make test
cargo test test_name
```

## Formatting

```bash
make fmt
```
