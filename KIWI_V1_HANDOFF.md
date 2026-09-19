# Kiwi v1 product handoff

This branch is a frozen product handoff for **Kiwi**, the TETR.IO Season 2 replay-review bot used by Tetrp.

The purpose of this branch is not to continue strategy research. It exists so Tetrp can integrate a stable browser artifact while strategy work continues elsewhere.

## Product identity

- UI name: `Kiwi`
- Product version: `kiwi-v1`
- Engine lineage: Cold Clear 2
- Source branch: `kiwi-v1`
- Frozen from `tetrp-authority@e13ec57e7f3795ea0c4dd7e256346d533fcb36e3`
- Intended Tetrp baseline at handoff: `jush0147/tetrp@0b48cb7e1a50e5f0bba6fcfee05ba8e291bebee2`
- Default review compute budget: **200,000 evaluator nodes per decision**

The persistent browser bot constructor is authoritative for Kiwi v1:

`new WasmBot()` -> `BotConfig::interactive_review()`

That product profile is the previously tuned review profile plus correctness fixes:

- H1 pending safety = 1
- H2 useful attack reward = 1, cancellation reward = 0
- H6C row-transition scale = 2.5x legacy
- H9 cavity excavation = -0.5
- H12 best-child demotion/backprop correctness = enabled
- H13 persistent-DAG despeculation/backprop correctness = enabled

Do not replace this profile with the current experimental reset profiles. Those belong to the research line, not Kiwi v1.

## Integration boundary

**Do not merge or vendor the Cold Clear 2 source repository into Tetrp.**

Tetrp should consume the browser artifact produced by the `Kiwi v1 browser artifact` workflow. The artifact is named `kiwi-v1-browser`.

The artifact contains:

- wasm-pack browser output, including `cold_clear_2.js` and `cold_clear_2_bg.wasm`
- generated TypeScript declarations/package metadata from wasm-pack
- `tetrp-authority-adapter.mjs`
- `tetrp-placement-path.mjs`
- this handoff document
- `kiwi-build.json` with the exact source commit and product contract

The copied adapter/path helpers are product integration references. Prefer reusing them or porting them with tests rather than re-deriving board orientation, bag visibility, hold accounting, or placement coordinates from memory.

## Authority split

Tetrp owns gameplay truth.

Tetrp is authoritative for:

- board state
- piece progression and SevenBag history
- Hold and exactly NEXT 5 visibility
- combo and B2B/Surge state
- incoming garbage timing and cancellation/tanking
- garbage travel time
- late-round attack scaling
- hypothetical continuation timing
- KO/topout

Kiwi owns:

- search
- move ranking
- placement choice

Never let Kiwi read:

- opponent board
- hidden future pieces beyond NEXT 5
- hidden bag RNG
- future opponent attacks
- replay future that is not yet visible at the selected decision point

The existing authority adapter enforces the intended visible-state boundary and should be treated as executable documentation.

## Browser API: persistent path

For a position with no incoming garbage requiring the pending-aware approximation, use one persistent `WasmBot` session.

```js
import init, { WasmBot } from "./cold_clear_2.js";

await init();
const bot = new WasmBot();

bot.start(JSON.stringify({
  board,
  queue,        // current + exactly NEXT 5, length 6
  hold,
  combo,
  back_to_back,
  b2b_count,
  randomizer: {
    type: "seven_bag",
    bag_state,  // derived only from already-observed draw history
  },
}));

bot.think_nodes(200000);
const suggestions = JSON.parse(bot.suggest_json());
const best = suggestions[0];
```

When the user advances the Kiwi line by one placement:

```js
bot.play_with_hold_json(JSON.stringify(best), usedHold);

const count = bot.preview_refill_needed();
for (let i = 0; i < count; i++) {
  bot.new_piece(newlyVisiblePieces[i]);
}

bot.think_nodes(200000);
const nextSuggestions = JSON.parse(bot.suggest_json());
```

Do not use `play_json()` for replay integration. Use `play_with_hold_json()` so an empty-Hold action is not confused with identical piece types.

Keep the same `WasmBot` alive across user clicks whenever the persistent path is valid. H13 exists specifically so newly revealed NEXT information can correct the retained DAG.

## Pending-garbage path

`WasmBot.start()` intentionally rejects pending-garbage fields. Do not silently drop incoming garbage.

When observable incoming garbage matters, use the snapshot API:

```js
const report = JSON.parse(analyze_pending_json(JSON.stringify(request)));
const best = report.candidates[0].placement;
```

Build the request with `buildAnalysisRequest()` from `tetrp-authority-adapter.mjs`.

Important properties:

- `incoming` contains only packets already observable by this player.
- Prefer exact `ready_in_frames` per packet.
- Supply the Tetrp attack clock fields together when available.
- The default product pace assumption is 24 frames per piece = 2.5 PPS.
- Total search budget remains 200,000 evaluator nodes, divided across garbage-hole scenarios.
- Pending analysis is snapshot-only and uses the frozen tuned H9+H12 strategy. H13 is not observable because the DAG is rebuilt.

