# Kiwi v1 browser handoff: snapshot API revision 3.1

This revision is the compatibility/packaging follow-up to snapshot-v3. The previous
v3 archive exposed a placement helper whose relative import was not shipped, and
its adapter over-pinned source rules to mode=tl, garbageare=0 and
garbagearebump=0. Revision 3.1 fixes those adoption blockers without restoring
history-derived bags or speculative tail search.

Read Tetrp `docs/KIWI_SNAPSHOT_PRODUCT_HANDOFF.md` and `docs/PHASE_4_PLAN.md`.
Tetrp remains at Phase 4A. This release does not change the Tetrp vendor pin, does
not implement user-visible continuation, does not authorize Phase 4B, and does not
start or validate a strategy experiment. Evaluator coefficients remain
`review_h9_h12`.

## Snapshot contract remains history-free

The product entrypoint is still `analyze_snapshot_json(requestJson)`. Revision 3
uses request `kiwi-snapshot/3` and result `kiwi-snapshot-result/3`.

Every request contains exactly current + NEXT 5 and explicitly declares an unknown
bag with a finite visible tail. Do not send or derive a SevenBag remainder. Do not
scan earlier draws, use piece-count modulo, inspect hidden queue/RNG state, or pass
future original placements/opponent attacks. The Worker receives only the detached
allowlisted request. Each request builds fresh search state; there is no product
persistent DAG reuse.

The finite known horizon is per request, not a continuation-length limit. Tetrp
owns the private original sequence in an isolated authority branch and reveals a
new preview only when an actual branch action consumes a draw.

All TL roots use the same clock-aware snapshot API, including incoming=[] and
non-unit attack multipliers. 40L-source competitive_stacking uses the same
stateless snapshot API but deliberately carries no TL attack clock. The default
hard cap remains 200,000 evaluated nodes
per request. When both Place and Hold are available, that one request divides its
cap deterministically between the two root branches. A required post-Hold search
is a separate request and gets its own request cap. Always use the returned actual
`nodes` and `completion` fields; finite-visible search may finish early.

## Same-piece Hold is now an independent action

Revision 3 never infers Hold merely from a different piece type. Root choices are
explicitly modeled as separate Place and Hold branches.

Examples:

```json
{"kind":"place","placement":{"location":{"type":"T","orientation":"north","x":4,"y":0},"spin":"none"}}
{"kind":"hold","mode":"empty","same_piece":true,"requires_reanalysis":true}
{"kind":"hold","mode":"occupied","same_piece":true,"requires_reanalysis":true}
```

A Hold action NEVER contains a landing or path.

Empty Hold consumes NEXT[0], changes Hold, immediately reveals one new preview,
and then requires a fresh current+NEXT5 request with `hold_locked=true`. Occupied
Hold consumes no draw, but still changes the active-piece spawn/pose and locks Hold,
so even an occupied same-piece exchange is not treated as a type-equivalent no-op.

The pre-Hold request cannot know the preview revealed by an empty Hold. Kiwi scores
that Hold branch only over the already visible post-Hold prefix. It does NOT sample,
infer, peek, or optimize the value of the unknown reveal. Capability
`hold_information_gain_optimized=false` is intentional. The real revealed preview
is considered only by the mandatory post-Hold request.

Minimal Phase 4A pattern:

```js
const visible = captureSnapshotFromEngine(authorityFork, placementTools);
const request = buildSnapshotRequest(visible, {
  nodeBudget: 200000,
  framesPerPiece: 24,
});
worker.postMessage({type: "analyze", id, request});

// result.action.kind === "hold" is a valid Phase 4A recommendation.
// Do NOT invent or reuse a landing.
if (result.action.kind === "hold") {
  const postHoldFork = applyHoldForReanalysis(authorityFork, result.action, {Engine});
  const postVisible = captureSnapshotFromEngine(postHoldFork, placementTools);
  const postRequest = buildSnapshotRequest(postVisible, {
    nodeBudget: 200000,
    framesPerPiece: 24,
  });
  // Send postRequest as a NEW Worker request. It has hold_locked=true.
}
```

Phase 4A may display "建議 Hold" directly. A missing landing is not an error.
Obtaining a landing after Hold requires the isolated Hold/refill/new-request
sequence above. That protocol does not authorize user-visible Phase 4B continuation.

## Actual root-pose geometry is constrained before first search expansion

Revision 2 searched from spawn and filtered afterward. Revision 3 instead asks the
pinned Tetrp authority helper to enumerate the complete geometry-only landing set
reachable from the ACTUAL active piece without Hold. That allowlist is transported
in the detached request and applied to Kiwi's first Place expansion before scoring.

The authority enumerator has no top-K cutoff. If its explicit state bound is
exceeded it rejects rather than silently truncating. The result reports the ranked
candidate index; the adapter's selection helper also reports the post-filter
candidate index. A Place candidate outside the root allowlist is a contract error.

x/y/rotation alone are insufficient. The snapshot now also transports/audits
`hy`, `kick`, `rotated`, `spin`, `totalRotations`, `resets`,
`rotationResets`, `locking`, `forceLock`, `safelock`, `softDropped`, and
`wall`. Geometry enumeration consumes the real Tetrp active-piece state, including
SRS+ kick/spin history. Exact input-timing executability additionally depends on
the complete authority state and handling/input timers, so it remains a separate
Tetrp-side validation boundary.

The package deliberately distinguishes:

- geometry reachability: used to constrain the first Kiwi Place layer;
- input timing/reset executability: optionally checked with
  `validateSnapshotTimingAction` on an isolated Tetrp clone.

