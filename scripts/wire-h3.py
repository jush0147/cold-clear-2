from pathlib import Path


def replace(text, old, new, count=1):
    actual = text.count(old)
    assert actual == count, (old[:120], actual)
    return text.replace(old, new)


# Add two zero-default H3 terminal-state features. H2 remains fixed and the old
# tetrio_s2 switch remains off, so this does not silently change clear shaping.
p = Path('src/bot/freestyle.rs')
s = p.read_text()
s = replace(
    s,
    '''    /// H2: independently reward visible incoming garbage actually cancelled.\n    #[serde(default)]\n    pub cancellation_reward: f32,\n    pub cell_coveredness: f32,''',
    '''    /// H2: independently reward visible incoming garbage actually cancelled.\n    #[serde(default)]\n    pub cancellation_reward: f32,\n    /// H3: terminal value of B2B count progress toward the Surge threshold.\n    #[serde(default)]\n    pub h3_b2b_charge_value: f32,\n    /// H3: terminal value per line of currently banked Surge once charged.\n    #[serde(default)]\n    pub h3_surge_bank_value: f32,\n    pub cell_coveredness: f32,'''
)
s = replace(
    s,
    '''    reward += weights.useful_attack_reward * useful_outgoing as f32;\n    reward += weights.cancellation_reward * cancelled as f32;\n\n    if info.placement.location.piece == Piece::T''',
    '''    reward += weights.useful_attack_reward * useful_outgoing as f32;\n    reward += weights.cancellation_reward * cancelled as f32;\n\n    // H3 values unrealized B2B/Surge inventory only at the search leaf. H2\n    // already rewards Surge when it is actually released and sent, so release\n    // attack is deliberately not rewarded a second time here. Legacy CC2\n    // already rewards merely having B2B, therefore charge starts at count x1.\n    let (charge_progress, surge_bank) = h3_inventory(state.back_to_back, state.b2b_count);\n    eval += weights.h3_b2b_charge_value * charge_progress as f32;\n    eval += weights.h3_surge_bank_value * surge_bank as f32;\n\n    if info.placement.location.piece == Piece::T'''
)
s = replace(
    s,
    '''fn useful_attack_delta(\n    forecast_enabled: bool,''',
    '''fn h3_inventory(back_to_back: bool, b2b_count: u16) -> (u32, u32) {\n    if !back_to_back {\n        return (0, 0);\n    }\n    let count = b2b_count as u32;\n    (count.min(4), tetrio::surge_size(count))\n}\n\nfn useful_attack_delta(\n    forecast_enabled: bool,'''
)
s = replace(
    s,
    '''mod h2_tests {\n    use super::useful_attack_delta;''',
    '''mod h2_tests {\n    use super::{h3_inventory, useful_attack_delta};\n\n    #[test]\n    fn h3_values_only_live_charge_and_banked_surge() {\n        assert_eq!(h3_inventory(false, 12), (0, 0));\n        assert_eq!(h3_inventory(true, 0), (0, 0));\n        assert_eq!(h3_inventory(true, 1), (1, 0));\n        assert_eq!(h3_inventory(true, 3), (3, 0));\n        assert_eq!(h3_inventory(true, 4), (4, 4));\n        assert_eq!(h3_inventory(true, 9), (4, 9));\n    }'''
)
p.write_text(s)

# Derive the H3 duel harness from the already regression-tested H2 harness. It
# freezes H1 + H2 for both sides and exposes only H3 charge/bank values.
h = Path('src/bin/strategy_h2.rs').read_text()
h = replace(h, '//! KO-only H2 A/B: H1 incumbent versus useful outgoing/cancellation rewards.',
            '//! KO-only H3 A/B: frozen H2 incumbent versus B2B/Surge inventory values.')
h = replace(h, 'struct Strategy { attack: f32, cancel: f32 }',
            'struct Strategy { charge: f32, surge_bank: f32 }')
