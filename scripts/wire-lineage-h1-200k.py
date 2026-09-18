from pathlib import Path

src = Path("src/bin/strategy_h2.rs")
out = Path("src/bin/strategy_lineage_h1.rs")
s = src.read_text()

s = s.replace(
    "//! KO-only H2 A/B: H1 incumbent versus useful outgoing/cancellation rewards.",
    "//! 200k lineage revalidation: H1 pending safety versus corrected legacy baseline.",
    1,
)

s = s.replace(
    "#[derive(Clone, Copy, Debug, Default)]\nstruct Strategy { attack: f32, cancel: f32 }",
    "#[derive(Clone, Copy, Debug, Default, PartialEq)]\nstruct Strategy { pending_safety: f32 }",
    1,
)

old_settings = '''        match p[0].as_str() {
            "--seeds" => s.seeds = p[1].parse().map_err(|_| "invalid seeds")?,
            "--start" => s.start = p[1].parse().map_err(|_| "invalid start")?,
            "--nodes" => s.nodes = p[1].parse().map_err(|_| "invalid nodes")?,
            "--attack" => s.strategy.attack = p[1].parse().map_err(|_| "invalid attack reward")?,
            "--cancel" => s.strategy.cancel = p[1].parse().map_err(|_| "invalid cancellation reward")?,
            "--incumbent-attack" => s.incumbent.attack = p[1].parse().map_err(|_| "invalid incumbent attack reward")?,
            "--incumbent-cancel" => s.incumbent.cancel = p[1].parse().map_err(|_| "invalid incumbent cancellation reward")?,
            _ => return Err(format!("unknown flag {}; no piece cap or tiebreak", p[0])),
        }
    }
    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.attack.is_finite() || s.strategy.attack < 0.0
        || !s.strategy.cancel.is_finite() || s.strategy.cancel < 0.0
        || !s.incumbent.attack.is_finite() || s.incumbent.attack < 0.0
        || !s.incumbent.cancel.is_finite() || s.incumbent.cancel < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative H2 rewards required".into());
    }'''
new_settings = '''        match p[0].as_str() {
            "--seeds" => s.seeds = p[1].parse().map_err(|_| "invalid seeds")?,
            "--start" => s.start = p[1].parse().map_err(|_| "invalid start")?,
            "--nodes" => s.nodes = p[1].parse().map_err(|_| "invalid nodes")?,
            "--pending-safety" => s.strategy.pending_safety = p[1].parse().map_err(|_| "invalid pending safety")?,
            "--incumbent-pending-safety" => s.incumbent.pending_safety = p[1].parse().map_err(|_| "invalid incumbent pending safety")?,
            _ => return Err(format!("unknown flag {}; no piece cap or tiebreak", p[0])),
        }
    }
    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.pending_safety.is_finite() || s.strategy.pending_safety < 0.0
        || !s.incumbent.pending_safety.is_finite() || s.incumbent.pending_safety < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative pending-safety weights required".into());
    }'''
if old_settings not in s:
    raise SystemExit("H1 settings anchor not found")
s = s.replace(old_settings, new_settings, 1)

old_config = '''    // H1 is the frozen incumbent for BOTH sides in H2.
    config.freestyle_weights.softdrop = 0.0;
    config.freestyle_weights.pending_safety = 1.0;
    config.freestyle_weights.useful_attack_reward = strategy.attack;
    config.freestyle_weights.cancellation_reward = strategy.cancel;'''
new_config = '''    // Isolate H1 on the corrected current core. H2/H6C/H9 remain off on both sides.
    config.freestyle_weights.softdrop = 0.0;
    config.freestyle_weights.pending_safety = strategy.pending_safety;
    config.freestyle_weights.useful_attack_reward = 0.0;
    config.freestyle_weights.cancellation_reward = 0.0;
    config.freestyle_weights.h3_b2b_charge_value = 0.0;
    config.freestyle_weights.h3_surge_bank_value = 0.0;
    config.freestyle_weights.h6_base_holes_scale = 1.0;
    config.freestyle_weights.h6_base_coveredness_scale = 1.0;
    config.freestyle_weights.h9_cavity_excavation = 0.0;
    config.dag_backprop_best_demotion = true;
    config.dag_backprop_despeculated_values = false;'''
if old_config not in s:
    raise SystemExit("H1 config anchor not found")
s = s.replace(old_config, new_config, 1)

old_game = '''        "attack_reward": s.strategy.attack,
        "cancellation_reward": s.strategy.cancel,
        "incumbent_attack_reward": s.incumbent.attack,
        "incumbent_cancellation_reward": s.incumbent.cancel,''';
new_game = '''        "pending_safety": s.strategy.pending_safety,
        "incumbent_pending_safety": s.incumbent.pending_safety,''';
if old_game not in s:
    raise SystemExit("H1 game metadata anchor not found")
s = s.replace(old_game, new_game, 1)

