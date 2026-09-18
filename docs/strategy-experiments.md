# TL S2 strategy experiment framework

> **Read this first before changing bot strategy on `s2-strategy-clean`.**
>
> This is the canonical document for the current Cold Clear 2 strategy-optimization work. The goal is not to build a replay product. The goal is to make the bot stronger for TETR.IO Tetra League Season 2 and prove or reject strategy changes with direct KO-only bot-vs-bot matches.

## Continuation checkpoint: compute regime and current lineage

> **Important update:** the historical strategy experiments below were primarily run at a 10k evaluator-node budget. H14 later demonstrated that 10k is compute-starved for the review-style search harness. Therefore strategy promotions discovered under 10k should be read as **10k-regime results**, not automatically as compute-invariant global rankings.

Current scored lineage entering H14 is **H9 + H12**:

- H6C promoted a 2.5x row-transition scale under the historical 10k regime.
- H9 promoted a light sealed-cavity excavation penalty under that regime.
- H12 is a correctness fix for stale best-child demotion/backprop and is enabled in the corrected core.
- H13 fixes despeculation/backprop when a speculative layer becomes a known NEXT piece. Its targeted regression is valid, but the current snapshot KO harness rebuilds the DAG via `set_forecast()` before each scored search, so H13 is not observable as a KO strength change there.

H14 changed only per-move compute while freezing H9 + H12. Important results:

- 25k vs 10k fresh validation: 29-11 games; paired sweeps 9-0 with 11 splits; exact paired sign p = 0.00390625.
- 100k vs 25k fresh headroom: 30-10 games; paired sweeps 11-1 with 8 splits; exact paired sign p = 0.00634765625.

Thus both 10k and 25k remain below the observed native strength ceiling. The active bridge maps 25k -> 50k -> 100k before any final review budget is chosen.

This discovery does **not** invalidate earlier fair A/B comparisons at 10k; it limits their scope. After a practical final review compute regime is selected, revalidate the promoted strategy lineage at that regime before treating H1/H2/H6C/H9 ordering as final.

For intended replay-review/WASM behavior, persistent sessions, H13 product relevance, config parity, and browser performance rules, read [review-bot-design.md](review-bot-design.md).

## Objective

Compare two strategies on the same shared TL S2 rules implementation:

- **Baseline:** the original Cold Clear 2 strategy/evaluator behavior, adapted only where necessary so both sides run under the same shared rules, information boundary and experiment harness.
- **Candidate:** a TL S2-oriented strategy derived from that baseline.

A strategy change counts as progress only when direct KO match results support it. APP, total attack, average survival length, evaluator score and similar internal metrics may be diagnostic, but they do not override actual match results.

## Scope guard

Before modifying code, ask:

> **Does this change directly help Candidate become stronger in the KO experiment below, or make that comparison more correct?**

If the answer is not clearly yes, do not implement it as part of this work. Record unrelated ideas briefly and leave them alone.

### In scope

1. Fix bugs that would invalidate actual bot-vs-bot play or its comparison.
2. Correctly feed existing TL S2 state such as pending garbage into search and the duel authority.
3. Modify evaluator weights, strategic features or search/exploration policy.
4. Run Baseline vs Candidate to KO.
5. Keep, modify or revert changes according to match evidence.
6. Add only the regression tests and benchmark instrumentation required for the above.

### Out of scope unless explicitly requested

- review UI or replay-viewer features
- Web Worker UI features
- grading a player's recorded move
- mistake/blunder labels
- explanation systems
- 1-5 ply review wrappers or other arbitrary finite-horizon review systems
- opponent-board analysis
- new product features
- architecture rewrites justified only by possible future usefulness

## Information boundary

At each decision the bot may use only information a real player could know about their own state at that moment:

- own board
- current piece
- actual hold
- exactly five visible NEXT pieces
- combo state
- B2B / B2B count / Surge state
- currently visible incoming garbage
- necessary history derivable from the bot's own previous observations/actions

The bot must **not** receive:

- the true sixth-or-later NEXT piece before it becomes visible
- future replay piece sequence
- hidden RNG state
- opponent future attacks
- opponent board
- hidden garbage data the player cannot yet observe

After the known preview is exhausted, normal seven-bag probability / Cold Clear speculation may continue search. It must not read the simulator's true future sequence. There is no fixed five-move search cutoff.

## Shared KO protocol

Unless an experiment explicitly records a justified protocol revision, Baseline and Candidate must share all of the following:

- the same TL S2 rules implementation
- the same piece seed
- the same search resource / evaluator-node budget per move
- zero gravity and no placement-speed or soft-drop-time strategy cost
- the same visible-state adapter
- the same garbage authority and cancellation rules
- the same uncertainty treatment for information that is still hidden

Every piece seed is played **twice**, swapping the sides assigned to Baseline and Candidate.

The only scored outcome is **KO**.

There is deliberately:

- no piece cap
- no APP tiebreak
- no attack tiebreak
- no cumulative-score tiebreak
- no artificial winner on timeout, invalid search state or harness error

Errors and incomplete jobs are experiment failures/incomplete samples, not wins or draws.

## Experiment workflow

Use a sequential champion/incumbent process. Do not combine many dependent strategy changes and then guess which one caused the result.

