# Repository continuation instructions

**Active strategy branch: `tetrp-authority`.** Treat `s2-strategy-clean` as legacy-arena history unless explicitly doing archaeology.

For any work on TETR.IO TL S2 strategy, KO experiments, replay review, Cold Clear search behavior, or browser/WASM integration:

1. **Read `docs/strategy-experiments.md` first.**
2. **Read `docs/review-bot-design.md` before touching review/WASM/product behavior.**
3. Check the relevant machine-readable record under `experiments/` before proposing the next experiment.
4. Check the latest GitHub Actions result before assuming a run is still pending.
5. Preserve the documented information boundary: current + hold + exactly NEXT 5, own state, observable incoming garbage, no hidden future sequence/opponent state.
6. Do not pool reruns of the same seed/configuration as independent evidence.
7. Correctness fixes and strength promotions are different categories. Do not require a correctness bug fix to win KO if the harness cannot observe that code path.
8. Native strength and WASM performance are not comparable until both use the same bot config and the same hard evaluator-node budget semantics.
9. H13 despeculation/backprop is now landed in branch core, but the scored `review_h9_h12()` profile intentionally leaves it disabled. Do not silently reinterpret historical H14 strength results as H13-enabled.

The repository documents are the continuity source of truth when chat context and summaries disagree.
10. Browser performance evidence lives in `experiments/wasm-browser-devices.json`. Prefer downloadable benchmark JSON over copied page output, and do not treat one run's device ordering as a pure CPU ranking.
11. H14 has a fresh 200k-vs-100k headroom probe on seeds 47500-47519; check `experiments/h14-review-compute-scaling-plan.json` and the latest Actions run before launching more compute tests.
12. After compute testing, follow the ordered roadmap in `docs/review-bot-design.md` under **Canonical product roadmap after compute testing**; do not skip directly from strength experiments to UI integration.

## 2026-09-19 authority migration gate

Historical H1-H14 strategy-strength results were produced by the legacy simplified **turn-based** KO arena. They remain useful evidence about that arena, but they are **not sufficient to claim strength under target TETR.IO TL S2 play**.

Before any new strategy-strength work:

1. Read `experiments/tetrp-authority-migration-plan.json`.
2. Treat the reverse-engineered deterministic Tetrp TL engine as the target match authority for board, RNG, attack, garbage timing, frame timing, and KO.
3. Do not launch new evaluator tuning on the legacy turn-based arena.
4. Fix and verify the visible-state adapter first, especially SevenBag state across bag boundaries.
5. Use a synchronous decision barrier: both bots choose from pre-commit snapshots before either new attack can affect the other's decision.
6. Keep current+hold+exactly NEXT5 and own observable garbage as the information boundary. Never expose hidden authority RNG, NEXT6+, opponent board, or future attack.
7. First run a whole-profile transfer test: current tuned H1+H2+H6C+H9+H12 vs corrected legacy+H12. Only then perform retained-feature ablations or reopen old rejected hypotheses.
8. Historical H3-H11 rejections mean **rejected under the legacy arena**, not globally disproven. Reopen B2B/Surge and combo ideas early after the new baseline is established.
9. Pace is part of match semantics because garbage timing and late-round scaling are frame-based. A 2 PPS scored match is not merely a visualization speed.
10. H12/H13 correctness fixes, hard-node plumbing, deterministic search, replay reconstruction, and browser latency evidence remain valid unless a separate bug is found.

