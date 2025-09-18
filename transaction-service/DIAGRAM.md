## 🏗️ System Architecture
The following diagram shows how client systems interact with the Transaction Service, its internal components, and the database:

```mermaid
flowchart TD
    subgraph Client["Business Client"]
        APIConsumer["Client System (uses API Key)"]
        WebhookReceiver["Webhook Endpoint (business-owned)"]
    end

    subgraph Service["Transaction Service (Rust + Axum)"]
        API["HTTP API Layer"]
        Auth["API Key Middleware"]
        TxLogic["Transaction Logic"]
        WebhookWorker["Webhook Dispatcher (Background Task)"]
    end

    subgraph DB["Postgres Database"]
        Accounts["Accounts Table"]
        Transactions["Transactions Table"]
        APIKeys["API Keys Table"]
        Webhooks["Webhooks Table"]
        Events["Webhook Events (Outbox)"]
    end

    APIConsumer -->|HTTPS + API Key| API
    API --> Auth
    Auth --> TxLogic
    TxLogic -->|Read/Write| Accounts
    TxLogic -->|Insert| Transactions
    TxLogic -->|Lookup| APIKeys
    TxLogic -->|Register| Webhooks
    TxLogic -->|Insert| Events
    WebhookWorker -->|Query pending events| Events
    WebhookWorker -->|POST + HMAC| WebhookReceiver
```

## 🗄️ Database Schema (ERD)
The following ER diagram describes the relational model for accounts, transactions, API keys, and webhooks:

```mermaid
erDiagram
    ACCOUNTS {
        uuid id PK
        uuid business_id
        decimal balance
        timestamp created_at
    }

    TRANSACTIONS {
        uuid id PK
        uuid account_from FK
        uuid account_to FK
        decimal amount
        text type "credit | debit | transfer"
        text status "pending | success | failed"
        timestamp created_at
    }

    API_KEYS {
        uuid id PK
        uuid business_id
        text key_hash
        timestamp created_at
    }

    WEBHOOKS {
        uuid id PK
        uuid business_id
        text url
        text secret
        timestamp created_at
    }

    WEBHOOK_EVENTS {
        uuid id PK
        uuid webhook_id FK
        uuid transaction_id FK
        text status "pending | delivered | failed"
        int retries
        timestamp created_at
    }

    ACCOUNTS ||--o{ TRANSACTIONS : "account_from/account_to"
    ACCOUNTS ||--o{ API_KEYS : "owns"
    ACCOUNTS ||--o{ WEBHOOKS : "registers"
    TRANSACTIONS ||--o{ WEBHOOK_EVENTS : "triggers"
    WEBHOOKS ||--o{ WEBHOOK_EVENTS : "receives"
```
