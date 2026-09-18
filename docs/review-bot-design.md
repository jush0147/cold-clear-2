# Replay review bot design and continuity notes

> **Read this before changing browser/WASM review behavior or connecting strategy experiments to the replay UI.**
>
> This document records product intent and architecture decisions that are easy to lose in chat context. It is not a substitute for the KO experiment protocol in [strategy-experiments.md](strategy-experiments.md).

## Product intent

The review feature is **interactive, not a full-replay automatic grader**.

Expected flow:

1. The user watches a replay normally.
2. At an interesting snapshot, the user asks the bot for an alternative continuation.
3. A bot session starts from exactly that visible replay state.
4. The bot proposes/plays **one next piece per user action**.
5. The user may continue the bot line for as many pieces as they want.

The primary target is a **desktop browser**. Mobile browser support is desirable, but it should not force the desktop review engine down to the weakest-device budget.

This is analysis, not live play. Hundreds of milliseconds of latency for a high-quality move can be acceptable if the strength gain is real. Do not optimize for 60 FPS or real-time placement latency unless product requirements change.

## Persistent session matters

After the first snapshot search, continuing the bot line should preserve the same bot/search session when possible:

```text
replay snapshot
  -> construct bot
  -> search
  -> suggest/play one move
  -> reveal the newly visible NEXT piece(s)
  -> keep the DAG
  -> search again
  -> next move
```

Do not rebuild from the replay snapshot after every bot move without a specific reason.

This matters for correctness work such as H13. H13 fixes the case where a newly revealed NEXT piece replaces a speculative bag-average layer and the corrected value must backpropagate through the persistent DAG. The KO snapshot harness currently rebuilds the DAG through `set_forecast()` before each scored decision, so H13 is not observable there. That **does not** mean H13 is irrelevant to the intended interactive product path.

See `experiments/h13-despeculate-backprop-plan.json` for the exact H13 evidence and limitation.

## Current strategy / compute state

As of the H14 work:

- scored strategy lineage currently used by the compute harness: **H9 + H12**
- H12 is a correctness fix for stale best-child demotion/backprop
- H13 is a latent persistent-DAG correctness fix, not a scored KO promotion
- the old 10k-node regime is now known to be compute-starved for review-style search

H14 native results:

- screen, seeds 47000-47009:
  - 25k vs 10k: 19-1; paired sweeps 9-0, 1 split
  - 50k vs 10k: 18-2; paired sweeps 8-0, 2 split
  - 100k vs 10k: 19-1; paired sweeps 9-0, 1 split
- fresh confirmation, seeds 47100-47119:
  - 25k vs 10k: 29-11; paired sweeps 9-0, 11 split; paired exact sign p = 0.00390625
- fresh headroom test, seeds 47200-47219:
  - 100k vs 25k: 30-10; paired sweeps 11-1, 8 split; paired exact sign p = 0.00634765625

Therefore:

- 10k is not an acceptable assumption for final review strength
- 25k is also not the native strength knee
- native strength still scales materially between 25k and 100k
- 50k-vs-25k and 100k-vs-50k bridge tests are the next mapping step
- do **not** select a browser default budget from native strength alone

The accidental rerun `35317697118` repeated the same 47000-47009 screen and produced identical traces. It is useful determinism evidence only and must **not** be pooled as independent strength evidence.

## Native/WASM parity requirement

Before using browser timing to choose a production budget, native H14 and the WASM review path must represent the **same bot** and the **same budget unit**.

At the time this note was written, they do not.

### Config mismatch

H14 freezes the promoted H9 evaluator plus H12 corrected DAG, including:

- pending safety = 1.0
- useful attack reward = 1.0
- row-transition scale = 2.5 relative to legacy
- H9 cavity excavation = -0.5
- H12 `dag_backprop_best_demotion = true`

The current WASM `analysis.rs` constructs `BotConfig::default()`. Do not assume that is equivalent to the H14 configuration. In particular, the current default configuration does not encode the H9/H6C experimental values used by H14.

The long-term fix should be one canonical review/strategy configuration shared by native strength experiments and WASM review code, rather than duplicated constants.

### Budget mismatch

H14 uses a hard **evaluator-node budget per move** through limited search.

The current WASM pending-analysis API accepts `iterations` and repeatedly calls `do_work()`. An iteration count is **not** the same quantity as H14's evaluator-node budget.

Do not compare "25k" native results with "25k iterations" in WASM as if they were equivalent.

The browser review API should expose or internally use the same hard node-budget semantics before latency/strength trade-offs are measured.

## Performance decision model

The final product budget should be chosen using two curves:

```text
evaluator nodes -> bot strength
evaluator nodes -> browser latency / sustained device cost
```

Measure the second curve with the exact same config and node semantics used by the first.

Useful device measurements:

- first suggestion from a fresh snapshot
- second/third/etc. suggestion from a persistent bot session
- no-pending-garbage case
- pending-garbage case with the ten hidden-hole scenarios
- 25k / 50k / 100k, and higher only if strength experiments justify it
- sustained several-step use, not only a single cold call
- desktop browser first; representative mobile browser second

Record at least wall-clock milliseconds and actual nodes searched. Nodes/sec is more useful than CPU marketing labels.

## Product implications if high compute keeps scaling

Do not assume every user action must use one universal budget.

Possible product policies include:

- a desktop-oriented default budget
- a lower mobile default if measured latency requires it
- an optional deeper-analysis mode
- progressive search that shows an early answer and continues refining
- adaptive extra compute for ambiguous/high-pressure positions

These are product choices. First establish native strength scaling and exact browser cost.

## Continuity rule

When resuming work on this project after chat/context loss:

1. read [strategy-experiments.md](strategy-experiments.md)
2. read this file
3. inspect the latest experiment JSON under `experiments/`
4. inspect the latest relevant GitHub Actions run before assuming an experiment is still pending
5. never treat a repeated run of identical seeds/config as fresh evidence
