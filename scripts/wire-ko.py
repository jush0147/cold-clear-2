from pathlib import Path

def replace(path, old, new, count=1):
    p = Path(path)
    text = p.read_text()
    assert text.count(old) == count, (path, old[:80], text.count(old))
    p.write_text(text.replace(old, new))

replace('src/lib.rs', 'pub mod forecast;', 'pub mod forecast;\npub mod ko_support;')
replace('src/bot.rs', 'fn do_work(&self, options: &BotOptions) -> Statistics;', 'fn do_work(&self, options: &BotOptions, budget: u64) -> Statistics;')
replace('src/bot.rs', 'self.mode.do_work(&self.options)', 'self.mode.do_work(&self.options, u64::MAX)')
replace('src/bot.rs', '    /// Search state.', '''    /// Hard evaluator-node allocation, shared by both sides of a KO experiment.
    pub fn do_work_limited(&self, budget: u64) -> Statistics {
        self.mode.do_work(&self.options, budget)
    }
    /// Search state.''')
replace('src/bot.rs', 'pub struct Statistics { pub nodes: u64, pub selections: u64, pub expansions: u64 }', 'pub struct Statistics { pub nodes: u64, pub selections: u64, pub expansions: u64, pub max_depth: usize, pub speculative_expansions: u64, pub budget_exhausted: bool }')
replace('src/bot.rs', '        self.expansions += other.expansions;', '''        self.expansions += other.expansions;
        self.max_depth = self.max_depth.max(other.max_depth);
        self.speculative_expansions += other.speculative_expansions;
        self.budget_exhausted |= other.budget_exhausted;''')
replace('src/bot/freestyle.rs', 'fn do_work(&self, options: &BotOptions) -> Statistics {', 'fn do_work(&self, options: &BotOptions, budget: u64) -> Statistics {')
replace('src/bot/freestyle.rs', '            let (state, next) = node.state();', '''            new_stats.max_depth = node.depth();
            let (state, next) = node.state();
            new_stats.speculative_expansions = u64::from(next.is_none());''')
replace('src/bot/freestyle.rs', '                    for &(mv, sd_distance) in moves {\n                        let mut state = state;', '''                    for &(mv, sd_distance) in moves {
                        if new_stats.nodes == budget {
                            // Charge evaluated nodes, but never publish a partially
                            // enumerated action set as a completed expansion.
                            node.cancel();
                            new_stats.budget_exhausted = true;
                            return new_stats;
                        }
                        new_stats.nodes += 1;
                        let mut state = state;''')
replace('src/bot/freestyle.rs', '                    new_stats.nodes += children[next].len() as u64;', '')
replace('src/bot/freestyle.rs', '    pub cell_coveredness: f32,', '''    /// Experimental H1 multiplier; zero preserves legacy evaluation.
    #[serde(default)]
    pub pending_safety: f32,
    pub cell_coveredness: f32,''')
replace('src/bot/freestyle.rs', '    let legacy_shape = legacy_clear_reward(weights, info);', '''    // H1: unsafe structure costs more while observed garbage remains pending.
    // Use the real board, before optimistic T-slot cutouts. Cancellation
    // lowers the remaining pressure automatically.
    if weights.pending_safety != 0.0 {
        let (height, holes, covered) = crate::ko_support::board_danger(&state.board, weights.max_cell_covered_height);
        let pressure = state.forecast.remaining().min(16) as f32 / 8.0;
        eval += weights.pending_safety * pressure * (
            weights.holes * holes as f32 + weights.cell_coveredness * covered as f32
            + weights.height_upper_half * height.saturating_sub(10) as f32
            + weights.height_upper_quarter * height.saturating_sub(15) as f32);
    }
    let legacy_shape = legacy_clear_reward(weights, info);''')
replace('src/dag.rs', "impl<E: Evaluation> Selection<'_, E> {", '''impl<E: Evaluation> Selection<'_, E> {
    pub fn depth(&self) -> usize { self.layers.len() }
    pub fn cancel(self) {
        use std::sync::atomic::Ordering;
        self.layers.last().unwrap().kind.with(|this| match this.data {
            LayerKind::Known(l) => l.states.get(&self.game_state).unwrap().expanding.store(false, Ordering::Relaxed),
            LayerKind::Speculated(l) => l.states.get(&self.game_state).unwrap().expanding.store(false, Ordering::Relaxed),
        });
    }
''')
for path in ['src/dag/known.rs', 'src/dag/speculated.rs']:
    replace(path, 'let s: f64 = thread_rng().gen();', 'let s: f64 = crate::ko_support::random_unit();')
    replace(path, 'use rand::prelude::*;\n', '')
replace('src/dag/speculated.rs', 'thread_rng().gen_range(0..game_state.bag.len())', 'crate::ko_support::random_index(game_state.bag.len())')
replace('src/movegen.rs', '    locks.extend(underground_locks.into_iter());\n    locks', '''    locks.extend(underground_locks.into_iter());
    locks.sort_by_key(|(m, sd)| (m.location.piece as u8, m.location.x, m.location.y, m.location.rotation as u8, m.spin as u8, *sd));
    locks.dedup_by_key(|(m, _)| *m);
    locks''')
replace('src/forecast.rs', 'impl Forecast {', '''impl Forecast {
    /// Zero-gravity snapshot; no PPS-derived future activation timestamps.
    pub fn snapshot(packets: &[GarbagePacket], pieces: u32, sent: u32, scenario: u32) -> Result<Self, String> {
        let mut f = Self::new(packets, pieces, sent, 1, 600, scenario)?;
        f.frames_per_piece = 0;
        Ok(f)
    }
''')
replace('src/forecast.rs', '''            let bonus=if self.pieces_placed<14 && self.remaining()>=self.sent {attack} else {0};
            // Ordinary attack is consumed before opening-phase extra cancel;
            // the extra cancel is never sent to an opponent.
            let ordinary=self.consume(attack);
            self.consume(bonus);
            self.sent=self.sent.saturating_add(attack-ordinary);''', '''            let (cancelled, outgoing) = crate::ko_support::cancel_plan(attack, self.remaining(), self.pieces_placed, self.sent);
            self.consume(cancelled);
            self.sent=self.sent.saturating_add(outgoing);''')
