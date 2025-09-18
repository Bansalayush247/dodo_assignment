# Transaction Service API

This document summarizes the HTTP API implemented in the `transaction-service` (Axum + SQLx + Postgres).

Auth
- All protected endpoints require the header `x-api-key: <api_key>` returned when creating an API key.

Common types
- id: UUID string
- amount: decimal string (e.g. "10.00")

Endpoints

1) Health
- GET /health
- Response: 200 OK, body: `{"status":"ok"}`

2) Accounts
- POST /api/accounts
  - Public. Create account.
  - JSON body: {"business_name": "Acme Ltd", "initial_balance": 1000.00}
  - Response: 201 Created
    {
      "id": "<uuid>",
      "business_name": "Acme Ltd",
      "balance": "1000.00",
      "created_at": "2025-09-16T...Z"
    }

- GET /api/accounts (protected)
  - Header: `x-api-key: <api_key>`
  - Response: 200 OK, JSON array of accounts

- GET /api/accounts/{id} (protected)
  - Response: 200 OK, account object

- GET /api/accounts/{id}/balance (protected)
  - Response: 200 OK, `{ "balance": "123.45" }`

3) Transactions
- POST /api/transactions (protected)
  - Header: `x-api-key: <api_key>`
  - JSON body examples:
    - Credit account:
      {"txn_type":"credit","to_account":"<account_id>","amount":10.00}
    - Debit account:
      {"txn_type":"debit","from_account":"<account_id>","amount":5.00}
    - Transfer:
      {"txn_type":"transfer","from_account":"<from_id>","to_account":"<to_id>","amount":10.00}
  - Response: 201 Created
    {
      "id": "<uuid>",
      "from_account": "<uuid>|null",
      "to_account": "<uuid>|null",
      "amount": "10.00",
      "txn_type": "transfer",
      "status": "completed",
      "created_at": "..."
    }

- GET /api/transactions (protected)
  - Query params: optional filtering (not implemented in-full)
  - Response: 200 OK, array of transactions

- GET /api/transactions/{id} (protected)
  - Response: transaction object

4) API Keys
- POST /api/api-keys (protected)
  - Header: `x-api-key: <api_key>` (this endpoint is protected to allow creating keys scoped to an account)
  - JSON body: { "account_id": "<uuid>", "description": "CI key" }
  - Response: 201 Created
    {
      "id":"<uuid>",
      "account_id":"<uuid>",
      "key":"<raw_api_key>",
      "description":"...",
      "created_at":"..."
    }

Note: The service returns the raw API key at creation only. The service now stores only an Argon2 hash and a fingerprint; keep the raw token safe because it will not be shown again.

Migration note: if you previously created API keys with the older schema (raw `key` column), apply the migration `migrations/20250920120000_add_api_key_columns.sql` and then run the converter binary:

```bash
# apply migration (psql or your migration tool)
PGPASSWORD=postgres psql -h localhost -U postgres -d transaction_service -f migrations/20250920120000_add_api_key_columns.sql

# run converter (this will copy legacy `key` values into hashed storage; it requires DATABASE_URL env)
cargo run --bin migrate_api_keys
```

After verifying records, you may drop the old `key` column.

5) Webhooks
- POST /api/webhooks (protected)
  - JSON body: { "account_id": "<uuid>", "url": "https://example.com/webhook" }
  - Response: 201 Created
    { "id":"<uuid>", "account_id":"<uuid>", "url":"https://...", "secret":"<secret returned>", "created_at":"..." }

- GET /api/webhooks (protected)
  - List webhooks for the authenticated account

Webhook delivery
- When a transaction affects an account with registered webhooks, the service enqueues a `webhook_event` and attempts delivery in background.
- The payload is JSON and looks like:
  {
    "event_type": "transaction.created",
    "transaction": { /* transaction object */ }
  }
- Signature header: `X-Signature: sha256=<hex>` where `<hex>` is the HMAC-SHA256 of the raw JSON payload using the webhook `secret`.
- Retries: the service retries delivery up to a few times with exponential backoff and records attempts in `webhook_events`.

Errors
- 400 Bad Request — malformed input
- 401 Unauthorized — missing or invalid `x-api-key`
- 404 Not Found — missing resource
- 409 Conflict / 422 Unprocessable — business rule failures (e.g. insufficient funds)

Examples
- Create account:
  curl -X POST http://localhost:3000/api/accounts -H "Content-Type: application/json" -d '{"business_name":"Beta Co","initial_balance":500.00}'

- Create API key (use an existing management key):
  curl -X POST http://localhost:3000/api/api-keys -H "Content-Type: application/json" -H "x-api-key: <manage_key>" -d '{"account_id":"<uuid>","description":"dev key"}'

Notes & Known limitations
- Full OpenAPI spec is available in `openapi.yaml` for automated clients and codegen.
- API keys are stored securely: the service returns a one-time raw token at creation and stores only an Argon2 hash + fingerprint.
