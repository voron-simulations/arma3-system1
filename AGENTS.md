# AGENTS.md

Arma 3 addon: Rust extension (`src/`, arma-rs) + SQF (`addons/`, HEMTT). Layout mirrors `../arma3-dynops`.

- Function tag is `System1` (`System1_fnc_*`); CBA settings are `system1_*`.
- Context format and endpoint: [docs/context-format.md](docs/context-format.md). Keep SQF `collectContext` and `src/context.rs` in sync.
- `cargo clippy` denies `unwrap`/`expect` outside tests.
- `tests/http.rs` live tests are `#[ignore]`d; they need the model on `localhost:8000`.
- `hemtt` needs a git commit to run.
