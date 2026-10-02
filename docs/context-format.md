# Context format

SQF (`System1_fnc_collectContext`) sends the extension a positional array; `src/context.rs` renders it to
the text sent as `state` in `POST /api/alpha/decisions`.

```
[[wpType, wpDist, wpDir], initialSize, [[damage, incapacitated], ...aliveUnits], ammoFraction, [[category, dist, dir], ...contacts]]
```

- `wpType` is empty when the group has no current waypoint (rendered `TASK none`).
- `ammoFraction` is current rounds divided by the rounds recorded when the loop started.
- `category` is one of `inf`, `armor`, `air`, `static`, `veh`, `other`.

Rendered example:

```
TASK SAD 400m northeast. GRP 6/8 alive, 1 wnd, hp 85%. CAS 2. AMMO 45%. CONTACTS inf 150m north, inf 200m north, armor 400m northwest (+3 more).
```

- Distances are rounded to 50 m; bearings are absolute (world) directions snapped to 8 compass points (`north` … `northwest`), measured from the leader.
- Contacts are sorted by distance, capped at `system1_maxContacts`, the rest is counted as `(+K more)`.
- `CAS` counts dead plus incapacitated units.

## Endpoint

The extension talks to a System One compatible service through the [`jev`](https://crates.io/crates/jev)
crate, e.g. [Laya](https://github.com/DarkWanderer/laya). It posts to `{system1_endpoint}/api/alpha/decisions`
with model `convaiinnovations/laya-multilingual` and one `choice` question whose options are
`continue`, `retreat` and `flank`; the answer's `choice`, `probabilities` and `confidence` are returned to SQF.

Errors: `400` with `{"error": {"code": 400, "message": "..."}}` for oversized contexts (4096-token limit);
FastAPI validation errors give `detail`. Both are surfaced as script errors.
