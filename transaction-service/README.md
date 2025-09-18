# Transaction Service

Run and test locally or with Docker.

Prerequisites
- Rust toolchain (for local builds)
- Docker & Docker Compose (optional)

Run locally
1. Start Postgres (or use Docker compose DB only):

```bash
docker compose up -d db
```

1. Apply migrations:

```bash
PGPASSWORD=postgres psql -h localhost -U postgres -d transaction_service -f migrations/20250915204803_create_accounts.sql
PGPASSWORD=postgres psql -h localhost -U postgres -d transaction_service -f migrations/20250915204818_create_transactions.sql
PGPASSWORD=postgres psql -h localhost -U postgres -d transaction_service -f migrations/20250915204839_create_api_keys.sql
PGPASSWORD=postgres psql -h localhost -U postgres -d transaction_service -f migrations/20250915204901_create_webhooks.sql
```

3. Run the app:

```bash
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/transaction_service
cargo run
```

Run with Docker (recommended flow)
1. Build release locally:

```bash
cargo build --release
```

2. Build & start services using the runtime image

```bash
docker compose up --build
```

Quick access to the API documentation (when the app is running):

- Open the interactive docs at: `http://localhost:3000/docs`

Troubleshooting: port conflicts
-------------------------------
If `docker compose up` fails because host port `3000` is already in use, you have two options:

- Stop the local process using the port (example on systemd Linux):

```bash
sudo ss -ltnp | grep ':3000'
sudo systemctl stop <service-name>   # or kill <pid>
docker compose up
```

- Or map the container to a different host port by editing `docker-compose.yml` (change `3000:3000` to `3001:3000`) and restart:

```bash
# edit docker-compose.yml
docker compose up -d --build
curl http://localhost:3001/health
```


Example requests
- Create account (public):

```bash
curl -s -X POST http://localhost:3000/api/accounts \
  -H "Content-Type: application/json" \
  -d '{"business_name":"Acme Ltd","initial_balance":1000.00}' | jq
```

- Create API key (public):

```bash
curl -s -X POST http://localhost:3000/api/api-keys \
  -H "Content-Type: application/json" \
  -d '{"account_id":"<ACCOUNT_UUID>"}' | jq
```

- Call protected endpoint:

```bash
curl -s http://localhost:3000/api/accounts -H "x-api-key: <YOUR_API_KEY>" | jq
```

Notes
- Webhook payloads are signed with HMAC-SHA256 and include an `X-Signature: sha256=<hex>` header.
- API responses use a consistent error shape: `{ error, message }`.

Docs
----
- Design specification: `DESIGN.md` (architecture, schema, webhook design, trade-offs)
- API documentation: `API.md` (endpoint request/response examples and error codes)
