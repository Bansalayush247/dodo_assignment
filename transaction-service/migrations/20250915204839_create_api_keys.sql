-- migrate:up
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID REFERENCES accounts(id) ON DELETE CASCADE,
    -- A short fingerprint to quickly find candidate keys (e.g. first 8 hex chars of SHA256)
    key_fingerprint TEXT NOT NULL UNIQUE,
    -- Argon2 hash of the raw key
    key_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used TIMESTAMPTZ
);

