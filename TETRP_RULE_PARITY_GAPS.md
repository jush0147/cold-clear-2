# Tetrp / Kiwi rule-parity ledger

Date: 2026-09-20

This file is the stop-sign for strategy tuning. Do not resume H-family promotion work
until the public TL rule contract below is either verified against Tetrp or the
remaining approximation is explicitly accepted for that experiment.

## Repaired in `tetrp-authority`

- Public attack-rule transport now carries:
  `b2bcharging`, `b2bcharge_at`, `b2bcharge_base`, `b2bchaining`,
  `openerphase_pieces`, `allclears`, `allclear_garbage`,
  `allclear_b2b`, `garbagespecialbonus`, and `clutch`.
- The synchronous TL authority now uses `b2bcharge_base=3` instead of the old
  base-0 default.
- CC2 Surge calculation is rule-driven instead of hard-coded to base 0.
- Snapshot analysis receives the same public rule contract as the authority.
- Root Hold lock is represented explicitly. Search may not Hold again at that
  root, while descendants regain Hold after the next lock/spawn.
- Opener-phase length is no longer hard-coded in cancellation helpers.
- Clutch spawn rescue is gated by the transported public `clutch` rule.
- Non-unit attack multipliers are supported by snapshot analysis even when
  `incoming=[]`, because the authority clock/multiplier is part of the request.

Current repaired authority gates at commit
`2b6c197e8bcb1cd9e86c8888e5dd7b66cc41c9d0` all pass:
Tetrp authority correctness, A/A mirror, slot symmetry, and WASM.

## Still incomplete / not certified

1. **Opening double-cancel full parity is not certified.**
   The forecast has the opener bonus cancellation mechanism and a configurable
   opener-phase length, but `capabilities_json()` still reports
   `opening_double_cancel=false`. Add direct Tetrp fixture comparisons for the
   complete pending/cumulative-sent semantics before calling this parity-complete.

2. **Clutch is only partially certified.**
   Spawn rescue is implemented and rule-gated, but the browser capability still
   reports `clutch_clears=false`. Verify the full clear -> spawn-rescue transition
   against Tetrp fixtures, including topout/garbage-smash edge cases.

3. **Pending-garbage search remains an approximation.**
   It uses an explicit assumed pace, currently 24 frames/piece for the review
   regime, integer-frame timing, and ten equally weighted hypothetical clean-hole
   scenarios. ARE / garbage-ARE-bump timing is not yet an exact authority replay
   inside CC2. Unknown activation times must continue to fail explicitly rather
   than be guessed or ignored.

4. **Same-piece Hold is not a distinct search action.**
   Root Hold locking is now supported, but when current and Hold/next can yield the
   same piece type, action identity is still placement-based and does not rank
   "Hold same piece" separately from "do not Hold".

5. **Persistent no-pending search still needs product routing for late multipliers.**
   The CC2 snapshot API can model `incoming=[]` with a non-unit multiplier, but
   Tetrp/Kiwi must route such roots through that snapshot path until persistent DAG
   state itself carries the authority attack clock.

6. **Full rule parity remains false.**
   Unsupported rule variants such as `b2bchaining=true` must fail explicitly.
   Do not silently substitute legacy defaults for replay-visible public rules.

7. **The shipped Kiwi v1 artifact is still the old pin until rebuilt.**
   Tetrp currently vendors Kiwi artifact commit
   `89dcfe6cf544991bc9bb59098dd43d2ca2173945`. The repaired rule contract lives
   on `tetrp-authority` and must be deliberately rebuilt/pinned into `kiwi-v1`
   and then consumed by Tetrp before the browser limitation text can be removed.

## Research validity

- H9 and H2 results produced before the real-TL base-3 repair remain useful only
  as historical / diagnostic evidence for the old base-0 context.
- H2 local refinement run 35491099927 completed successfully but is explicitly
  archived as `completed_diagnostic_base0_only`; it cannot select or promote a
  real-TL finalist.
- Do not launch fresh H2 promotion research merely because the base-3 Surge fix
  compiles. Finish the parity checks above first, then restart H2 with fresh seeds.