For an interactive Kiwi continuation while pending garbage remains, Tetrp must own a separate hypothetical engine fork, apply Kiwi's chosen placement to that fork, advance authority time, then build the next visible snapshot. Do not mutate the recorded replay reconstruction.

Once the hypothetical line reaches a clean no-pending decision boundary, a persistent `WasmBot` session may be started from that state.

## SevenBag visibility

This is a correctness requirement, not an optimization.

At an arbitrary replay position, do not calculate `bag_state` from hidden replay future or from the engine's full internal queue.

Use `SevenBagObserver` from `tetrp-authority-adapter.mjs`.

The observer needs the draw history that was already visible to the player. When entering review at an arbitrary placement, reconstruct that observed history from the beginning of the round, then align it with current + NEXT 5.

A normal placement reveals one new preview piece. The first real Hold from an empty Hold reveals two.

## Placement transport into Tetrp

Kiwi returns a final placement, not Tetrp input events.

`tetrp-placement-path.mjs` contains the already-tested mapping from a Kiwi placement to a legal Tetrp input path, including:

- SRS+ rotations
- 180 rotation
- Hold
- CC2/Tetrp coordinate conversion
- spin classification
- hard drop

Do not directly paint the tetromino onto the board. Apply the move through Tetrp authority so line clears, B2B, combo, garbage and timing remain Tetrp-owned.

When scheduling a path, pass the live hypothetical Tetrp `Engine` into `schedulePath(startFrame, lockFrame, moves, engine)`. The helper first preserves the ordinary transport, validates it against an isolated authority clone, and only uses compact equal-subframe transport if ordinary synthetic tap spacing would auto-lock early or lock more than one piece. This guards reset-heavy high-stack finesse without changing the scheduled lock frame or Tetrp rules.

This is a correctness fix discovered by the H9 coarse-sweep failure on seeds 65206/65207: the old transport spaced a 17-reset path across source time, Tetrp auto-locked the intended Z early, and the later scheduled hard drop locked the following I as a second piece. Do not reintroduce an unvalidated scheduler.

The research harness used fixed pace only to advance authority time. Placement reachability must not depend on PPS or gravity.

## Recommended Tetrp architecture

Run Kiwi off the main UI thread.

A clean product split is:

```text
viewer UI
  |
  | selected replay decision / "Kiwi" action
  v
Tetrp analysis session (separate from recorded Reconstruction)
  |
  +--> Tetrp engine fork / observed SevenBag history
  |
  v
Kiwi worker
  |
  +--> wasm search
  |
  v
placement suggestion
  |
  v
Tetrp hypothetical authority applies placement
  |
  v
render Kiwi continuation
```

Do not let bot analysis mutate `ViewerSession` or the replay's canonical `Reconstruction`.

## Product behavior

The intended UX is interactive review, not full-replay automatic grading.

1. User watches a replay.
2. User pauses at an interesting placement boundary.
3. User invokes **Kiwi**.
4. Kiwi shows/plays one alternative placement.
5. Each further user action advances Kiwi by one placement.
6. The user can stop at any time and return to the untouched recorded replay.

No realtime gameplay automation is part of Kiwi v1.

## Minimum acceptance checks in Tetrp

Before calling the integration complete:

1. Kiwi assets load from the Tetrp static/PWA build without external network requests.
2. Search runs in a Worker and does not freeze playback UI.
3. A fresh decision uses exactly current + NEXT 5 and observed SevenBag state.
4. Hidden replay future cannot affect a suggestion.
5. Empty-Hold use reveals two previews; normal moves reveal one.
6. Repeated identical state + budget produces deterministic suggestion order.
7. 200k is the default budget.
8. Recorded replay reconstruction remains byte/state independent from Kiwi continuation.
9. Applying a Kiwi placement uses Tetrp rules, not direct board mutation.
10. Positions with incoming garbage are never passed to persistent `WasmBot.start()` as if garbage did not exist.
11. Existing Tetrp unit/browser/PWA tests remain green.
12. Add synthetic browser tests for invoke -> suggest -> advance one Kiwi move -> advance another move.

## Files worth reading before changing semantics

In this repository:

- `src/wasm.rs`
- `src/bot.rs`
- `src/analysis.rs`
- `scripts/lib/tetrp-authority-adapter.mjs`
- `scripts/lib/tetrp-placement-path.mjs`
- `docs/review-bot-design.md`

In Tetrp:

- `src/engine.js`
- `src/replay/reconstruction.js`
- `viewer/app.js`
- `viewer/worker.js`
- `viewer/session.js`
- `docs/PHASE_3_HANDOFF.md`

## Non-goals for the Codex integration pass

Do not:

- retune Kiwi parameters
- import the Cold Clear 2 Rust source into Tetrp
- change Tetrp gameplay rules to match Kiwi
- add opponent-state information to search
- use replay future to derive bag state
- turn Kiwi into live-play automation
- redesign the entire replay viewer before the bot path works
- block integration waiting for the experimental strategy branch to finish

Kiwi v1 is intentionally a frozen, usable product baseline. Later strategy research can ship as a new Kiwi build without changing this product boundary.
