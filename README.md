# System1

Proof-of-concept Arma 3 addon that lets an AI group ask a local decision model what to do next
(`continue`, `retreat` or `flank`) and acts on the answer with waypoints.

Linux dedicated server only (`system1_x64.so`). Requires [CBA_A3](https://github.com/CBATeam/CBA_A3)
and a System One compatible model service such as [Laya](https://github.com/DarkWanderer/laya) (`POST {endpoint}/api/alpha/decisions`, used via the `jev` crate; see [docs/context-format.md](docs/context-format.md)).

## Usage

1. Load `@system1` and CBA on the dedicated server, and copy `system1_x64.so` next to the server executable
   (hemtt packs it into the release zip).
2. On an AI group that is local to the server, near enemies:

   ```sqf
   [group this] spawn System1_fnc_makeDecisions;
   ```

   The loop runs until every unit of the group is dead. Each tick it skips if the group knows no enemies
   or its previous request is still in flight.
3. Watch the server RPT: every decision is logged with its probabilities and confidence, and the group
   gets a `system1_maneuver` waypoint for `retreat` / `flank`.

Endpoint errors (including 4xx) are shown as script errors and written to the RPT; the loop retries on
the next tick. To provoke a 400, set `system1_maxContacts` very high in a large fight.

### CBA settings (server)

| Setting | Default | |
| --- | --- | --- |
| `system1_endpoint` | `http://localhost:8000` | Base URL of the model service |
| `system1_interval` | 30 | Seconds between decisions per group |
| `system1_maxContacts` | 8 | Contacts listed in the context (the model rejects contexts over 4096 tokens) |

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D clippy::unwrap_used -D clippy::expect_used
cargo test
cargo test -- --ignored      # live tests; needs the model service on localhost:8000
cargo llvm-cov               # coverage
cargo build --release && cp target/release/libsystem1.so system1_x64.so
hemtt check && hemtt build   # hemtt needs at least one git commit
```

See [AGENTS.md](AGENTS.md) and [adr/0001-async-decision-callback.md](adr/0001-async-decision-callback.md).
