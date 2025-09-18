use std::env;

// This integration test is ignored by default. To run it, set SERVER_URL and DATABASE_URL and run:
// cargo test -- --ignored

#[tokio::test]
#[ignore]
async fn smoke_create_key_and_use_it() {
    let server = env::var("SERVER_URL").expect("SERVER_URL must be set for integration test");
    let client = reqwest::Client::new();

    // Create account
    let resp = client.post(format!("{}/api/accounts", server))
        .json(&serde_json::json!({"business_name":"TestCo","initial_balance":10.0}))
        .send()
        .await
        .expect("request failed");
    assert!(resp.status().is_success());
    let acct: serde_json::Value = resp.json().await.expect("invalid json");
    let account_id = acct.get("id").unwrap().as_str().unwrap();

    // Create API key
    let resp = client.post(format!("{}/api/api-keys", server))
        .header("x-api-key", "") // put a management key if your server requires
        .json(&serde_json::json!({"account_id": account_id}))
        .send()
        .await
        .expect("request failed");
    assert!(resp.status().is_success());
    let key_obj: serde_json::Value = resp.json().await.expect("invalid json");
    let api_key = key_obj.get("key").expect("no key returned").as_str().unwrap();

    // Use API key to hit protected endpoint
    let resp = client.get(format!("{}/api/transactions", server))
        .header("x-api-key", api_key)
        .send()
        .await
        .expect("request failed");
    assert!(resp.status().is_success());
}
