use sqlx::PgPool;
use sqlx::Row;
use std::env;
use sha2::{Digest, Sha256};
use argon2::{Argon2, password_hash::SaltString, PasswordHasher};
use rand::rngs::OsRng;
use anyhow::Context;

fn compute_fingerprint(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let digest = hasher.finalize();
    let hex = hex::encode(digest);
    hex.chars().take(16).collect()
}

fn hash_key(key: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(key.as_bytes(), &salt)?;
    Ok(password_hash.to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let pool = PgPool::connect(&database_url).await.context("failed to connect to DB")?;

    // Check if old 'key' column exists
    let has_old_key: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM information_schema.columns WHERE table_name='api_keys' AND column_name='key')")
        .fetch_one(&pool)
        .await?;

    if !has_old_key.0 {
        println!("No legacy 'key' column found; nothing to do.");
        return Ok(());
    }

    // Ensure new columns exist
    let has_fp: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM information_schema.columns WHERE table_name='api_keys' AND column_name='key_fingerprint')")
        .fetch_one(&pool)
        .await?;

    if !has_fp.0 {
        println!("Please apply migration 20250920120000_add_api_key_columns.sql before running this converter.");
        return Ok(());
    }

    let rows = sqlx::query("SELECT id, key FROM api_keys WHERE key IS NOT NULL").fetch_all(&pool).await?;

    for row in rows.iter() {
        let id: uuid::Uuid = row.get("id");
        let raw_opt: Option<String> = row.get("key");
        if let Some(raw) = raw_opt {
            let fp = compute_fingerprint(&raw);
            let hashed = hash_key(&raw).map_err(|e| anyhow::anyhow!(e))?;
            // Update row
            sqlx::query("UPDATE api_keys SET key_fingerprint = $1, key_hash = $2 WHERE id = $3")
                .bind(fp)
                .bind(hashed)
                .bind(id)
                .execute(&pool)
                .await?;
        }
    }

    // Drop old column if desired
    println!("Migration complete. Consider dropping the old 'key' column after verification.");
    Ok(())
}
