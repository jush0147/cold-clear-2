# Codex task: integrate Kiwi v1 into Tetrp

You are implementing the first usable Kiwi bot integration in **jush0147/tetrp**.

Read `KIWI_V1_HANDOFF.md` in this repository first. Treat it as the product contract.

## Repositories / refs

Kiwi source handoff:
- repository: `jush0147/cold-clear-2`
- branch: `kiwi-v1`
- consume the **`kiwi-v1-browser`** GitHub Actions artifact
- do not vendor the Rust repository into Tetrp

Tetrp:
- repository: `jush0147/tetrp`
- starting reference at handoff: `0b48cb7e1a50e5f0bba6fcfee05ba8e291bebee2`

## Goal

Make Kiwi a real interactive review feature inside the existing Tetrp replay viewer.

The user flow must be:

1. Open a replay normally.
2. Navigate to a placement boundary.
3. Invoke **Kiwi**.
4. Show one Kiwi continuation placement from that exact visible state.
5. Let the user advance the Kiwi line one placement at a time.
6. Keep the recorded replay untouched and allow returning to it.

Do not implement automatic full-replay grading in this pass.

## Product constraints

- Display name is **Kiwi**.
- Default search budget is 200,000 evaluator nodes per decision.
- Tetrp remains gameplay authority.
- Kiwi remains search / placement-choice only.
- Kiwi sees own board, current, Hold, exactly NEXT 5, B2B/Surge/combo and already-observable incoming garbage/timing.
- Kiwi must not see opponent board, replay future, hidden RNG or future attacks.
- Bot work must run off the main UI thread.
- Local replay privacy must remain intact. No replay content is uploaded.
- Existing PWA/offline behavior must continue to work.

## Integration approach

Do not start by redesigning Tetrp.

First establish a small, testable vertical slice:

### A. Asset integration

Copy the built Kiwi browser artifact into Tetrp's static build in a reproducible way. Do not hand-copy opaque files without recording provenance.

Keep a small manifest in Tetrp containing:
- Kiwi product version
- source commit
- artifact build commit
- default node budget

Ensure the PWA/service-worker asset list includes the Kiwi JS/WASM required for offline use.

### B. Dedicated Kiwi worker

Create a dedicated module Worker for Kiwi search.

Responsibilities:
- initialize `cold_clear_2.js` / WASM once
- own the active `WasmBot`
- receive detached visible-state requests
- return suggestions/errors/progress without mutating viewer state

Do not run 200k-node search on the main thread.

### C. Analysis-session state

Do not mutate the replay `Reconstruction`.

Create a separate hypothetical analysis session / engine fork for Kiwi continuation.

At the invocation point, reconstruct only information observable up to that point. In particular, reconstruct SevenBag observation history from the round start rather than reading hidden future queue state.

Reuse or faithfully port:
- `tetrp-authority-adapter.mjs`
- `tetrp-placement-path.mjs`

### D. First-move path

For a no-pending decision:
- initialize one persistent `WasmBot`
- call `think_nodes(200000)`
- call `suggest_json()`
- use the top suggestion
- render the suggested placement without changing the recorded replay

### E. Continuation path

When the user advances Kiwi:
- apply the selected placement through a separate Tetrp authority engine
- use explicit Hold semantics
- reveal the correct number of new previews
- keep the same `WasmBot` and refill it with `new_piece()` when the persistent path remains valid
- search the next 200k-node decision

Do not directly write cells onto the board.

### F. Incoming-garbage path

Never ignore incoming garbage.

The persistent `WasmBot.start()` path does not model pending garbage. When observable incoming exists, use `analyze_pending_json()` and a Tetrp-owned hypothetical continuation as specified in `KIWI_V1_HANDOFF.md`.

It is acceptable for pending-aware continuation to rebuild snapshot search while pending remains. Correctness is more important than preserving the DAG in a state the persistent API does not support.

## UI scope

Keep UI changes deliberately small.

Add a clear **Kiwi** action near replay navigation / analysis controls. When active:
- distinguish the Kiwi hypothetical line from recorded replay state
- show searching / error state
- show the suggested move/board
- provide one-step forward through the Kiwi line
- provide an obvious way back to the recorded replay

Do not introduce a large analysis dashboard in this pass.

Desktop browser is primary. Preserve the existing responsive/mobile viewer rather than designing Kiwi desktop-only DOM that breaks portrait/landscape.

## Tests required

Add tests before declaring the feature complete.

At minimum cover:

1. Kiwi artifact/module can load in the built Tetrp app.
2. WASM search is Worker-owned.
3. current + exactly NEXT 5 boundary.
4. hidden replay future changes do not change the request at the same visible position.
5. SevenBag observer alignment at bag boundaries.
6. empty-Hold consumes/reveals two draws; ordinary placement one.
7. deterministic suggestion for identical state + budget.
8. replay Reconstruction is unchanged after starting/advancing/stopping Kiwi.
9. a Kiwi placement is transported through Tetrp rules and reaches the intended final placement.
10. pending garbage never falls through the no-pending persistent path.
11. existing unit tests stay green.
12. existing Chromium/WebKit viewer + PWA tests stay green.
13. add a synthetic browser flow for invoke Kiwi -> suggestion -> advance -> second suggestion -> return to replay.

Do not use private replay fixtures in CI.

## Deliverable

Implement the feature in Tetrp, not just a design document.

At completion provide:
- changed files
- integration architecture in a short summary
- tests run and exact results
- any remaining product limitation that is real, reproducible and not merely future strategy tuning

Do not retune Kiwi during this task. Strategy research is intentionally decoupled from product integration.
