//! The HTTP surface, over a real socket.
//!
//! Binding port zero and asking the OS which port it gave keeps the test from colliding with the
//! server a developer already has running.

mod support;

use regex_entity_scanner::service;

#[tokio::test]
async fn health_rules_and_scan() {
    let scanner = support::scanner();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("binding an ephemeral port");
    let address = listener.local_addr().expect("the bound address");
    tokio::spawn(async move {
        axum::serve(listener, service::router(scanner)).await.ok();
    });

    let client = reqwest::Client::new();
    let base = format!("http://{address}");

    let health: serde_json::Value = client
        .get(format!("{base}/health"))
        .send()
        .await
        .expect("health request")
        .json()
        .await
        .expect("health json");
    assert_eq!(health["status"], "ok");
    assert!(health["rules"].as_u64().expect("a rule count") > 0);

    let rules: serde_json::Value = client
        .get(format!("{base}/rules"))
        .send()
        .await
        .expect("rules request")
        .json()
        .await
        .expect("rules json");
    assert!(rules["rules"]
        .as_array()
        .expect("a rule list")
        .iter()
        .any(|rule| rule == "email.basic"));

    let scanned: serde_json::Value = client
        .post(format!("{base}/scan"))
        .json(&serde_json::json!({
            "text": "filed 2021-03-04 by ops@example.org",
            "offset": 100,
        }))
        .send()
        .await
        .expect("scan request")
        .json()
        .await
        .expect("scan json");

    let entities = scanned["entities"].as_array().expect("an entity list");
    assert_eq!(entities.len(), 2);
    assert_eq!(entities[0]["type"], "date");
    assert_eq!(entities[0]["start"], 106);
    assert_eq!(entities[0]["rule_id"], "date.iso8601");
    assert_eq!(entities[1]["type"], "email");
    assert_eq!(entities[1]["value"]["address"], "ops@example.org");
}
