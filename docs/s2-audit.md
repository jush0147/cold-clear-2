# S2 correctness audit

Current status: experimental offline review, not complete TETR.IO parity or
playing-strength certified.

The current implementation is described in [the review engine report](review-engine.md)
with [validation results](review-quality-results.json). It adds actual-move
comparison, explicit hold availability, concrete continuations and non-clairvoyant
scenario policies on top of the existing rules and static garbage model.

Historical evidence is retained in [clock-free review](clock-free-review.md) and
[recorded-game validation](replay-validation.md). The received-counter discrepancy
was resolved by distinguishing admission from confirmation, without changing
actual gameplay. All twenty recorded endpoints and audited counters still match.

Only own current/hold/next-five, board, combo/B2B, observed incoming and own
historical counters enter review. Replay RNG is exclusively a historical decoder
input, never a counterfactual bot input. Opponent board and future events are excluded.

Implemented code, regression tests, independent-model agreement, recorded endpoint
agreement and strategic quality are different claims. New first-step checks cover
all 3,455 reconstructed locks but do not prove future-policy optimality. Two-width
agreement measures sensitivity, not statistical confidence. A beam that finds no
continuation does not prove a forced KO.

Continue preserving all native/WASM and observation regressions while tuning.
Unverified timing/Clutch/topout edge cases, opening-rule examples absent from the
recording, finite-horizon bias and limited garbage scenario distributions remain.
Default review is deliberately clock-free and has no execution-speed penalty.
KO-only duels retain no piece cap or attack tiebreak. Earlier capped/extra-preview
scores are not valid strength evidence for this version.
