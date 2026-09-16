# S2 correctness audit

Current status: experimental offline static review, not complete TETR.IO parity
or playing-strength certified.

The latest implementation and tests are documented in
[clock-free review and receive resolution](clock-free-review.md). This supersedes
older reports of unresolved received counters and mandatory guessed piece timing.
See [recorded-game validation](replay-validation.md) for the preceding SRS+/180,
attack, and pending-search work and its historical evidence.

Inputs are own board, actual current/hold, five NEXT, combo/B2B, observed incoming
packets and own history. Replay RNG is used only by the offline historical decoder,
never as input to the counterfactual bot. The opponent board is excluded.

Default review is zero gravity with no execution-speed penalty. Garbage activation
is a separate, explicitly static observation model; it is not inferred from PPS.

Implemented code, regression tests, independent-model comparisons, recorded
endpoint agreement, and broad rules certification remain distinct evidence levels.
Forty-four native tests and native/WASM recorded and repeated-review checks passed
at implementation 0d6e5aac26efd7b935cfbd220c7c5c6ff620e560. Full-recording checks
were rerun locally, including all twenty received counters, with no discrepancies.

All new strategy work must retain these checks. Tuning can now proceed under the
stated static objective without pretending to solve real-time pace prediction.
Exact live timing, unobserved opening-rule cases, uncertainty-aware future decisions
and complete Clutch/topout coverage still need additional evidence.
