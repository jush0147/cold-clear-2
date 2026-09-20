# Tetrp / Kiwi rule-parity and snapshot-product ledger

Date: 2026-09-20

Strategy tuning remains paused. This ledger separates implemented source, verified
behavior, released artifacts, and downstream adoption. A green old gate is not
acceptance evidence for a new product contract.

## Supplemental product authority

Read Tetrp `docs/KIWI_SNAPSHOT_PRODUCT_HANDOFF.md` (blob
`eb7ece11f289b670d2953368dfa2166846ec4a8a`) alongside `docs/PHASE_4_PLAN.md`.
The supplement supersedes the earlier product requirement for SevenBagObserver
history recovery. Tetrp remains at Phase 4A. This work authorizes Kiwi API/artifact
repairs and isolated protocol tests, NOT user-visible Phase 4B or new experiments.

## New product gaps: implementation and acceptance pending

1. Each decision must use only a detached visible snapshot. No prior draw log,
   replay-prefix scan, historical bag remainder, piece-count-modulo inference,
   hidden tail/RNG, opponent board, future attack, or future original placement.
   Combo/B2B, time, multiplier, counters and already observable pending are valid
   current facts and must not be reset merely because history is forbidden.
2. Unknown bag is not a fresh complete SevenBag. The proposed v2 policy is an
   explicit unknown-bag, finite-visible-horizon search: no speculative piece beyond
   the supplied window, terminal existing evaluator at its boundary. The existing
   CC2 empty-Hold normalization has five known search layers; occupied Hold has
   six. This is a conservative per-request horizon, NOT a continuation length cap.
3. Proposed API: `analyze_snapshot_json(request)` and
   `snapshot_capabilities_json()`, with a versioned snapshot-only request that
   cannot accept `bag_state` or a randomizer. Return explicit tagged `hold` or
   `place` actions. A Hold result contains NO executable landing placement.
4. Empty Hold is executed alone by Tetrp, consumes one sequence draw, immediately
   reveals one preview, and requires a new snapshot with `hold_locked=true`.
   Occupied Hold consumes no draw and leaves NEXT 5 unchanged. A lock/spawn
   restores availability and refills the window from Tetrp's private sequence.
   Never execute a previously bundled pre-Hold landing after the reveal.
5. The initial v2 search explicitly excludes a separate same-piece Hold branch.
   This is a declared search limitation, not inferred general Hold support. All
   represented Hold decisions have explicit action identity at the public API.
6. Proposed product routing uses the stateless snapshot path for ALL requests,
   including no-pending roots and non-unit multipliers. Legacy WasmBot interfaces
   remain compatibility APIs, not conforming v2 product entrypoints. Recreating
   the search for every request avoids history-dependent persistent-DAG reuse.
7. Tetrp alone owns original-sequence consumption. Successive snapshots may
   continue beyond the initial six visible pieces; only newly visible previews
   cross the Worker boundary. Do not copy the original player's future board.
8. Default budget is 200,000 evaluated nodes per request. Post-Hold re-analysis
   is another request. Report actual nodes and early completion. Worker-only
   execution, cancellation/disposal, unchanged recorded checkpoints, and no
   analysis-history persistence remain required.

Required new evidence: history/hidden-tail invariance; strict input boundary;
unknown-tail nonexpansion with and without pending; explicit Hold action/no landing;
empty-Hold immediate reveal and locked re-analysis; occupied-Hold zero draw;
repeated lock/refill beyond six pieces; original-sequence consumption; rule/clock
routing and hard budgets; detached replay checkpoints and discarded sessions.
Old authority, A/A, symmetry and WASM gates do not establish these properties.

## Rule repairs already in tetrp-authority source

- Public rule transport: b2bcharging, b2bcharge_at/base, b2bchaining,
  openerphase_pieces, allclears, allclear_garbage/b2b, garbagespecialbonus, clutch.
- Synchronous research authority explicitly selects Surge base 3. Legacy
  compatibility defaults remain base 0; a product must pass explicit public rules.
- Rule-driven Surge and snapshot rules; root-only Hold locking; configurable
  opener limit; rule-gated Clutch spawn rescue.
- Snapshot analysis accepts incoming=[] and the complete attack clock.

Existing repaired-source gates passed at
`2b6c197e8bcb1cd9e86c8888e5dd7b66cc41c9d0`: authority correctness, A/A mirror,
slot symmetry and WASM. They predate the snapshot supplement above.

## Remaining rule limitations

- Full opener double-cancel parity is not certified. Forecast has an opener
  mechanism, but direct Tetrp fixtures must cover packet/cumulative-sent and
  opener-boundary behavior. Preserve truthful capability flags.
- Clutch spawn rescue exists; full clear/spawn/topout/garbage-smash edge parity
  remains uncertified and needs separate authority fixtures.
- Pending search is approximate: explicit 24F/placement assumption, ten equally
  weighted hypothetical clean-hole scenarios, integer timing and simplified
  ARE/bump handling. Unknown activation must fail, not be guessed or discarded.
  New snapshot visibility does not silently approve execution timing assumptions.
- Unsupported public rule variants must fail explicitly. Full rules parity is
  still false; finite-horizon search is not a win-probability guarantee.

## Delivery gate

Tetrp currently pins `kiwi-v1-browser` build `35444205867`, commit
`89dcfe6cf544991bc9bb59098dd43d2ca2173945`. It still scans observed history and
has the older Hold/rule behavior. Documentation updates are NOT runtime fixes.

Release repaired source deliberately on kiwi-v1; include browser/WASM, new snapshot
adapter, placement helper, updated capabilities, kiwi-build.json, hashes, test
reports, licenses and handoff. Record exact source/build commit, run/artifact IDs
and unresolved limits. Do not change Tetrp's vendor pin or begin 4B in this task.

## Research validity

Pre-repair H9/H2 outcomes remain base-0-context evidence only. H2 run 35491099927
is archived as completed_diagnostic_base0_only and cannot select/promote a real-TL
finalist. No strategy run is authorized here. Keep the incumbent and evaluator
weights frozen during the product and correctness repairs.
