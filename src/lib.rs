pub mod api;
pub mod context;

use arma_rs::{Context, ContextState, Extension, arma};
use context::GroupSnapshot;
use std::panic::{AssertUnwindSafe, catch_unwind};

const EXTENSION_NAME: &str = "system1";

#[arma]
pub fn init() -> Extension {
    Extension::build()
        .state(api::new_agent())
        .command("decide", decide)
        .command("render", render)
        .finish()
}

fn render(snapshot: GroupSnapshot, max_contacts: u32) -> String {
    context::render(&snapshot, max_contacts as usize)
}

/// Returns immediately; the outcome arrives as a `decision` or `error`
/// callback so that a slow model never stalls the game thread.
fn decide(
    ctx: Context,
    url: String,
    group_id: u32,
    snapshot: GroupSnapshot,
    max_contacts: u32,
) -> Result<&'static str, String> {
    let agent = ctx
        .global()
        .get::<ureq::Agent>()
        .ok_or_else(|| "extension state unavailable".to_owned())?
        .clone();
    let state = context::render(&snapshot, max_contacts as usize);

    std::thread::spawn(move || {
        // A panic would otherwise leave SQF's in-flight flag set forever.
        let result = catch_unwind(AssertUnwindSafe(|| api::predict(&agent, &url, &state)))
            .unwrap_or_else(|_| Err(api::ApiError::Transport("worker thread panicked".into())));
        let sent = match result {
            Ok(p) => ctx.callback_data(
                EXTENSION_NAME,
                "decision",
                (
                    group_id,
                    p.decision.as_str(),
                    p.probabilities.to_vec(),
                    p.confidence,
                ),
            ),
            Err(e) => ctx.callback_data(
                EXTENSION_NAME,
                "error",
                (group_id, u32::from(e.status()), e.detail()),
            ),
        };
        // Nothing to report to: the game is shutting down.
        let _ = sent;
    });
    Ok("OK")
}
