-- migrate:up
CREATE TABLE transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_account UUID REFERENCES accounts(id) ON DELETE CASCADE,
    to_account UUID REFERENCES accounts(id) ON DELETE CASCADE,
    amount NUMERIC(12,2) NOT NULL,
    txn_type VARCHAR(20) NOT NULL CHECK (txn_type IN ('credit', 'debit', 'transfer')),
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

