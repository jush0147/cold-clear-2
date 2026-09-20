# Kiwi v1 browser handoff: snapshot API revision 2

This supersedes the earlier history-derived SevenBag/persistent-product instructions.
Read Tetrp `docs/KIWI_SNAPSHOT_PRODUCT_HANDOFF.md` and `docs/PHASE_4_PLAN.md`.
Tetrp stays at Phase 4A. No Tetrp files or vendor pin are changed by this release.
No strategy experiment or strength promotion is authorized by this work.

## Contract chosen before implementation

The request schema is `kiwi-snapshot/2`. `bag_knowledge` must be `unknown` and
`unknown_tail` must be `finite_visible`. There is no randomizer/bag_state input.
Do not infer bag remainder from draw history, current counters or hidden state.
Only known queue layers are searched; frontier leaves retain CC2's existing
heuristic evaluation. This is an explicit finite-horizon choice, not probabilistic
seven-bag inference. With the existing CC2 normalization the conservative known
horizon is five lock layers when Hold is empty and six when occupied; it does not
claim to use every possible terminal reserve placement. This search-policy change
has correctness tests, not a new strength claim. Evaluator coefficients are frozen
at `review_h9_h12`; do not treat old speculative-search KO results as validation.

The single product entrypoint is `analyze_snapshot_json(requestJson)`. Capabilities
come from `snapshot_capabilities_json()`, NOT legacy WasmBot capabilities.
Every request creates new search DAGs, including no-pending roots. Thus pending and
non-unit multiplier roots cannot accidentally take a clock-less persistent path.
The legacy `WasmBot`, `start`, `start_tetrp`, `analyze_pending_json`, and the old
SevenBag adapter remain compatibility APIs; they are NOT v2 product entrypoints.
There is deliberately no cross-request persistent DAG reuse in this version.

Use `kiwi-snapshot-adapter.mjs`:

```js
const visible = captureSnapshot(currentAuthorityState);
const request = buildSnapshotRequest(visible, {nodeBudget: 200000, framesPerPiece: 24});
worker.postMessage({type: 'analyze', id: requestGeneration, request});
```

`captureSnapshot` reads only the current projection. It never seeks or scans a
replay prefix. Pass only the resulting request into the dedicated Worker, not the
Engine/checkpoint/replay. `kiwi-snapshot-worker.mjs` is a directly usable module
Worker reference with local web WASM loading. Keep monotonic request generations
on the host. Cancel/exit by `worker.terminate()`; a synchronous WASM search cannot
process a queued cancel message. Do not add a UI-thread fallback. Drop branch,
request and result state on exit; static offline program caching is separate.

Request fields: `schema`, `bag_knowledge`, `unknown_tail`, `start` (bottom-up board,
current+NEXT5 queue, Hold, combo, back_to_back, b2b_count), `root_pose` (top-down x/y,
Tetrp rotation 0..3), explicit `rules`, `hold_locked`, incoming packets with mandatory
`ready_in_frames`, pieces_placed, garbage_sent, frames_per_piece, authority_frame,
authority_subframe, garbage_multiplier, garbage_margin_frames,
garbage_increase_per_second, and optional node_budget (default 200000).
Missing public rule fields and additional history/bag/hidden fields fail explicitly.
The adapter copies named fields; it does not forward unrelated caller properties.

## Explicit action protocol

Results use `kiwi-snapshot-result/2`, including ranked candidates, actual evaluated
nodes, budget, completion reason and assumptions. Each candidate's `action` is one
of these disjoint variants:

```json
{"kind":"hold","mode":"empty","requires_reanalysis":true}
{"kind":"hold","mode":"occupied","requires_reanalysis":true}
{"kind":"place","placement":{"location":{"type":"I","orientation":"north","x":4,"y":0},"spin":"none"}}
```

Hold actions contain NO landing placement or executable placement path. Apply only
Hold through Tetrp. Empty Hold consumes one draw and immediately reveals the next
preview; occupied Hold consumes none. Submit the new current+NEXT5 window with
hold_locked=true and perform a NEW search before any placement. A Place action uses
only the current piece, never implicitly Hold. After lock/spawn, Tetrp unlocks Hold
and refills the window. Each fresh request has its own node budget; a Hold request
never silently receives double budget.

Same-type Hold is not a separately searched action in this version. The explicit
protocol can validate such a Tetrp action, but Kiwi does not rank it as an alternative.
The capability `same_piece_hold_search=false` is intentional. Do not infer general
Hold support merely from differing placement types or claim information-gain
optimization for same-piece empty Hold.

The core is spawn-based and does not yet restrict search using current x/y/rotation.
Tetrp must validate geometry from the actual root pose. The supplied
`validateSnapshotAction(engine, action, {Engine, placementTools})` performs validation
on an isolated clone, without mutating its input. `selectReachableSnapshotAction`
can select the first reachable ranked action; it reports the candidate index.
If no candidate is reachable, fail explicitly. Do not paint cells or silently
execute a Hold-plus-placement bundle. The existing placement helper and reset-safe
scheduler remain available for AUTHORITY-side execution, not for input-speed claims.

## Progressive reveal is not a six-piece demonstration limit

Tetrp alone retains the original private sequence/generator in an isolated branch.
Advance it according to actual branch Hold/lock actions, not original-player frames
or placement numbers. Send only the now-visible six-piece window each time.
The acceptance harness executes ten locks plus Holds across repeated requests,
checking the branch sequence against a separate private audit model. This is an
API protocol test, NOT implementation or authorization of user-visible Phase 4B.
Never replay original-player future placements or future opponent attacks.
Both recorded players remain frozen; leaving analysis returns paused to the same
unchanged recorded checkpoint.

## Rules and unresolved limits

Explicit Surge base/threshold, opener limit, all-clear rules, garbage bonus and
Clutch switch are included from the repaired `tetrp-authority` source. The new
snapshot API always carries the authority clock, even with incoming=[] and
multiplier != 1. Unknown packet activation is rejected, not ignored or guessed.

Full opener packet/ARE parity and full Clutch clear/topout/garbage-smash parity are
still not certified. Do not change those capability flags just because the new
snapshot tests pass. Pending remains an approximation: explicit assumed pace,
ten hypothetical clean-hole scenarios, integer clock and simplified ARE/bump
handling. An existing positive ARE queue is rejected by the v2 projection. Nonzero
garbage-ARE/bump delays are not exactly modeled. Rules outside the projection's
supported contract fail; root geometry remains separately validated. Full parity
is false. Solo replay scoring requires separately disclosed neutral competitive
counter interpretation and compatible public rules; this release does not certify
all solo/custom-rule variants.

## Release verification

`kiwi-build.json` records exact source/build commit, source branch, workflow run,
artifact name, rule-source revision, product API and actual capabilities.
`kiwi-snapshot-acceptance.json` records Node-WASM protocol tests;
`kiwi-snapshot-browser.json` records real Chromium/WebKit dedicated-Worker tests.
Native snapshot tests prove unknown-tail nonexpansion, strict input handling,
determinism and locked roots with and without pending. These are NEW gates, not
claims borrowed from old A/A/WASM checks. Native regression log is included.
`sha256.json` records every artifact file except the hash manifest itself.
Both upstream licenses and THIRD_PARTY_NOTICES.md are shipped.

Consume only a successful `kiwi-v1-browser` artifact whose snapshot-v2 acceptance
reports both say passed. Changing Tetrp's pin and replacing its old history-scan
routing is a separate Phase 4A integration task; it has not happened automatically.