Passing geometry does not claim a move can be executed at an arbitrary requested
lock frame. Never paint a board to substitute for authority execution.

Acceptance fixtures cover non-spawn positions, wall positions, rotated states,
near-lock states and spin-related states.

## Rules and stable rejection contract

The repaired Surge/public-rule/root-Hold-lock/clock behavior is retained. The
adapter exports `snapshotRuleContract()` and rejects unsupported mechanical rule
values instead of silently substituting defaults.

Important stable rejection codes include:

- `PENDING_ARE_QUEUE_UNSUPPORTED`: a positive existing garbage ARE queue;
- `PENDING_ACTIVATION_UNKNOWN`: observable pending packet lacks confirmed activation;
- `PENDING_PACKET_HARDENED_UNSUPPORTED`;
- `PENDING_PACKET_SHIELDED_UNSUPPORTED`;
- `PENDING_PACKET_STATUS_UNSUPPORTED`;
- `RULE_VALUE_UNSUPPORTED`;
- root geometry / timing / Hold-specific codes exposed by
  `normalizeSnapshotError()`.

Pending which has not entered ARE remains supported only with an explicit
`ready_in_frames`. Unknown activation is never replaced with zero or guessed.
Positive existing ARE is still rejected.

Pending forecasting remains approximate: explicit 24 frames/placement in the
product review regime, ten hypothetical clean-hole scenarios when incoming exists,
integer-frame clocking, and simplified ARE/bump behavior. Capability flags continue
to say full rules parity, full opener parity, full Clutch parity and exact ARE/bump
timing are false unless the final build's tests establish otherwise.

The rule differential suite expands opener pending/cumulative-sent/opener-boundary
coverage, Clutch spawn/clear rescue cases, topout observations, full-vs-partial
storage-top garbage-smash behavior, and pinned Tetrp public-rule variants. Supported
variable attack-rule values are compared field-for-field against Kiwi; a Tetrp-visible
b2bchaining=true fixture is required to remain an explicit Kiwi rejection. Limited fixtures must not be rewritten as a
claim of complete parity. Active-piece repair failure after garbage insertion and
complete terminal-reason classification remain outside the forecast model.

## Worker, determinism and disposal

Use `kiwi-snapshot-worker.mjs` as a dedicated module Worker. Do not run WASM on the
UI thread. A synchronous search cannot service a cancel message while executing;
cancel/exit by terminating the Worker. Repeated and restarted requests with the
same allowed input/search settings must be deterministic and independent of prior
analysis or replay history.

The original recorded checkpoint must remain unchanged. Any Hold/placement
validation happens on isolated Tetrp clones. Discard branch/request/result state on
exit. Static offline program caching is separate from analysis-history persistence.

## Replay compatibility: TL ARE rules and 40L stacking

`garbageare` and `garbagearebump` are no longer exact-zero gates. Their real
nonnegative public values are preserved in `timing_rules`, returned in the
analysis result, and covered by synthetic TL fixtures including
`garbageare=5` / `garbagearebump=12`. They are NOT silently rewritten to
zero.

This does not mean ARE/bump simulation became exact. `exact_are_bump_timing=false`
remains truthful. A positive ARE queue that already exists at the snapshot is a
different state from merely having nonzero public ARE rules; that current positive
ARE queue still rejects with `PENDING_ARE_QUEUE_UNSUPPORTED`. Unknown packet
activation remains an explicit rejection rather than zero-filling or guessing.

40L is accepted only through the separately labeled
`analysis_mode=competitive_stacking` with `source_mode=40l`. It is a
competitive board/placement heuristic using neutral root combo/B2B counters, no
pending garbage, and no TL authority attack clock. It is NOT presented as TL and
is NOT a 40L score/time optimizer. The evaluator may use its declared competitive
analysis rule context internally, but no solo attack/B2B history is fabricated.

## Package closure and post-upload E2E

`tetrp-placement-path.mjs` is self-contained in revision 3.1; it no longer
imports the obsolete history-oriented `tetrp-authority-adapter.mjs`.

Hash identity alone is insufficient. The release workflow now also checks relative
ESM import closure and, after GitHub uploads the artifact, downloads that artifact
onto a fresh job and executes this chain from the downloaded files:

`packaged placement helper -> pinned Tetrp Engine snapshot -> packaged snapshot adapter -> packaged web WASM -> analyze_snapshot_json`

That post-upload test includes TL with ARE rules 5/12 and 40L competitive stacking.
The same chain is also run before upload and its report is included as
`kiwi-package-e2e.json`. A release is not accepted until both the file/hash
gate and the downloaded-package E2E gate pass.

## Release verification

Consume only a successful `kiwi-v1-browser` whose `kiwi-build.json` says
`kiwi-v1-snapshot-v3.1` and whose post-upload archive verification and packaged E2E both succeeded.

The artifact includes web WASM/glue, snapshot adapter and Worker, placement helper,
capabilities through the WASM API, `kiwi-build.json`, `sha256.json`, native and
Node-WASM acceptance logs, Chromium/WebKit Worker evidence, rule differential
evidence, licenses/notices, this handoff and the Tetrp consumer task.

The workflow re-downloads the uploaded artifact and verifies the exact file set and
all SHA-256 hashes before ntfy completion notification. Exact final commit/run/
artifact IDs live in `kiwi-build.json` and the release receipt recorded after the
accepted workflow.

Do not infer strategy strength from this release. No H2/H9 promotion evidence is
created by snapshot-v3 correctness testing.