old_protocol = '''        "experiment": "H2 useful outgoing attack and cancellation",
        "incumbent": "H1 pending_safety=1.0 plus configured H2 rewards",
        "candidate_attack_reward": s.strategy.attack,
        "candidate_cancellation_reward": s.strategy.cancel,
        "incumbent_attack_reward": s.incumbent.attack,
        "incumbent_cancellation_reward": s.incumbent.cancel,''';
new_protocol = '''        "experiment": "200k lineage revalidation H1 vs corrected legacy",
        "candidate": "pending_safety=1.0; H2/H6C/H9 off; H12 on",
        "incumbent": "pending_safety=0.0; H2/H6C/H9 off; H12 on",
        "candidate_pending_safety": s.strategy.pending_safety,
        "incumbent_pending_safety": s.incumbent.pending_safety,
        "h12_best_child_demotion": true,
        "h13_despeculation_backprop": false,''';
if old_protocol not in s:
    raise SystemExit("H1 protocol anchor not found")
s = s.replace(old_protocol, new_protocol, 1)

s = s.replace(
    "if s.strategy.attack == s.incumbent.attack && s.strategy.cancel == s.incumbent.cancel {",
    "if s.strategy == s.incumbent {",
    1,
)
s = s.replace(
    'return Err("H1/H1 paired game not repeatable".into());',
    'return Err("200k H1 lineage A/A paired game not repeatable".into());',
    1,
)

test_start = s.index("#[cfg(test)]")
s = s[:test_start] + r'''#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_caps_negative_and_nan_weights() {
        for args in [
            vec!["--pieces", "120"],
            vec!["--nodes", "0"],
            vec!["--pending-safety", "NaN"],
            vec!["--pending-safety", "-0.1"],
            vec!["--incumbent-pending-safety", "NaN"],
            vec!["--incumbent-pending-safety", "-0.1"],
        ] {
            assert!(settings(&args.iter().map(|x| x.to_string()).collect::<Vec<_>>()).is_err());
        }
    }

    #[test]
    fn accepts_direct_h1_vs_legacy_configuration() {
        let args = vec![
            "--pending-safety", "1.0",
            "--incumbent-pending-safety", "0.0",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { pending_safety: 1.0 });
        assert_eq!(s.incumbent, Strategy { pending_safety: 0.0 });
    }

    #[test]
    fn hidden_holes_do_not_enter_observation() {
        let a = VecDeque::from([Packet { lines: 7, hole: 0 }]);
        let b = VecDeque::from([Packet { lines: 7, hole: 9 }]);
        assert_eq!(serde_json::to_value(observable(&a)).unwrap(), serde_json::to_value(observable(&b)).unwrap());
    }

    #[test]
    fn five_next_does_not_limit_search_depth() {
        let mut seq = Sequence::new(7);
        let visible = (0..6).map(|i| seq.get(i)).collect();
        let p = make_player(visible, Strategy::default()).unwrap();
        assert_eq!(p.bot.player_pieces().next.len(), 5);
        let mut stats = Statistics::default();
        with_search_seed(43, || for _ in 0..1000 { stats.accumulate(p.bot.do_work()); });
        assert!(stats.max_depth > 6);
        assert!(stats.speculative_expansions > 0);
    }

    #[test]
    fn hard_budget_and_authority_unchanged() {
        let mut seq = Sequence::new(1);
        let visible = (0..6).map(|i| seq.get(i)).collect();
        let mut p = make_player(visible, Strategy::default()).unwrap();
        let (_, stats) = choose(&mut p.bot, &[], 0, 0, 1003).unwrap();
        assert_eq!(stats.nodes, 1003);
        assert!(!p.bot.state().forecast.enabled);
    }

    #[test]
    fn authority_and_forecast_agree() {
        for pieces in [0, 13, 14, 40] {
            for attack in 0..10 {
                let incoming = [GarbagePacket { lines: 11, active: true }];
                let mut f = Forecast::snapshot(&incoming, pieces, 2, 4).unwrap();
                let mut board = Board::default();
                f.resolve(&mut board, &[attack], 0);
                let (cancelled, sent) = cancel_plan(attack, 11, pieces, 2);
                let remain = 11 - cancelled;
                let rise = remain.min(8);
                assert_eq!(f.remaining(), remain - rise);
                assert_eq!(f.sent, 2 + sent);
                assert_eq!(board.cols[0], (1 << rise) - 1);
                assert_eq!(board.cols[4], 0);
            }
        }
    }
}
'''

for needle in [
    "config.freestyle_weights.pending_safety = strategy.pending_safety;",
    "config.freestyle_weights.useful_attack_reward = 0.0;",
    "config.freestyle_weights.cancellation_reward = 0.0;",
    "config.freestyle_weights.h9_cavity_excavation = 0.0;",
    "config.dag_backprop_best_demotion = true;",
    "config.dag_backprop_despeculated_values = false;",
]:
    if needle not in s:
        raise SystemExit(f"missing H1 lineage invariant: {needle}")

out.write_text(s)
print(out)
