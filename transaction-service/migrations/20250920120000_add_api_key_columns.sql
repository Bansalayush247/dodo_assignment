-- Add columns if missing
ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS key_fingerprint TEXT;
ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS key_hash TEXT;

-- Create an index for faster fingerprint lookup (non-concurrent to allow running inside migration tooling)
CREATE UNIQUE INDEX IF NOT EXISTS idx_api_keys_key_fingerprint ON api_keys(key_fingerprint);

-- migrate:down
-- noop: rolling back this operation is manual because it may contain migrated data
