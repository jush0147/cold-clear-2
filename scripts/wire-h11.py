from pathlib import Path

bot_path = Path('src/bot.rs')
freestyle_path = Path('src/bot/freestyle.rs')
dag_path = Path('src/dag.rs')
harness_src = Path('src/bin/strategy_h6c.rs')
harness_out = Path('src/bin/strategy_h11.rs')

# --- BotConfig: separate exploitation for known vs speculative layers ---
bot = bot_path.read_text()
old = '''pub struct BotConfig {
    pub freestyle_weights: freestyle::Weights,
    pub freestyle_exploitation: f64,
}
'''
new = '''pub struct BotConfig {
    pub freestyle_weights: freestyle::Weights,
    pub freestyle_exploitation: f64,
    #[serde(default = "default_freestyle_speculated_exploitation")]
    pub freestyle_speculated_exploitation: f64,
}
fn default_freestyle_speculated_exploitation() -> f64 {
    std::f64::consts::LN_2
}
'''
if old not in bot:
    raise SystemExit('BotConfig exploitation block not found')
bot = bot.replace(old, new, 1)
bot_path.write_text(bot)

# --- Freestyle: pass both values into DAG selection ---
freestyle = freestyle_path.read_text()
old = '''        if let Some(node) = self
            .dag
            .select(options.speculate, options.config.freestyle_exploitation)
        {
'''
new = '''        if let Some(node) = self.dag.select(
            options.speculate,
            options.config.freestyle_exploitation,
            options.config.freestyle_speculated_exploitation,
        ) {
'''
if old not in freestyle:
    raise SystemExit('Freestyle DAG select call not found')
freestyle = freestyle.replace(old, new, 1)
freestyle_path.write_text(freestyle)

# --- DAG: select by information regime, not one scalar for the whole tree ---
dag = dag_path.read_text()
old = '''    pub fn select(&self, speculate: bool, exploration: f64) -> Option<Selection<E>> {
        puffin::profile_function!();
        let mut layers = vec![&*self.top_layer];
        let mut game_state = self.root;
        loop {
            let &layer = layers.last().unwrap();

            match layer.kind.select(&game_state, speculate, exploration) {
'''
new = '''    pub fn select(
        &self,
        speculate: bool,
        known_exploration: f64,
        speculated_exploration: f64,
    ) -> Option<Selection<E>> {
        puffin::profile_function!();
        let mut layers = vec![&*self.top_layer];
        let mut game_state = self.root;
        loop {
            let &layer = layers.last().unwrap();

            match layer.kind.select(
                &game_state,
                speculate,
                known_exploration,
                speculated_exploration,
            ) {
'''
if old not in dag:
    raise SystemExit('Dag::select block not found')
dag = dag.replace(old, new, 1)

old = '''    fn select(&self, game_state: &GameState, speculate: bool, exploration: f64) -> SelectResult {
        puffin::profile_function!();
        self.with(|this| match this.data {
            LayerKind::Known(l) => l.select(game_state, exploration),
            LayerKind::Speculated(l) if speculate => l.select(game_state, exploration),
            LayerKind::Speculated(_) => SelectResult::Failed,
        })
    }
'''
new = '''    fn select(
        &self,
        game_state: &GameState,
        speculate: bool,
        known_exploration: f64,
        speculated_exploration: f64,
    ) -> SelectResult {
        puffin::profile_function!();
        self.with(|this| match this.data {
            LayerKind::Known(l) => l.select(game_state, known_exploration),
            LayerKind::Speculated(l) if speculate => {
                l.select(game_state, speculated_exploration)
            }
            LayerKind::Speculated(_) => SelectResult::Failed,
        })
    }
'''
if old not in dag:
    raise SystemExit('WithBump::select block not found')
dag = dag.replace(old, new, 1)
dag_path.write_text(dag)

# --- Harness: derive a two-regime A/B from the stable H6C duel harness ---
s = harness_src.read_text()
s = s.replace(
    '//! KO-only H6C A/B: frozen H2 incumbent versus row-transition shaping scale.',
    '//! KO-only H11 A/B: promoted H9 incumbent versus known/speculative search allocation.',
    1,
)

old = '''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { row_transition_scale: f32 }
impl Default for Strategy {
    fn default() -> Self { Self { row_transition_scale: 1.0 } }
}
'''
new = '''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy {
    known_exploitation: f64,
    speculated_exploitation: f64,
}
impl Default for Strategy {
    fn default() -> Self {
        Self {
            known_exploitation: std::f64::consts::LN_2,
            speculated_exploitation: std::f64::consts::LN_2,
        }
    }
}
'''
if old not in s:
    raise SystemExit('H6C Strategy block not found')