1. **Freeze the current comparison version.** Record its commit/configuration.
2. **State one strategy hypothesis**, or one small tightly related group of changes.
3. **Add/execute required regressions.** Do not expand unrelated rule/tool work.
4. **If the harness changed, run an A/A control** to verify side swapping and deterministic/reproducible behavior as far as the harness promises.
5. **Screen candidate parameterizations** using identical seed sets.
6. **Retest survivors on fresh seeds.** Seeds used to select a parameter should not also be treated as independent final confirmation evidence.
7. **Confirm the fixed winner on fresh/holdout seeds.** Do not keep changing coefficients after looking at confirmation results and pretend it was one preplanned test.
8. **Retain, revert or leave experimental** based primarily on paired KO results.

A failed hypothesis is a useful result. Do not rescue a losing Candidate by pointing to APP, attack totals or prettier internal scores.

## Parallelism on GitHub Actions

Parallelize **within a hypothesis**, not across dependent hypotheses.

Good:

```text
current incumbent
├─ H2 candidate A
├─ H2 candidate B
├─ H2 candidate C
└─ H2 candidate D
```

All candidates in the same stage should use the same seeds and node budget. GitHub Actions matrix jobs can split candidates and/or seed batches while using one frozen executable/source revision.

Prefer a practical staged search such as random/quasi-random candidate sampling plus successive halving:

1. many candidates, small shared seed set
2. discard clearly weak candidates
3. fewer candidates, larger fresh seed set
4. fixed winner vs incumbent on fresh seeds
5. optional final fixed winner vs original Baseline on untouched holdout seeds

Exact candidate counts and game counts are experiment parameters, not permanent laws. Record the plan before inspecting its results. If the plan changes after results are seen, record that fact.

Do not make each match its own GitHub job; VM/bootstrap overhead is pointless. A matrix cell should normally run a useful batch of paired seeds.

## Current status

### Baseline

The comparison Baseline is legacy Cold Clear 2 strategy/evaluator behavior running under the **same shared S2 core and experiment policy** as Candidate. It is not a claim that the entire binary is byte-for-byte untouched upstream CC2.

### H1 - pending-garbage board safety

**Status: tested; retain as an experimental incumbent/candidate, not promoted as conclusively superior.**

Hypothesis: while currently observed garbage remains pending, penalize unsafe board structure more strongly, specifically holes, covered depth and dangerous height. Cancellation naturally reduces that extra pressure term.

The tested Candidate changed `pending_safety` from `0.0` to `1.0` while keeping the shared zero-gravity policy and otherwise using legacy strategy weights.

Scored KO results:

| Batch | Seeds | Games | H1 wins | Baseline wins |
|---|---:|---:|---:|---:|
| Pilot | 100-119 | 40 | 24 | 16 |
| Validation | 1000-1039 | 80 | 44 | 36 |
| Confirmation | 2000-2099 | 200 | 112 | 88 |
| **Total** | 160 paired seeds | **320** | **180** | **140** |

Overall H1 win rate in these scored games was **56.25%**. All three disjoint batches favored H1, but the fixed 200-game confirmation alone remained statistically uncertain (`paired two-sided sign p ~= 0.0884`). Therefore H1 is useful as the current experimental incumbent for the next hypothesis, but it is not declared proven/default.

The exact H1 protocol, commits, CI runs, caveats and reproduction commands are recorded in [`experiments/h1-pending-safety.json`](../experiments/h1-pending-safety.json). That experiment record is authoritative for H1 numbers.

### H2 - useful S2 attack and cancellation value

**Status: proposed, not yet an experimental result.**

Working hypothesis: once H1 is held fixed, tune the strategic value of **actual useful outgoing attack** and **actual cancellation of visible incoming garbage** rather than relying only on legacy clear-type rewards as proxies.

Expected experiment shape:

- incumbent: H1 (`pending_safety = 1.0`)
- candidates: H1 plus different attack/cancellation-value parameterizations
- same rules, observations, paired seeds and node budget
- shared seed sets for screening candidates
- fresh seeds for later validation/confirmation

Do not record H2 as an improvement until KO results exist.

## Candidate research backlog

These are research directions, **not implemented commitments and not a fixed ordering**:

- board safety, holes and covered cells
- height danger
- attack vs survival tradeoff
- cancellation under garbage pressure
- B2B Charging
- Surge building/release timing
- T-Spin / Quad setup value
- T-piece usage/waste
- combo value and downstack tradeoffs
- search/exploration policy

Only promote one of these into an active hypothesis when the previous experiment leaves a clear incumbent/comparison point.

## Per-experiment record

Every completed or abandoned strategy experiment should leave a compact machine- or human-readable record under `experiments/` containing at least:

```text
experiment / status
hypothesis
baseline/incumbent revision + configuration
candidate revision + configuration
single intended variable or tightly related change group
shared protocol and node budget
seed ranges and paired-game counts
Candidate wins / comparison wins
aborted or incomplete games
whether seeds were screening, validation or holdout
post-hoc changes to the sampling plan
keep / revert / experimental decision
major correctness limitations discovered
reproduction command(s)
```

Do not silently pool repeated runs of the same seed/configuration as independent evidence.

## Relationship to the older correctness audit

[`docs/s2-audit.md`](s2-audit.md) is an older correctness-audit snapshot and contains statements that were true before later pending-aware/search work. It remains useful as history and as a list of rule-conformance cautions, but it is **not** the canonical current strategy status document.

If a newly discovered rule bug would invalidate the KO comparison, fix it for both sides and document the protocol change. Otherwise, do not abandon strategy experiments to expand the rules/tooling surface merely because another feature might someday be useful.
