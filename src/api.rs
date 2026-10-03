//! Client for the decision model endpoint, a System One compatible service
//! (`POST {base}/api/alpha/decisions`), reached through the `jev` crate.

use jev::{ChoiceAnswer, State, TypeSafeClient, TypeSafeError};
use std::fmt;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(10);

/// The only model the local Laya service accepts.
const MODEL: &str = "convaiinnovations/laya-multilingual";

const DECISIONS_PATH: &str = "/api/alpha/decisions";

const INSTRUCTIONS: &str = "Choose the infantry group's next action given its state.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Continue,
    Retreat,
    Flank,
}

impl Decision {
    /// Order of [`Prediction::probabilities`].
    pub const ALL: [Decision; 3] = [Decision::Continue, Decision::Retreat, Decision::Flank];

    pub const fn as_str(self) -> &'static str {
        match self {
            Decision::Continue => "continue",
            Decision::Retreat => "retreat",
            Decision::Flank => "flank",
        }
    }

    /// Shown to the model next to the option name.
    const fn description(self) -> &'static str {
        match self {
            Decision::Continue => "keep the current task",
            Decision::Retreat => "withdraw from the contacts",
            Decision::Flank => "maneuver around the contacts",
        }
    }

    fn parse(name: &str) -> Option<Decision> {
        Decision::ALL.into_iter().find(|d| d.as_str() == name)
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

impl From<TypeSafeError> for ApiError {
    fn from(e: TypeSafeError) -> Self {
        match e {
            TypeSafeError::Api { status, message } => ApiError::Http {
                status,
                detail: extract_detail(&message),
            },
            TypeSafeError::InvalidResponse(m) => ApiError::Parse(m),
            e @ TypeSafeError::UnexpectedAnswer { .. } => ApiError::Parse(e.to_string()),
            // Key errors cannot occur: the client is built keyless.
            e => ApiError::Transport(e.to_string()),
        }
    }
}

fn parse_prediction(answer: &ChoiceAnswer) -> Result<Prediction, ApiError> {
    let decision = Decision::parse(&answer.choice)
        .ok_or_else(|| ApiError::Parse(format!("unknown choice {:?}", answer.choice)))?;
    let mut probabilities = [0.0; 3];
    for (slot, d) in probabilities.iter_mut().zip(Decision::ALL) {
        *slot = *answer
            .probabilities
            .get(d.as_str())
            .ok_or_else(|| ApiError::Parse(format!("no probability for {:?}", d.as_str())))?;
    }
    Ok(Prediction {
        decision,
        probabilities,
        confidence: answer.confidence,
    })
}

/// jev reports the response body (compact JSON, or a JSON string for a
/// non-JSON body) when it finds no top-level `message`. The service nests
/// its message as `{"error": {"message": ...}}`; FastAPI validation failures
/// use `detail`, which may be an array.
fn extract_detail(body: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(serde_json::Value::String(s)) => s,
        Ok(serde_json::Value::Object(mut o)) => {
            if let Some(serde_json::Value::String(m)) = o
                .get_mut("error")
                .and_then(|e| e.as_object_mut())
                .and_then(|e| e.remove("message"))
            {
                return m;
            }
            match o.remove("detail") {
                Some(serde_json::Value::String(s)) => s,
                Some(other) => other.to_string(),
                None => body.to_owned(),
            }
        }
        _ => body.to_owned(),
    }
}

/// Shared by every `decide` call so connections are pooled; the endpoint is
/// set per call because SQF supplies it.
pub fn new_client() -> Result<TypeSafeClient, ApiError> {
    Ok(TypeSafeClient::local(DECISIONS_PATH, MODEL)?.with_timeout(TIMEOUT))
}

fn endpoint(base_url: &str) -> String {
    format!("{}{DECISIONS_PATH}", base_url.trim_end_matches('/'))
}

pub fn predict(
    client: &TypeSafeClient,
    base_url: &str,
    state: &str,
) -> Result<Prediction, ApiError> {
    let criteria = Decision::ALL.map(|d| (d.as_str(), d.description()));
    let answer = client.clone().with_endpoint(endpoint(base_url)).choose(
        &State::text(state),
        INSTRUCTIONS,
        &criteria,
    )?;
    parse_prediction(&answer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn answer(choice: &str, probs: &[(&str, f64)]) -> ChoiceAnswer {
        ChoiceAnswer {
            choice: choice.to_owned(),
            probabilities: probs.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
            confidence: 0.14,
        }
    }

    const PROBS: [(&str, f64); 3] = [("continue", 0.2), ("retreat", 0.3), ("flank", 0.5)];

    #[test]
    fn parses_choice_answer() {
        let p = parse_prediction(&answer("flank", &PROBS)).unwrap();
        assert_eq!(p.decision, Decision::Flank);
        assert_eq!(p.probabilities, [0.2, 0.3, 0.5]);
        assert_eq!(p.confidence, 0.14);
    }

    #[test]
    fn unknown_choice_is_parse_error() {
        assert!(matches!(
            parse_prediction(&answer("charge", &PROBS)),
            Err(ApiError::Parse(_))
        ));
    }

    #[test]
    fn missing_probability_is_parse_error() {
        let a = ChoiceAnswer {
            probabilities: BTreeMap::new(),
            ..answer("flank", &[])
        };
        assert!(matches!(parse_prediction(&a), Err(ApiError::Parse(_))));
    }

    #[test]
    fn endpoint_appends_path_once() {
        assert_eq!(
            endpoint("http://h:8000"),
            "http://h:8000/api/alpha/decisions"
        );
        assert_eq!(
            endpoint("http://h:8000/"),
            "http://h:8000/api/alpha/decisions"
        );
    }

    #[test]
    fn detail_nested_error_message() {
        let body = r#"{"error":{"code":400,"message":"question 'decision' needs 5000 tokens"}}"#;
        assert_eq!(
            extract_detail(body),
            "question 'decision' needs 5000 tokens"
        );
    }

    #[test]
    fn detail_string() {
        assert_eq!(extract_detail(r#"{"detail":"too long"}"#), "too long");
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
        assert_eq!(extract_detail(r#""boom""#), "boom");
        assert_eq!(extract_detail(r#"{"other":1}"#), r#"{"other":1}"#);
    }

    #[test]
    fn status_is_zero_without_http_response() {
        assert_eq!(ApiError::Transport("x".into()).status(), 0);
        assert_eq!(ApiError::Parse("x".into()).status(), 0);
        assert_eq!(
            ApiError::Http {
                status: 400,
                detail: String::new()
            }
            .status(),
            400
        );
    }
}
