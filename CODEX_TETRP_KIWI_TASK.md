# Tetrp consumer task: Kiwi snapshot-v2 artifact

Scope remains Phase 4A. Read KIWI_V1_HANDOFF.md and the product supplemental handoff.
This artifact does NOT authorize Phase 4B or new strategy tests.

Before integrating, verify kiwi-build.json, sha256.json, and both new snapshot test
reports. Pin the exact successful kiwi-v1-browser artifact. Vendor generated JS/WASM
and helpers, not Rust source, another branch's bot, or a replacement implementation.

Replace observed-prefix SevenBag scanning with captureSnapshot/buildSnapshotRequest
from the current detached state. No bag inference from history, piece counts or RNG.
Use analyze_snapshot_json for every product request, including empty incoming and
non-unit multiplier. Do not call the old WasmBot/SevenBag product path.

Handle explicit Hold actions separately. A Hold result has no landing; empty Hold
must reveal one preview before locked re-analysis. A Place result cannot secretly
Hold. Same-piece Hold is not separately ranked. Validate geometry from the actual
pose; fail if unsupported. Preserve pending and public rules, hard request budgets,
Worker isolation, cancellation and unchanged frozen recorded checkpoints.

No user-visible continuation is implemented here. Future authorized continuation
uses Tetrp's private original-sequence cursor, reveals NEXT 5 after every actual draw,
and is not capped at the initial six visible pieces. No future original placements
or opponent attacks may enter analysis.

Old Tetrp Phase 4A tests and old Kiwi gates are insufficient for this contract.
Add downstream tests for no history scan, worker message allowlisting, Hold-reveal
interaction, no stale results, late multiplier routing and unchanged checkpoints.
Keep unsupported behavior explicit and retain remaining parity warnings.
