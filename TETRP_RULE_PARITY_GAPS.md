# Tetrp / Kiwi rule-parity and snapshot-product ledger

Updated 2026-09-20. Strategy tuning remains paused. Separate source implementation,
behavioral evidence, accepted archive, and downstream adoption. Old green gates do
not establish new product requirements. No Phase 4B is authorized by this ledger.

## Product authority and proposal history

Read Tetrp `docs/KIWI_SNAPSHOT_PRODUCT_HANDOFF.md` (blob
`eb7ece11f289b670d2953368dfa2166846ec4a8a`) and `docs/PHASE_4_PLAN.md`.
The snapshot supplement supersedes historical SevenBagObserver/prefix recovery.
The contract below was recorded before implementation in research commit
`e8aabe80240b5cf5051d60e3d777afd31e633354`. Implementation is on kiwi-v1,
starting at `6d68c2ee9ed5f93852f1913fa3882984d0048a1e`. Research branch source
has not silently been switched to this product search mode.

## Implemented snapshot-v2 contract

- Every request uses only the detached current visible snapshot. No prior draw
  log, prefix scan, bag remainder, piece-count-modulo inference, hidden tail/RNG,
  opponent board or future original placement. Current combo/B2B, time,
  multiplier, counters and already observable pending remain real current facts.
- `bag_knowledge=unknown`, `unknown_tail=finite_visible`. No speculative expansion
  beyond known queue layers; terminal leaves retain the existing heuristic.
  The internal unused bag bitset is NOT a known fresh-bag distribution.
  CC2's conservative empty-Hold normalization gives five known search layers;
  occupied Hold gives six. This is a per-request horizon, NOT a continuation cap.
- APIs: `analyze_snapshot_json(request)` and `snapshot_capabilities_json()`.
  Strict schema kiwi-snapshot/2 rejects randomizer/bag/history/extra fields.
  All public rule fields are explicit, not silently defaulted.
- Tagged `hold` or `place` actions. A Hold result has NO landing or placement path.
  Empty Hold consumes one draw, immediately reveals one preview and MUST be
  followed by a new snapshot with hold_locked=true. Occupied Hold consumes none.
  Place uses only current. Next lock/spawn restores Hold availability.
- Same-type Hold is NOT separately ranked. This remains an explicit limitation,
  not an inferred claim of complete Hold or information-gain search support.
- All roots, including incoming=[] and non-unit multipliers, use the same
  stateless clock-aware snapshot path. Each request creates a fresh DAG. Legacy
  WasmBot and SevenBag entrypoints are compatibility APIs, not v2 product routing.
- Tetrp owns original private sequence advancement. After each actual draw it
  refills NEXT 5; only the new window crosses the Worker boundary. Do not use
  original-player placement/frame indices or future boards as branch continuation.
- Default hard cap is 200000 evaluated nodes PER REQUEST. Post-Hold re-analysis
  is another request. Finite-visible search can become idle below that cap; report
  actual nodes and completion. No performance/strength equivalence to the previous
  SevenBag speculative 200k regime is established by these correctness tests.
- Root pose is transported, but core generation is still spawn-based. Validate
  actual-pose reachability on a detached Tetrp clone and expose filtered candidate
  index. No reachable action means an explicit failure, not invented cell painting.
- Reference module Worker loads local WASM. Host cancellation terminates the
  Worker. Repeated/restarted requests are deterministic; no cross-request DAG or
  analysis-history persistence. Preserve frozen recorded checkpoints on exit.

## New product evidence, separate from old gates

Release-source run 35499464310 passed native snapshot tests, WASM builds, Node
protocol acceptance and actual Chromium/WebKit module-Worker tests. Evidence:

- Strict request boundary and throwing history/hidden-NEXT/RNG/opponent getters.
- Unknown-tail speculative expansions remain zero with and without pending.
- Hold results expose no landing; empty Hold immediately reveals a preview;
  occupied Hold consumes zero draws; locked re-analysis places only current.
- Ten locks plus three Holds through thirteen decisions, consuming twelve draws
  against a separate authority-only original-sequence audit model. NEXT 5 is
  replenished beyond the initial window; both recorded checkpoints are unchanged.
- Pending, unknown activation rejection, explicit base-3 rules, late multipliers,
  no-pending clock routing, small and default hard node caps.
- Real Chromium/WebKit Workers: deterministic repeated/restarted requests,
  responsive host timers, no late reply after termination, local-only requests.
- Representative default-budget fixture used 159254 of 200000 nodes. This is
  reported early-idle behavior, not evidence of a missing continuation window.

Archive from run 35499464310 is SUPERSEDED for delivery: its hash manifest included
pkg/.gitignore, which upload-artifact excluded. The corrected packaging removes
that generated hidden file and adds a post-upload re-download gate checking the
exact file set and every SHA-256 before notification. Use the successful repaired
build's kiwi-build.json and release receipt, not the superseded archive.

## Public-rule repairs and bounded differential evidence

Repaired source carries b2bcharging, b2bcharge_at/base, b2bchaining,
openerphase_pieces, allclears, allclear_garbage/b2b, garbagespecialbonus and clutch.
Research authority explicitly selects base 3. Legacy defaults stay base 0;
product requests carry the actual public rule contract. Root Hold lock,
configurable opener limit, rule-gated Clutch rescue and snapshot clock are present.

Separate run 35499811778 tested exact product source
`6d68c2ee9ed5f93852f1913fa3882984d0048a1e` against pinned Tetrp
`0b48cb7e1a50e5f0bba6fcfee05ba8e291bebee2`: 546 fixtures passed, consisting
of 432 normal-pending opener/cancellation cases, 112 spawn-rescue cases and two
representative clear-to-rescue transitions. Artifact 10602360303 contains the
fixtures, counts and audit log. This verifies those cases, NOT complete rule parity.

Remaining limitations:
- Full opener packet/ARE/shield/hardened behavior and cumulative-sent edge coverage
  remain incomplete. Preserve false full-parity capability declarations.
- Clutch spawn rescue and representative clear transitions are tested; full
  terminal-reason/topout/garbage-smash classification is not certified.
- Pending remains approximate: explicit assumed 24F/placement, ten hypothetical
  clean-hole scenarios, integer time and simplified ARE/bump. Positive existing
  ARE queue is rejected. Unknown activation fails, never guessed or discarded.
- Unsupported public rule variants fail explicitly. Supported-rule coverage,
  finite search, and actual-pose ranking remain bounded. Scores are not win odds.

## Delivery and downstream boundary

Accepted package must include web WASM/glue, snapshot adapter and Worker,
placement helper, capabilities, kiwi-build.json, sha256.json, a file verifier,
Node and browser test reports, native log, licenses, notices and updated handoffs.
A successful post-upload archive check is now required. Documentation alone is
not a runtime repair; exact release commit/run/artifact are recorded separately.

Tetrp's existing Phase 4A pin and history-scan routing are NOT changed by this work.
Consuming the new artifact and validating the Phase 4A adapter is a separate
integration step. No user-visible continuation or Phase 4B was implemented.

## Research validity

Pre-repair H9/H2 outcomes remain base-0-context evidence. H2 run 35491099927 is
completed_diagnostic_base0_only, not a real-TL finalist or promotion. No strategy
experiment was opened for this product repair. Evaluator weights stay at
review_h9_h12; the new unknown-tail policy has correctness, not strength evidence.
