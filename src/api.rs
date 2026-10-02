//! Client for the decision model endpoint (`POST {base}/predict`).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(10);

const INSTRUCTIONS: &str = "Choose the infantry group's next action given its state.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Continue,
    Retreat,
    Flank,
}

impl Decision {
    /// Order of the `criteria` sent to the model; also the order of
    /// [`Prediction::probabilities`].
    pub const ALL: [Decision; 3] = [Decision::Continue, Decision::Retreat, Decision::Flank];

    pub const fn as_str(self) -> &'static str {
        match self {
            Decision::Continue => "continue",
            Decision::Retreat => "retreat",
            Decision::Flank => "flank",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Prediction {
    pub decision: Decision,
    /// In [`Decision::ALL`] order.
    pub probabilities: [f64; 3],
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    Http { status: u16, detail: String },
    Transport(String),
    Parse(String),
}

impl ApiError {
    /// Status code reported to SQF; 0 for errors without an HTTP response.
    pub fn status(&self) -> u16 {
        match self {
            ApiError::Http { status, .. } => *status,
            ApiError::Transport(_) | ApiError::Parse(_) => 0,
        }
    }

    pub fn detail(&self) -> String {
        match self {
            ApiError::Http { detail, .. } => detail.clone(),
            ApiError::Transport(m) => format!("transport error: {m}"),
            ApiError::Parse(m) => format!("invalid response: {m}"),
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Http { status, detail } => write!(f, "HTTP {status}: {detail}"),
            _ => f.write_str(&self.detail()),
        }
    }
}

impl std::error::Error for ApiError {}

#[derive(Serialize)]
struct Request<'a> {
    state: &'a str,
    questions: Questions,
}

#[derive(Serialize)]
struct Questions {
    decision: ChoiceQuestion,
}

#[derive(Serialize)]
struct ChoiceQuestion {
    #[serde(rename = "type")]
    kind: &'static str,
    instructions: &'static str,
    criteria: [&'static str; 3],
}

fn request_body(state: &str) -> Request<'_> {
    Request {
        state,
        questions: Questions {
            decision: ChoiceQuestion {
                kind: "choice",
                instructions: INSTRUCTIONS,
                criteria: Decision::ALL.map(Decision::as_str),
            },
        },
    }
}

#[derive(Deserialize)]
struct Response {
    answers: Answers,
}

#[derive(Deserialize)]
struct Answers {
    decision: Answer,
}

#[derive(Deserialize)]
struct Answer {
    choice: Decision,
    probabilities: Probabilities,
    confidence: f64,
}

#[derive(Deserialize)]
struct Probabilities {
    #[serde(rename = "continue")]
    continue_: f64,
    retreat: f64,
    flank: f64,
}

fn parse_prediction(body: &str) -> Result<Prediction, ApiError> {
    let r: Response = serde_json::from_str(body).map_err(|e| ApiError::Parse(e.to_string()))?;
    let a = r.answers.decision;
    Ok(Prediction {
        decision: a.choice,
        probabilities: [
            a.probabilities.continue_,
            a.probabilities.retreat,
            a.probabilities.flank,
        ],
        confidence: a.confidence,
    })
}

/// FastAPI reports validation failures as an array in `detail`, the model
/// service reports token-limit violations as a string.
fn extract_detail(body: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(serde_json::Value::Object(mut o)) => match o.remove("detail") {
            Some(serde_json::Value::String(s)) => s,
            Some(other) => other.to_string(),
            None => body.to_owned(),
        },
        _ => body.to_owned(),
    }
}

pub fn new_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        // Otherwise ureq turns 4xx/5xx into errors and drops the body we need.
        .http_status_as_error(false)
        .build()
        .into()
}

pub fn predict(agent: &ureq::Agent, base_url: &str, state: &str) -> Result<Prediction, ApiError> {
    let url = format!("{}/predict", base_url.trim_end_matches('/'));
    let mut response = agent
        .post(&url)
        .send_json(request_body(state))
        .map_err(|e| ApiError::Transport(e.to_string()))?;
    let status = response.status().as_u16();
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|e| ApiError::Transport(e.to_string()))?;
    if !(200..300).contains(&status) {
        return Err(ApiError::Http {
            status,
            detail: extract_detail(&body),
        });
    }
    parse_prediction(&body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_matches_schema() {
        let v = serde_json::to_value(request_body("TASK none.")).unwrap();
        assert_eq!(
            v,
            json!({
                "state": "TASK none.",
                "questions": {"decision": {
                    "type": "choice",
                    "instructions": INSTRUCTIONS,
                    "criteria": ["continue", "retreat", "flank"],
                }},
            })
        );
    }

    const OK_BODY: &str = r#"{"answers":{"decision":{"choice":"flank","probabilities":{"continue":0.2,"retreat":0.3,"flank":0.5},"confidence":0.14}},"usage":{"tokens":40}}"#;

    #[test]
    fn parses_ok_response() {
        let p = parse_prediction(OK_BODY).unwrap();
        assert_eq!(p.decision, Decision::Flank);
        assert_eq!(p.probabilities, [0.2, 0.3, 0.5]);
        assert_eq!(p.confidence, 0.14);
    }

    #[test]
    fn unknown_choice_is_parse_error() {
        let body = OK_BODY.replace("flank\",", "charge\",");
        assert!(matches!(parse_prediction(&body), Err(ApiError::Parse(_))));
    }

    #[test]
    fn malformed_body_is_parse_error() {
        assert!(matches!(parse_prediction("{}"), Err(ApiError::Parse(_))));
    }

    #[test]
    fn detail_string() {
        let body = r#"{"detail":"question 'decision' needs 120015 tokens, exceeding the model limit of 512"}"#;
        assert_eq!(
            extract_detail(body),
            "question 'decision' needs 120015 tokens, exceeding the model limit of 512"
        );
    }

    #[test]
    fn detail_array_is_compact_json() {
        let body = r#"{"detail": [ {"loc": ["body", "state"], "msg": "field required"} ]}"#;
        assert_eq!(
            extract_detail(body),
            r#"[{"loc":["body","state"],"msg":"field required"}]"#
        );
    }

    #[test]
    fn detail_non_json_is_raw_body() {
        assert_eq!(extract_detail("Bad Gateway"), "Bad Gateway");
        assert_eq!(extract_detail(r#"{"other":1}"#), r#"{"other":1}"#);
    }

    #[test]
    fn status_is_zero_without_http_response() {
        assert_eq!(ApiError::Transport("x".into()).status(), 0);
        assert_eq!(ApiError::Parse("x".into()).status(), 0);
        assert_eq!(
            ApiError::Http {
                status: 422,
                detail: String::new()
            }
            .status(),
            422
        );
    }
}
