# Tetrp / Kiwi rule-parity and snapshot-product ledger

Updated 2026-09-20. Strategy tuning remains paused. Tetrp remains Phase 4A.
This round is a product/correctness follow-up to accepted artifact 10600729877,
workflow 35500047026, source/build
`15135ae8066c95308a3ae87ce149f5cdc7e13043`.

Read Tetrp `docs/KIWI_SNAPSHOT_PRODUCT_HANDOFF.md`. No entry here authorizes
Phase 4B, a Tetrp pin change, fresh H2, evaluator-weight changes, or a strength claim.

## Invariants which must not regress

- No replay-prefix scan, historical SevenBag recovery, piece-count modulo bag
  inference, hidden RNG/tail access, opponent future information or original-player
  future placements.
- Every external request is exactly current + NEXT5 with
  `bag_knowledge=unknown`, finite visible tail, public rules, clock/multiplier and
  already observable pending facts.
- Unknown tail is never expanded speculatively. A finite per-request search horizon
  is not a total continuation-length cap.
- Empty Hold consumes one draw and immediately refills visible NEXT5 before a new
  `hold_locked=true` request. Occupied Hold consumes no draw.
- Every lock/spawn refills from Tetrp's private isolated sequence; repeated requests
  may pass the initial six visible pieces without exposing hidden sequence state.
- Product requests are stateless and deterministic for the same allowed input.
- Default request cap is 200,000 evaluator nodes; post-Hold is a separate request.

## Snapshot-v3 repairs in this round

### Explicit same-piece Hold

Place and Hold are independent root actions. Same-type occupied Hold is not erased:
although the piece type is unchanged, Tetrp respawns/resets the active piece and
locks Hold, so the state transition is not generally identical to no-Hold.

Same-type empty Hold is also explicit and consumes NEXT[0] even when its type equals
current. The Hold action contains no landing and always requires re-analysis.

The preview revealed by an empty Hold is unknown at the pre-Hold request. Kiwi
does not infer or sample it and does not optimize its value-of-information.
The Hold branch is evaluated only through the currently known post-Hold prefix.
Capability `hold_information_gain_optimized=false` must remain false.

### Actual root geometry before Kiwi scoring

The pinned Tetrp helper enumerates the complete geometry-only current-piece landing
set from the actual active pose before Kiwi's first Place expansion. That allowlist
is applied inside root search. There is no top-K fallback; exceeding the explicit
geometry-state bound is a rejection rather than truncation.

x/y/rotation alone are insufficient. Root metadata now carries/audits hy, kick,
rotated, spin, totalRotations, resets, rotationResets, locking, forceLock, safelock,
softDropped and wall. The authority enumerator starts from the complete active Tetrp
piece, preserving SRS+ geometry/spin history.

Geometry reachability remains separate from input timing. Lock/reset timers,
handling/input state and future frame progression remain authoritative in Tetrp;
timing validation uses an isolated Engine clone. Do not claim arbitrary replay
positions are executable merely because a geometry candidate exists.

Dedicated fixtures cover non-spawn, wall, rotated, near-lock and spin-related roots.
Each result candidate is required to belong to the authority root allowlist and
the downstream helper reports its filtered candidate index.

### Stable rule/packet rejection surface

Snapshot-v3 freezes an explicit supported mechanical rule envelope in the adapter.
Values outside it reject with stable codes. Variable public attack rules continue
to be transported explicitly; Surge base/threshold, opener limit, all-clear,
garbage-special, Clutch switch, root Hold lock and attack clock/multiplier remain.

Important rejection codes include positive existing ARE queue, unknown activation,
hardened/shielded/unsupported pending status, unsupported rule values, and root
geometry/timing failures. Unknown activation is never filled with zero or guessed.

Pending that has not entered ARE remains a modeled approximation with explicit
activation timing. Positive existing ARE remains unsupported.

## Differential rule evidence

The new release workflow generates fixtures from pinned Tetrp
`0b48cb7e1a50e5f0bba6fcfee05ba8e291bebee2` and compares them against the exact
Kiwi source being built. The grid expands opener double-cancel across pending
amounts, cumulative-sent values and opener boundaries; Clutch spawn/clear rescue
cases retain topout/garbagesmash authority observations; storage-top garbage
insertion covers partial-vs-full top-row smash behavior. It also records pinned
Tetrp public-rule variants: supported variable attack-rule values must round-trip
field-for-field, while the Tetrp-visible b2bchaining=true variant must remain an
explicit unsupported Kiwi rule rather than being silently normalized.

