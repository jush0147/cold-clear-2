# Tetrp / Kiwi rule-parity blockers — 2026-09-20

This is the current blocking checklist before strategy tuning resumes.

The purpose is to separate **known public information that must be modeled exactly**
from **future-state uncertainty that may remain an explicit approximation**. Strategy
experiments are paused until the exact-rule side is repaired and regression-tested.

## Blocking rule/API gaps

### 1. Surge rule contract is wrong for the observed real TL replay

The current research harness constructs Tetrp TL with `g=0, gincrease=0` and
inherits the repository TL default `b2bcharge_base=0`. The real TL replay used by
the Phase 4A integration exposes `b2bcharge_base=3`.

Cold Clear also hard-codes the base-0 formula through `surge_size(count)`, so the
authority and bot are internally consistent with each other while still evaluating
the wrong public rule profile.

This invalidates pre-repair H9/H2 experiments as **real-TL promotion evidence**.
Keep them only as base-0-context diagnostics.

Required repair:

- transport public B2B/Surge parameters into every bot root;
- calculate Surge from the supplied rule profile, not a source-code constant;
- make the research harness explicitly choose the intended real-TL profile;
- regression-test threshold/base examples against Tetrp authority.

### 2. Root Hold lock is not expressible

After the player has already used Hold on the current piece, Tetrp exposes
`hold.locked=true`. The current Kiwi Start contract cannot forbid another Hold at
the search root, so Tetrp currently rejects those positions.

Required repair:

- transport a root-only `hold_locked` / `can_hold` fact;
- suppress Hold only for the current root;
- after the next lock/spawn, ordinary Hold availability resumes;
- keep explicit playback Hold semantics for same-piece current/Hold cases.

### 3. No-pending persistent search cannot use a non-unit attack multiplier

Pending snapshot analysis already transports authority frame, current multiplier,
margin and growth rate. Persistent `WasmBot.start()` does not.

Immediate safe routing is to use the snapshot analyzer even with `incoming=[]`
whenever the visible multiplier/clock cannot be represented by persistent search.
Longer-term continuation may carry the clock in persistent DAG state.

### 4. Opener double-cancel exists, but the contract/capability is stale

The current forecast path already implements the first-14-piece extra defensive
cancellation through `cancel_plan` and has regression tests. Therefore
`opening_double_cancel=false` is no longer an accurate description of the
implemented snapshot behavior.

What is still missing is a public rule parameter instead of the fixed 14-piece
assumption, plus explicit parity tests at the boundary.

### 5. Clutch support is partial, not absent

Move generation already has `find_moves_with_clutch` and enables spawn rescue
after a preceding clear. This should not be described as zero support, but full
Tetrp parity is not yet certified, especially exact high-board/topout/clear
interactions.

Keep this as a conformance task rather than silently claiming parity.

## Intentional approximation that is not a blocker by itself

Pending garbage remains a scenario model:

- explicit 24 frames/piece review pace;
- ten equally weighted unknown-hole scenarios;
- integer-frame timing;
- simplified ARE / garbage-ARE-bump behavior.

Those are acceptable only when disclosed as assumptions. Hidden future holes,
opponent future attacks or future replay confirms must never be used.

An observable packet with unknown activation time must continue to fail explicitly
instead of being ignored or assigned a guessed delay.

## Acceptance gate before strength tuning resumes

1. Public rule fields relevant to search are transported from Tetrp authority to
   Cold Clear and validated at the checked boundary.
2. Surge base/threshold, opener cancellation, attack multiplier and Hold-lock
   regressions pass.
3. The synchronous research harness asserts the exact intended rule profile.
4. A/A mirror, slot symmetry, transport correctness and WASM runtime gates pass
   after the rule change.
5. Pre-repair H9/H2 results remain archived but cannot promote a real-TL profile.
6. H2 restarts with fresh seeds. H3 must not begin before this gate is complete.

The rule source of truth is the replay-visible Tetrp authority state. Kiwi may
approximate unknown futures, but it must not approximate facts the replay already
makes public.
