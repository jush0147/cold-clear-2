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

This matters for correctness work such as H13. H13 fixes the case where a newly revealed NEXT piece replaces a speculative bag-average layer and the corrected value must backpropagate through the persistent DAG. The KO snapshot harness rebuilds the DAG through `set_forecast()` before each scored decision, so H13 is not observable there.

**Repository state:** H13 was promoted into the branch core by run `35335097243` (commit `be0811c`) after both targeted regressions, full release library tests and a wasm32 check passed. The scored H9+H12 profile still leaves H13 disabled intentionally so H14 strength results remain historically comparable. The interactive product profile can enable it separately.

See `experiments/h13-despeculate-backprop-plan.json` for the exact H13 evidence and limitation.

## Current strategy / compute state

As of the H14 work:

- scored strategy lineage currently used by the compute harness: **H9 + H12**
- H12 is a correctness fix for stale best-child demotion/backprop
- H13 is a persistent-DAG correctness fix now present in branch core, but it remains separate from the scored H9+H12 H14 profile
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

## H14 bridge update

Fresh bridge run `35322969711` completed the adjacent-budget checks:

- 50k vs 25k, seeds 47300-47319: 26-14 games; paired sweeps 8-2 with 10 splits; paired exact sign p = 0.109375.
- 100k vs 50k, seeds 47400-47419: 24-16 games; paired sweeps 6-2 with 12 splits; paired exact sign p = 0.2890625.

Both comparisons still point toward more compute, but the adjacent-budget effects are much weaker than the earlier fresh 100k-vs-25k result (30-10; sweeps 11-1; p = 0.00634765625). This is **consistent with diminishing returns**, not proof that 50k is the exact strength knee.

Do not spend another large native KO batch merely to force significance before measuring browser cost. The H9+H12 config and hard-node API are now shared; the active next step is to validate the WASM build/benchmark, then measure 25k/50k/100k on representative desktop browsers before choosing a product budget.

## Native/WASM parity requirement

Before using browser timing to choose a production budget, native H14 and the WASM review path must represent the **same scored configuration** and the **same compute unit**.

Current branch state:

- `BotConfig::review_h9_h12()` is the canonical scored H14 H9+H12 configuration.
- `BotConfig::interactive_review()` adds H13 persistent-DAG correctness without changing the scored profile.
- the H14 generator uses `review_h9_h12()` rather than duplicating evaluator constants.
- persistent `WasmBot` uses `interactive_review()`.
- `WasmBot::think_nodes()` uses the same hard evaluator-node budget unit as H14.
- pending `analysis.rs` intentionally stays on `review_h9_h12()` because scenario DAGs are rebuilt and H13 persistence is not observable there.
- the browser/Node benchmark workload uses SevenBag speculation rather than `Randomizer::Unknown`.

H14's scored configuration includes:

- pending safety = 1.0
- useful attack reward = 1.0
- row-transition scale = 2.5 relative to legacy
- H9 cavity excavation = -0.5
- H12 `dag_backprop_best_demotion = true`

### Remaining semantic differences

Config/budget parity does **not** mean every review path is now byte-for-byte the H14 KO harness.

- H14's KO `choose()` uses `Forecast::snapshot`; pending browser analysis retains its explicit pace/delay forecast model.
- `WasmBot` only performs SevenBag speculation if the supplied `Start.randomizer` is `SevenBag`. `Randomizer::Unknown` deliberately disables speculation beyond visible preview.
- replay integration must reconstruct `bag_state` only from already-observed piece history. Supplying hidden future randomizer state would violate the information boundary.
- pending analysis still rebuilds separate scenario bots; the no-pending persistent `WasmBot` path is the relevant baseline for measuring DAG reuse.

Do not compare browser timings to H14 strength unless the tested workload reports its exact profile, hard node budget, and randomizer/speculation mode. Fresh first-move timing under the H13 interactive profile is comparable as a compute-cost measurement because H13 has not fired yet; persistent continuation timing is a separate product-path measurement.

## Initial WASM performance baseline

WASM CI run `35334313492` completed the first hard-node baseline using the
canonical `h9+h12-review` profile. The workload was a fresh empty-board
snapshot with SevenBag speculation enabled and no pending garbage.

| Node budget | Median Node-WASM time | Median nodes/s |
|---:|---:|---:|
| 25k | 26.45 ms | 0.95M |
| 50k | 48.40 ms | 1.03M |
| 100k | 97.33 ms | 1.03M |

The 1003-node smoke test also returned exactly 1003 evaluated nodes, confirming
the hard-budget boundary through the built WASM package.

These are **GitHub runner Node-WASM measurements**, not desktop Chrome, laptop,
or mobile numbers. They are useful mainly because they show that 100k is not
obviously too expensive. A later CI run varied downward in absolute throughput,
which is another reason not to treat GitHub-runner milliseconds as device truth.

The pending-garbage benchmark in run `35334761106` held the **total** node
budget fixed while splitting search across ten hidden-hole scenarios:

| Node budget | Fresh/no pending | Active 8-line pending | Pending overhead |
|---:|---:|---:|---:|
| 25k | 35.23 ms | 42.10 ms | +19.5% |
| 50k | 64.64 ms | 78.87 ms | +22.0% |
| 100k | 131.97 ms | 150.67 ms | +14.2% |

So ten scenarios did not multiply latency by ten; on this run their orchestration
and reduced per-scenario search efficiency added roughly 14-22% over the
same-run fresh path. This is encouraging for the product design, but real
browser/device measurements remain the product decision evidence.

Machine-readable details live in
[`experiments/wasm-review-benchmark.json`](../experiments/wasm-review-benchmark.json).

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