The forecast was corrected so a partially occupied storage top row is not treated
as an immediate garbage smash; pinned Tetrp rejects insertion only at the tested
full-row boundary.

These fixtures are bounded evidence, not full parity. Keep
`rules_parity_verified=false`, full opener parity false, full Clutch parity false,
and exact ARE/bump false unless later evidence genuinely closes them.

## Remaining known limitations

- Empty-Hold unknown-preview information gain is not optimized.
- Geometry search is not a frame-accurate input-timing planner.
- Positive existing garbage ARE is rejected.
- Pending still uses the explicit review pace (normally 24F/placement), ten
  hypothetical clean-hole scenarios, integer clocking and simplified ARE/bump.
- Active-piece repair failure after an otherwise successful garbage insertion is
  not represented by the between-placement Forecast model.
- Complete terminal-reason/topout/garbage-smash classification is not certified.
- Unsupported custom rule variants reject rather than falling back to legacy values.
- Scores are evaluator heuristics, not win probabilities.
- Snapshot-v3 changes root search/compute allocation for correctness. No claim is
  made that its 200k request has the same strength as the historical speculative
  SevenBag 200k regime.

## Phase 4A downstream boundary

Phase 4A may show a landing-free Hold recommendation. Missing landing is expected.
If a post-Hold landing is desired, Tetrp must execute Hold in an isolated state,
refill the preview from its own private sequence, and send a new locked request.
Do not reuse any pre-Hold landing or infer a new one locally.

That isolated protocol test is not permission to ship user-visible continuation.
Tetrp's vendor pin is unchanged by this upstream task.

## Research validity

Pre-repair H9/H2 results remain historical/base-0-context evidence. H2 run
35491099927 remains `completed_diagnostic_base0_only`. No strategy experiment is
opened by snapshot-v3 work and `review_h9_h12` stays frozen for this artifact.


## Accepted snapshot-v3 release receipt

The accepted upstream delivery is source/build
`5c40af8e2970ab40b50381979671f8d52267514c`, workflow
`35503742126`, artifact `10602973298` (`kiwi-v1-browser`).
The uploaded archive passed the workflow's post-upload re-download verification
and a second independent download/verification pass: exact 28-file set, every
manifest SHA-256 valid, archive SHA-256
`13cf0771ed5c9cfe414a8f36b971ccea8ecc0c979792e67505bc508e95ee40c6`.

This receipt does not change the Tetrp pin, start Phase 4B, or create strategy
evidence. Downstream Phase 4A adoption remains a separate Tetrp repository task.


## Accepted snapshot-v3.1 compatibility release

Snapshot-v3.1 supersedes the previous v3 delivery for new Tetrp integration.
Accepted upstream identifiers:

- source/build commit: `60e75395be109e58d255a85ecd1cf0c981e696b3`
- workflow run: `35510089741`
- main artifact: `kiwi-v1-browser`, artifact ID `10605097237`
- post-upload verification artifact: `10605385940`
- independently downloaded archive SHA-256:
  `2b839ec06ec38a4297cbc673f14ff3913d4d396e94b761cabe0bb1f0d23feec3`
- exact delivered file set: 30 hashed files, all manifest hashes verified.

This release closes two adoption blockers found by downstream verification:

1. `tetrp-placement-path.mjs` is self-contained; it no longer imports an
   unshipped `tetrp-authority-adapter.mjs`.
2. TL no longer requires `garbageare=0` / `garbagearebump=0`.
   Real values are preserved, including synthetic/replay-compatible 5 / 12.
   Exact ARE/bump timing remains explicitly false and a positive current ARE queue
   remains an explicit unsupported state.

40L is supported only as separately labeled `competitive_stacking`: neutral root
combo/B2B, no pending garbage and no TL attack clock. It is neither represented as
TL nor claimed to optimize 40L sprint score/time.

Release acceptance now includes, after GitHub upload, re-downloading the artifact
on a fresh job, verifying exact file/hash/import closure, importing the packaged
placement helper, constructing real snapshots with pinned Tetrp, and searching
with the packaged web WASM. Both TL with ARE rules 5/12 and 40L competitive
stacking passed that downloaded-package E2E. Chromium/WebKit Worker tests and the
existing 200k deterministic fixture also remain green; the representative 200k
fixture completes at 197372 nodes with visible_search_idle.

This receipt does not change the Tetrp pin, authorize Phase 4B, open strategy
experiments, change evaluator weights, or claim improved playing strength.
