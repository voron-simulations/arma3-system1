# Context format

SQF (`System1_fnc_collectContext`) sends the extension a positional array; `src/context.rs` renders it to
the text sent as `state` in `POST /predict`.

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

Request:

```json
{"state": "<text>", "questions": {"decision": {"type": "choice", "instructions": "...", "criteria": ["continue", "retreat", "flank"]}}}
```

Response `200`: `{"answers": {"decision": {"choice": "...", "probabilities": {...}, "confidence": 0.14}}, "usage": {...}}`.

Errors: `422` with `{"detail": "question 'decision' needs N tokens, exceeding the model limit of 512"}`
for oversized contexts; FastAPI validation errors give `detail` as an array. Both are surfaced as script errors.