s = s.replace(old, new, 1)

old = '''            "--row-transition-scale" => s.strategy.row_transition_scale = p[1].parse().map_err(|_| "invalid row transition scale")?,
            "--incumbent-row-transition-scale" => s.incumbent.row_transition_scale = p[1].parse().map_err(|_| "invalid incumbent row transition scale")?,
'''
new = '''            "--known-exploitation" => s.strategy.known_exploitation = p[1].parse().map_err(|_| "invalid known exploitation")?,
            "--speculated-exploitation" => s.strategy.speculated_exploitation = p[1].parse().map_err(|_| "invalid speculated exploitation")?,
            "--incumbent-known-exploitation" => s.incumbent.known_exploitation = p[1].parse().map_err(|_| "invalid incumbent known exploitation")?,
            "--incumbent-speculated-exploitation" => s.incumbent.speculated_exploitation = p[1].parse().map_err(|_| "invalid incumbent speculated exploitation")?,
'''
if old not in s:
    raise SystemExit('H6C CLI block not found')
s = s.replace(old, new, 1)

old = '''    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.row_transition_scale.is_finite() || s.strategy.row_transition_scale < 0.0
        || !s.incumbent.row_transition_scale.is_finite() || s.incumbent.row_transition_scale < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative H6C row-transition scales required".into());
'''
new = '''    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.known_exploitation.is_finite() || s.strategy.known_exploitation <= 0.0
        || !s.strategy.speculated_exploitation.is_finite() || s.strategy.speculated_exploitation <= 0.0
        || !s.incumbent.known_exploitation.is_finite() || s.incumbent.known_exploitation <= 0.0
        || !s.incumbent.speculated_exploitation.is_finite() || s.incumbent.speculated_exploitation <= 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite positive H11 exploitation values required".into());
'''
if old not in s:
    raise SystemExit('H6C validation block not found')
s = s.replace(old, new, 1)

old = '''    // H1 + H2 winner are frozen for BOTH sides in H6C. Rejected H3/H4/H5/H6/H6B changes stay off.
    config.freestyle_weights.softdrop = 0.0;
    config.freestyle_weights.pending_safety = 1.0;
    config.freestyle_weights.useful_attack_reward = 1.0;
    config.freestyle_weights.cancellation_reward = 0.0;
    config.freestyle_weights.h3_b2b_charge_value = 0.0;
    config.freestyle_weights.h3_surge_bank_value = 0.0;
    // H6B is explicitly reset. Only the always-on row-transition term changes.
    config.freestyle_weights.h6_base_holes_scale = 1.0;
    config.freestyle_weights.h6_base_coveredness_scale = 1.0;
    config.freestyle_weights.row_transitions *= strategy.row_transition_scale;
'''
new = '''    // Freeze the promoted H9 evaluator on both sides. H11 changes only search
    // allocation between known current+Next5 layers and speculative bag layers.
    config.freestyle_weights.softdrop = 0.0;
    config.freestyle_weights.pending_safety = 1.0;
    config.freestyle_weights.useful_attack_reward = 1.0;
    config.freestyle_weights.cancellation_reward = 0.0;
    config.freestyle_weights.h3_b2b_charge_value = 0.0;
    config.freestyle_weights.h3_surge_bank_value = 0.0;
    config.freestyle_weights.h6_base_holes_scale = 1.0;
    config.freestyle_weights.h6_base_coveredness_scale = 1.0;
    config.freestyle_weights.row_transitions *= 2.5;
    config.freestyle_weights.h9_cavity_excavation = -0.5;
    config.freestyle_exploitation = strategy.known_exploitation;
    config.freestyle_speculated_exploitation = strategy.speculated_exploitation;
'''
if old not in s:
    raise SystemExit('H6C make_player block not found')
s = s.replace(old, new, 1)

old = '''        "row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
'''
new = '''        "known_exploitation": s.strategy.known_exploitation,
        "speculated_exploitation": s.strategy.speculated_exploitation,
        "incumbent_known_exploitation": s.incumbent.known_exploitation,
        "incumbent_speculated_exploitation": s.incumbent.speculated_exploitation,
        "fixed_row_transition_scale": 2.5,
        "fixed_h9_cavity_excavation": -0.5,
'''
if old not in s:
    raise SystemExit('H6C game metadata block not found')
