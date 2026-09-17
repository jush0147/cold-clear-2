# S2 correctness audit

> **Historical snapshot.** This document predates later pending-aware search and KO strategy experiments. It is retained for rule-conformance cautions and audit history, but it is **not the current strategy roadmap**.
>
> For current bot-strength work, scope, information boundaries, KO protocol, H1 evidence, and the next proposed hypothesis, read [`docs/strategy-experiments.md`](strategy-experiments.md) first.

Status at the time of this audit: **experimental, NOT TETR.IO-conformant or strength-certified**.

This branch is intended for retrospective analysis of a player's own position.
It does not read an opponent board or connect an automated player to TETR.IO.
Correct simulation and the observation boundary take priority over evaluator tuning.

## Evidence levels

1. **Implemented**: code exists. This alone says nothing about correctness.
2. **Regression-tested**: the specified behavior is protected by tests.
3. **Independent-model checked**: an independently expressed reference agrees.
4. **Game-conformance checked**: versioned, independently captured TETR.IO
   fixtures agree. No subsystem currently claims complete level 4 coverage.

A green CI run proves the tests passed at that revision. It does not turn test
expectations written alongside the implementation into a production-game oracle.
`WasmBot.capabilities_json()` reports `rules_parity_verified: false`.

## Observation boundary

The browser input contains a post-clear, start-of-piece decision board, current
piece, actual hold (including null), exactly five visible NEXT pieces, and
combo/B2B state. Rows are bottom-up; boards shorter than 40 rows are padded above,
not below. More than 40 rows, invalid row widths and more than five NEXT pieces
are rejected. Mid-fall decisions are not supported by this spawn-based interface.

`combo` in the legacy Start transport is the number of consecutive line-clearing
placements *before* the next move. It is not necessarily the displayed counter
from the previous clear. The player-state output calls this `consecutive_clears`
to make the distinction explicit. Unsupported large counters are rejected at the
checked input boundary rather than silently clamped.

Never copy a replay's full internal state directly into the bot. Hidden future
pieces, RNG seeds, future attack events, unrevealed hole coordinates and opponent
state are not observations. Seven-bag deductions may eventually use observed past
pieces, but must not use the replay's hidden randomizer state as an oracle.
Exact garbage-layout visibility still needs independent UI/trace verification.

The checked browser interface refuses unknown fields, including `incoming`,
because pending garbage is not yet in DAG search. It is better to return an
explicit unsupported-input error than analyze a threatened board as if no attack
were incoming.

## Empty hold normalization and the corrected adapter

CC2 internally normalizes an empty hold by putting the current piece into
`reserve` and the NEXT pieces into its internal queue. This is a useful search
abstraction, not an actual hold performed by the player.

The boundary now remembers whether hold is genuinely empty:

- Playing current without holding keeps hold empty and reveals one new preview.
- The first actual hold stores current, plays NEXT[0], and reveals two previews.
- Swapping an occupied hold consumes one queued piece and reveals one preview.
- When current and NEXT[0] are identical, the placement's piece type cannot tell
  whether hold was used. Replay playback must pass the explicit hold boolean.

Use `player_pieces()` / `player_state_json()` rather than interpreting the raw
`GameState.reserve` as real hold. Use `preview_refill_needed()` after placement;
never assume every placement reveals exactly one piece. The existing DAG still
has a shorter fully known horizon in the normalized empty-hold case. That is a
search limitation, not permission to reveal extra unknown pieces.

## Repairs covered by regression tests

- Short board input no longer indexes beyond its row count or traps WASM.
- Invalid start states and hidden extra previews are rejected at the checked API.
- Wrong-piece, unreachable and wrong-spin replay placements do not mutate state.
- Explicit empty/same-piece hold decisions preserve actual current/hold/NEXT.
- Optimized collision maps now have the same 40-row ceiling as Board::occupied.
- Out-of-range anchors do not perform shifts by 64 or more.
- The move-generation priority queue processes lower soft-drop cost first.
- A non-clear cannot send attack, PC bonus or Surge because of stale flags.
- A missing search suggestion with legal moves remaining is an error, not a KO.

Independent reference checks currently include:

- 4,096 bitboard compaction cases against a simple row-grid implementation.
- 1,024 line-detection cases against explicitly generated row masks.
- Optimized collision against cell-based collision across all seven pieces,
  four rotations, boundary coordinates and ceiling positions.
