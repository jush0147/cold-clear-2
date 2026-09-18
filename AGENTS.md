# Repository continuation instructions

For any work on TETR.IO TL S2 strategy, KO experiments, replay review, Cold Clear search behavior, or browser/WASM integration:

1. **Read `docs/strategy-experiments.md` first.**
2. **Read `docs/review-bot-design.md` before touching review/WASM/product behavior.**
3. Check the relevant machine-readable record under `experiments/` before proposing the next experiment.
4. Check the latest GitHub Actions result before assuming a run is still pending.
5. Preserve the documented information boundary: current + hold + exactly NEXT 5, own state, observable incoming garbage, no hidden future sequence/opponent state.
6. Do not pool reruns of the same seed/configuration as independent evidence.
7. Correctness fixes and strength promotions are different categories. Do not require a correctness bug fix to win KO if the harness cannot observe that code path.
8. Native strength and WASM performance are not comparable until both use the same bot config and the same hard evaluator-node budget semantics.
9. H13 despeculation/backprop is validated in the experiment patch but is not yet landed in the current branch core; check actual core code before assuming persistent review has H13.

The repository documents are the continuity source of truth when chat context and summaries disagree.