h = replace(
    h,
    '''            "--attack" => s.strategy.attack = p[1].parse().map_err(|_| "invalid attack reward")?,\n            "--cancel" => s.strategy.cancel = p[1].parse().map_err(|_| "invalid cancellation reward")?,\n            "--incumbent-attack" => s.incumbent.attack = p[1].parse().map_err(|_| "invalid incumbent attack reward")?,\n            "--incumbent-cancel" => s.incumbent.cancel = p[1].parse().map_err(|_| "invalid incumbent cancellation reward")?,''',
    '''            "--charge" => s.strategy.charge = p[1].parse().map_err(|_| "invalid B2B charge value")?,\n            "--surge-bank" => s.strategy.surge_bank = p[1].parse().map_err(|_| "invalid Surge bank value")?,\n            "--incumbent-charge" => s.incumbent.charge = p[1].parse().map_err(|_| "invalid incumbent B2B charge value")?,\n            "--incumbent-surge-bank" => s.incumbent.surge_bank = p[1].parse().map_err(|_| "invalid incumbent Surge bank value")?,'''
)
h = replace(
    h,
    '''        || !s.strategy.attack.is_finite() || s.strategy.attack < 0.0\n        || !s.strategy.cancel.is_finite() || s.strategy.cancel < 0.0\n        || !s.incumbent.attack.is_finite() || s.incumbent.attack < 0.0\n        || !s.incumbent.cancel.is_finite() || s.incumbent.cancel < 0.0\n    {\n        return Err("positive seeds, >=1000 nodes and finite nonnegative H2 rewards required".into());''',
    '''        || !s.strategy.charge.is_finite() || s.strategy.charge < 0.0\n        || !s.strategy.surge_bank.is_finite() || s.strategy.surge_bank < 0.0\n        || !s.incumbent.charge.is_finite() || s.incumbent.charge < 0.0\n        || !s.incumbent.surge_bank.is_finite() || s.incumbent.surge_bank < 0.0\n    {\n        return Err("positive seeds, >=1000 nodes and finite nonnegative H3 values required".into());'''
)
h = replace(
    h,
    '''    // H1 is the frozen incumbent for BOTH sides in H2.\n    config.freestyle_weights.softdrop = 0.0;\n    config.freestyle_weights.pending_safety = 1.0;\n    config.freestyle_weights.useful_attack_reward = strategy.attack;\n    config.freestyle_weights.cancellation_reward = strategy.cancel;''',
    '''    // H1 + H2 winner are frozen for BOTH sides in H3.\n    config.freestyle_weights.softdrop = 0.0;\n    config.freestyle_weights.pending_safety = 1.0;\n    config.freestyle_weights.useful_attack_reward = 1.0;\n    config.freestyle_weights.cancellation_reward = 0.0;\n    config.freestyle_weights.h3_b2b_charge_value = strategy.charge;\n    config.freestyle_weights.h3_surge_bank_value = strategy.surge_bank;'''
)
h = replace(
    h,
    '''        "attack_reward": s.strategy.attack,\n        "cancellation_reward": s.strategy.cancel,\n        "incumbent_attack_reward": s.incumbent.attack,\n        "incumbent_cancellation_reward": s.incumbent.cancel,''',
    '''        "h3_b2b_charge_value": s.strategy.charge,\n        "h3_surge_bank_value": s.strategy.surge_bank,\n        "incumbent_h3_b2b_charge_value": s.incumbent.charge,\n        "incumbent_h3_surge_bank_value": s.incumbent.surge_bank,'''
)
h = replace(
    h,
    '''        "experiment": "H2 useful outgoing attack and cancellation",\n        "incumbent": "H1 pending_safety=1.0 plus configured H2 rewards",\n        "candidate_attack_reward": s.strategy.attack,\n        "candidate_cancellation_reward": s.strategy.cancel,\n        "incumbent_attack_reward": s.incumbent.attack,\n        "incumbent_cancellation_reward": s.incumbent.cancel,''',
    '''        "experiment": "H3 B2B charge and Surge bank terminal value",\n        "incumbent": "H1 pending_safety=1.0 + H2 useful_attack_reward=1.0,cancellation_reward=0",\n        "candidate_h3_b2b_charge_value": s.strategy.charge,\n        "candidate_h3_surge_bank_value": s.strategy.surge_bank,\n        "incumbent_h3_b2b_charge_value": s.incumbent.charge,\n        "incumbent_h3_surge_bank_value": s.incumbent.surge_bank,'''
)
h = replace(
    h,
    'if s.strategy.attack == s.incumbent.attack && s.strategy.cancel == s.incumbent.cancel {',
    'if s.strategy.charge == s.incumbent.charge && s.strategy.surge_bank == s.incumbent.surge_bank {'
)
h = replace(h, '"H1/H1 paired game not repeatable"', '"H3 A/A paired game not repeatable"')
h = replace(
    h,
    '''            vec!["--attack", "NaN"],\n            vec!["--cancel", "-0.1"],\n            vec!["--incumbent-attack", "NaN"],\n            vec!["--incumbent-cancel", "-0.1"],''',
    '''            vec!["--charge", "NaN"],\n            vec!["--surge-bank", "-0.1"],\n            vec!["--incumbent-charge", "NaN"],\n            vec!["--incumbent-surge-bank", "-0.1"],'''
)
h = replace(
    h,
    '''    fn accepts_direct_h2_vs_h2_configuration() {\n        let args = vec![\n            "--attack", "1.0", "--cancel", "0",\n            "--incumbent-attack", "0.5", "--incumbent-cancel", "0",\n        ].into_iter().map(str::to_string).collect::<Vec<_>>();\n        let s = settings(&args).unwrap();\n        assert_eq!((s.strategy.attack, s.strategy.cancel), (1.0, 0.0));\n        assert_eq!((s.incumbent.attack, s.incumbent.cancel), (0.5, 0.0));\n    }''',
    '''    fn accepts_direct_h3_vs_h3_configuration() {\n        let args = vec![\n            "--charge", "0.5", "--surge-bank", "0.25",\n            "--incumbent-charge", "0.1", "--incumbent-surge-bank", "0.05",\n        ].into_iter().map(str::to_string).collect::<Vec<_>>();\n        let s = settings(&args).unwrap();\n        assert_eq!((s.strategy.charge, s.strategy.surge_bank), (0.5, 0.25));\n        assert_eq!((s.incumbent.charge, s.incumbent.surge_bank), (0.1, 0.05));\n    }'''
)
Path('src/bin/strategy_h3.rs').write_text(h)