- Generated placements checked for in-bounds cells, no overlap and grounding.
- 30,618 normal garbage-queue cases against a separate per-line boolean model.

These counts are reference-model test cases, **not simulated matches**.

## Pending garbage: current implementation boundary

`src/garbage.rs`, exported as `tetrio::garbage`, accepts ordered packets with
positive line counts and their *current* active/pending state. It intentionally
has no hidden hole or future activation timestamp field.

It implements and tests a specifically named **one-to-one cancellation policy**,
partial cancellation across packet boundaries, excess outgoing attack, active
FIFO-prefix removal, a supplied rise cap and line-clear blocking. It preserves
pending/active status and does not advance a clock. Counter conservation and the
per-line reference model are tested exhaustively over a small range.

`preview_garbage_one_to_one()` exposes this diagnostic primitive to WASM.
It is not pending-aware move selection. It does not implement opening double
cancellation, delayed Surge packets, evolving activation, garbage insertion into
a hypothetical board, or uncertain hole scenarios. The queue is NOT yet included
in each DAG node/key/transition. `pending_garbage_in_search` remains false.

The next integration must update both child expansion and DAG traversal using
the same transition, and must include every strategically relevant queue/timing
state in transposition keys. Adding a field or a root-only bonus is insufficient.
Unknown future activation/holes must be modeled as uncertainty, not filled in
from later replay events.

## Rule-conformance gaps

| Area | Current status |
|---|---|
| Combo/B2B transitions | Internal regressions, not complete game-fixture coverage |
| Attack, PC and mini-spin table | Draft; independent fixture verification required |
| Combo multiplier/minimum edge cases | Must verify both formula and operation order |
| SRS+ I kicks and 180 rotation | Not implemented/certified; inherited SRS CW/CCW |
| O rotations and all-spin corner cases | Not complete |
| Garbage-special +1 | Missing; board currently retains occupancy, not garbage provenance |
| Opening first-14-piece cancellation | Missing |
| Pending garbage inside DAG | Missing |
| Garbage travel, activation and timing | Missing |
| Surge segmentation and cancellation timing | Missing |
| Clutch Clears and exact topout timing | Missing |
| Deterministic full search | Not guaranteed; only the harness piece sequence is seeded |
| Reproducible locked dependencies | Lockfile/toolchain pinning still needs a separate pass |

### Do not turn ambiguous patch-note prose into a correction

The official Season 2 notes say All Clears send 5 garbage. That statement alone
does not specify whether 5 replaces or supplements ordinary clear attack. The
existing additive +5 behavior is retained rather than replaced on that sentence
alone. Its regression test is deliberately NOT named a game-conformance test.
The mini-spin descriptions also need to be reconciled with B2B, combo and exact
reward-table settings, rather than guessed from a shared enum name.

Primary starting point: <https://tetr.io/about/patchnotes/>.
Relevant rule changes span 2024-08-16, 2024-09-22, and 2025-01-18/25. A replay
must use a pinned rule profile appropriate to its version, not a timeless label
of "Season 2". `scripts/inspect-rule-evidence.py` only discovers public client
references and records hashes; successful fetching is NOT conformance evidence.

## Acceptance gate before resuming strength tuning

Capture independent fixtures with provenance: game version/options, decision
snapshot, explicit inputs/hold decision, observed before/after board, clear/spin
classification, combo/B2B, attack packets, cancellations, activations and topout.
Expected outputs must come from independent observations, not from this engine.

For each supported fixture compare the full transition, not just total attack.
Include no-clear/clear, first and subsequent B2B, PC combinations, T/non-T minis,
SRS+/180 reachability, high boards/Clutch, garbage-clearing bonuses, opening
cancellation boundaries, partial packets and Surge timing.

For every new feature: add a failing fixture/regression first, implement the
smallest change, rerun native debug/release and WASM runtime checks. Only after
rule/observation conformance should evaluator weights be swept again.

## Duel results and limits

The duel remains KO-only with no piece cap or attack tiebreak. Invalid budgets,
unsupported arguments, failed search with legal moves, and external timeouts
are errors/incomplete runs, never wins or draws invented to finish a benchmark.

The simulator still alternates whole placements and uses simplified garbage and
topout rules. Therefore its KOs are not certified TETR.IO KOs. Both contenders
share the modified core; `legacy` means legacy evaluator, not the original
untouched upstream engine. Earlier preview-12 / capped-score results must not be
used as evidence for this revised branch. CI strategy sweeps are paused.
