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
    let email_rule = rules["rules"]
        .as_array()
        .expect("a rule list")
        .iter()
        .find(|rule| rule["rule_id"] == "email.basic")
        .expect("email.basic listed");
    assert_eq!(email_rule["title"], "Email address");
    assert_eq!(email_rule["compiled"], true);

    let doc: serde_json::Value = client
        .get(format!("{base}/rules/email.basic"))
        .send()
        .await
        .expect("rule doc request")
        .json()
        .await
        .expect("rule doc json");
    assert!(!doc["checks"].as_array().expect("checks").is_empty());
    assert!(!doc["not_checked"]
        .as_array()
        .expect("not_checked")
        .is_empty());

    let missing = client
        .get(format!("{base}/rules/phone.zw.mobile"))
        .send()
        .await
        .expect("missing rule request");
    assert_eq!(missing.status(), reqwest::StatusCode::NOT_FOUND);

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

    // The entity goes back exactly as it arrived — this is the whole ergonomics of the endpoint.
    let card: serde_json::Value = client
        .post(format!("{base}/explain"))
        .json(&entities[1])
        .send()
        .await
        .expect("explain request")
        .json()
        .await
        .expect("explain json");
    assert_eq!(card["rule_id"], "email.basic");
    assert_eq!(card["title"], "Email address");
    assert!(card["subtitle"]
        .as_str()
        .expect("a subtitle")
        .contains("example.org"));
    assert!(card["body"].as_str().expect("a body").contains("IANA"));

    let unknown = client
        .post(format!("{base}/explain"))
        .json(&serde_json::json!({ "rule_id": "phone.zw.mobile" }))
        .send()
        .await
        .expect("explain request for an undocumented rule");
    assert_eq!(unknown.status(), reqwest::StatusCode::NOT_FOUND);
}
