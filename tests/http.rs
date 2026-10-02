//! End-to-end checks of the `decide` command against a mock model server,
//! observing the callbacks the SQF side would receive.

use arma_rs::{IntoArma, testing::Result as Handled};
use std::time::Duration;
use system1::init;

const SNAPSHOT: &str = r#"[["SAD",420,45],8,[[0,false],[0.3,false]],0.45,[["inf",150,10]]]"#;
const WAIT: Duration = Duration::from_secs(15);

const OK_BODY: &str = r#"{"id":"x","model":"convaiinnovations/laya-multilingual","answers":{"choice":{"choice":"flank","probabilities":{"continue":0.2,"retreat":0.3,"flank":0.5},"confidence":0.14}},"usage":{"input_tokens":40,"output_tokens":0}}"#;

/// Runs `decide` and returns the first callback as `(function, data)`.
fn decide(url: &str, group_id: u32) -> (String, String) {
    let extension = init().testing();
    let args = vec![
        url.to_owned().to_arma().to_string(),
        group_id.to_arma().to_string(),
        SNAPSHOT.to_owned(),
        8u32.to_arma().to_string(),
    ];
    let (output, code) = extension.call("decide", Some(args));
    assert_eq!(
        (output.as_str(), code),
        ("OK", 0),
        "decide must return at once"
    );

    let handled: Handled<(String, String), ()> = extension.callback_handler(
        |name, func, data| {
            assert_eq!(name, "system1");
            let data = data.map(|d| d.to_string()).unwrap_or_default();
            Handled::Ok((func.to_owned(), data))
        },
        WAIT,
    );
    match handled {
        Handled::Ok(v) => v,
        other => panic!("no callback received: {other:?}"),
    }
}

#[test]
fn ok_response_becomes_decision_callback() {
    let mut server = mockito::Server::new();
    let mock = server
        .mock("POST", "/api/alpha/decisions")
        .match_body(mockito::Matcher::PartialJson(serde_json::json!({
            "state": "TASK SAD 400m northeast. GRP 2/8 alive, 1 wnd, hp 85%. CAS 6. AMMO 45%. CONTACTS inf 150m north."
        })))
        .with_body(OK_BODY)
        .create();

    let (func, data) = decide(&server.url(), 7);
    mock.assert();
    assert_eq!(func, "decision");
    assert_eq!(data, r#"[7,"flank",[0.2,0.3,0.5],0.14]"#);
}

#[test]
fn http_400_becomes_error_callback_with_detail() {
    let mut server = mockito::Server::new();
    server
        .mock("POST", "/api/alpha/decisions")
        .with_status(400)
        .with_body(r#"{"error":{"code":400,"message":"question 'decision' needs 120015 tokens, exceeding the model limit of 4096"}}"#)
        .create();

    let (func, data) = decide(&server.url(), 3);
    assert_eq!(func, "error");
    assert!(data.starts_with("[3,400,"), "{data}");
    assert!(data.contains("needs 120015 tokens"), "{data}");
}

#[test]
fn http_500_becomes_error_callback() {
    let mut server = mockito::Server::new();
    server
        .mock("POST", "/api/alpha/decisions")
        .with_status(500)
        .with_body("boom")
        .create();

    let (func, data) = decide(&server.url(), 1);
    assert_eq!(func, "error");
    assert!(data.starts_with("[1,500,"), "{data}");
    assert!(data.contains("boom"), "{data}");
}

#[test]
fn invalid_choice_becomes_status_zero_error() {
    let mut server = mockito::Server::new();
    server
        .mock("POST", "/api/alpha/decisions")
        .with_body(OK_BODY.replace("\"flank\",", "\"charge\","))
        .create();

    let (func, data) = decide(&server.url(), 2);
    assert_eq!(func, "error");
    assert!(data.starts_with("[2,0,"), "{data}");
}

#[test]
fn connection_refused_becomes_status_zero_error() {
    // Bind then drop to get a port that is known to be closed.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let (func, data) = decide(&format!("http://127.0.0.1:{port}"), 9);
    assert_eq!(func, "error");
    assert!(data.starts_with("[9,0,"), "{data}");
}

/// Needs the local model service on :8000; run with `cargo test -- --ignored`.
mod live {
    use super::*;

    const LIVE_URL: &str = "http://localhost:8000";

    #[test]
    #[ignore = "requires the local model service"]
    fn normal_context_yields_a_decision() {
        let (func, data) = decide(LIVE_URL, 1);
        assert_eq!(func, "decision", "{data}");
    }

    #[test]
    #[ignore = "requires the local model service"]
    fn oversized_context_yields_400() {
        let contacts = vec![r#"["inf",150,10]"#; 20_000].join(",");
        let snapshot = format!(r#"[["SAD",420,45],8,[[0,false]],0.45,[{contacts}]]"#);
        let extension = init().testing();
        let args = vec![
            LIVE_URL.to_owned().to_arma().to_string(),
            1u32.to_arma().to_string(),
            snapshot,
            u32::MAX.to_arma().to_string(),
        ];
        let _ = extension.call("decide", Some(args));
        let handled: Handled<(String, String), ()> = extension.callback_handler(
            |_, func, data| {
                Handled::Ok((
                    func.to_owned(),
                    data.map(|d| d.to_string()).unwrap_or_default(),
                ))
            },
            Duration::from_secs(60),
        );
        let Handled::Ok((func, data)) = handled else {
            panic!("no callback")
        };
        assert_eq!(func, "error");
        assert!(data.starts_with("[1,400,"), "{data}");
    }
}