s = s.replace(old, new, 1)

s = s.replace(
    '"experiment": "H6C row-transition shaping scale",',
    '"experiment": "H11 known/speculative search allocation",',
    1,
)
s = s.replace(
    '"incumbent": "H2 winner with legacy row-transition scale 1.0; H1/H2 frozen; rejected H3/H4/H5/H6/H6B disabled",',
    '"incumbent": "H9 promoted evaluator with default known/speculated exploitation ln(2)/ln(2)",',
    1,
)
old = '''        "candidate_row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
'''
new = '''        "candidate_known_exploitation": s.strategy.known_exploitation,
        "candidate_speculated_exploitation": s.strategy.speculated_exploitation,
        "incumbent_known_exploitation": s.incumbent.known_exploitation,
        "incumbent_speculated_exploitation": s.incumbent.speculated_exploitation,
        "fixed_row_transition_scale": 2.5,
        "fixed_useful_attack_reward": 1.0,
        "fixed_h9_cavity_excavation": -0.5,
'''
if old not in s:
    raise SystemExit('H6C protocol metadata block not found')
s = s.replace(old, new, 1)
s = s.replace('"H6C A/A paired game not repeatable"', '"H11 A/A paired game not repeatable"')

old = '''            vec!["--row-transition-scale", "NaN"],
            vec!["--row-transition-scale", "-0.1"],
            vec!["--incumbent-row-transition-scale", "NaN"],
            vec!["--incumbent-row-transition-scale", "-0.1"],
'''
new = '''            vec!["--known-exploitation", "NaN"],
            vec!["--known-exploitation", "0"],
            vec!["--speculated-exploitation", "NaN"],
            vec!["--speculated-exploitation", "0"],
            vec!["--incumbent-known-exploitation", "NaN"],
            vec!["--incumbent-speculated-exploitation", "0"],
'''
if old not in s:
    raise SystemExit('H6C parser rejection tests not found')
s = s.replace(old, new, 1)

old = '''    fn accepts_direct_h6c_vs_h6c_configuration() {
        let args = vec![
            "--row-transition-scale", "0.5",
            "--incumbent-row-transition-scale", "1.5",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { row_transition_scale: 0.5 });
        assert_eq!(s.incumbent, Strategy { row_transition_scale: 1.5 });
    }
'''
new = '''    fn accepts_direct_h11_configuration() {
        let args = vec![
            "--known-exploitation", "0.5108256237659907",
            "--speculated-exploitation", "0.916290731874155",
            "--incumbent-known-exploitation", "0.6931471805599453",
            "--incumbent-speculated-exploitation", "0.6931471805599453",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy {
            known_exploitation: 0.5108256237659907,
            speculated_exploitation: 0.916290731874155,
        });
        assert_eq!(s.incumbent, Strategy {
            known_exploitation: std::f64::consts::LN_2,
            speculated_exploitation: std::f64::consts::LN_2,
        });
    }
'''
if old not in s:
    raise SystemExit('H6C direct configuration test not found')
s = s.replace(old, new, 1)

required = [
    'config.freestyle_weights.pending_safety = 1.0;',
    'config.freestyle_weights.useful_attack_reward = 1.0;',
    'config.freestyle_weights.row_transitions *= 2.5;',
    'config.freestyle_weights.h9_cavity_excavation = -0.5;',
    'config.freestyle_exploitation = strategy.known_exploitation;',
    'config.freestyle_speculated_exploitation = strategy.speculated_exploitation;',
    'known_exploitation: f64',
    'speculated_exploitation: f64',
]
for needle in required:
    if needle not in s:
        raise SystemExit(f'missing H11 harness invariant: {needle}')
if 'strategy.row_transition_scale' in s or 'combo_attack *=' in s:
    raise SystemExit('H11 must freeze H9 evaluator and legacy combo scale')

harness_out.write_text(s)

# Guard the core changes.
for path, needles in {
    bot_path: ['pub freestyle_speculated_exploitation: f64'],
    freestyle_path: ['options.config.freestyle_speculated_exploitation'],
    dag_path: ['known_exploration: f64', 'speculated_exploration: f64'],
}.items():
    text = path.read_text()
    for needle in needles:
        if needle not in text:
            raise SystemExit(f'missing H11 core invariant in {path}: {needle}')
