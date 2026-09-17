from pathlib import Path

p = Path('src/bin/strategy_h2.rs')
s = p.read_text()

def rep(old, new, n=1):
    global s
    c = s.count(old)
    assert c == n, (old[:100], c)
    s = s.replace(old, new)

rep(
    'struct Settings { seeds: u64, start: u64, nodes: u64, strategy: Strategy }',
    'struct Settings { seeds: u64, start: u64, nodes: u64, strategy: Strategy, incumbent: Strategy }'
)
rep(
    '        strategy: Strategy::default(),\n    };',
    '        strategy: Strategy::default(),\n        incumbent: Strategy::default(),\n    };'
)
rep(
    '            "--cancel" => s.strategy.cancel = p[1].parse().map_err(|_| "invalid cancellation reward")?,\n            _ =>',
    '            "--cancel" => s.strategy.cancel = p[1].parse().map_err(|_| "invalid cancellation reward")?,\n            "--incumbent-attack" => s.incumbent.attack = p[1].parse().map_err(|_| "invalid incumbent attack reward")?,\n            "--incumbent-cancel" => s.incumbent.cancel = p[1].parse().map_err(|_| "invalid incumbent cancellation reward")?,\n            _ =>'
)
rep(
    '        || !s.strategy.cancel.is_finite() || s.strategy.cancel < 0.0\n    {',
    '        || !s.strategy.cancel.is_finite() || s.strategy.cancel < 0.0\n        || !s.incumbent.attack.is_finite() || s.incumbent.attack < 0.0\n        || !s.incumbent.cancel.is_finite() || s.incumbent.cancel < 0.0\n    {'
)
rep(
    '    let incumbent = Strategy::default();\n    let strategies = if swapped {\n        [s.strategy, incumbent]\n    } else {\n        [incumbent, s.strategy]\n    };',
    '    let incumbent = s.incumbent;\n    let strategies = if swapped {\n        [s.strategy, incumbent]\n    } else {\n        [incumbent, s.strategy]\n    };'
)
rep(
    '        "cancellation_reward": s.strategy.cancel,\n        "nodes":',
    '        "cancellation_reward": s.strategy.cancel,\n        "incumbent_attack_reward": s.incumbent.attack,\n        "incumbent_cancellation_reward": s.incumbent.cancel,\n        "nodes":'
)
rep(
    '        "incumbent": "H1 pending_safety=1.0; legacy evaluator otherwise unchanged",\n        "candidate_attack_reward": s.strategy.attack,\n        "candidate_cancellation_reward": s.strategy.cancel,',
    '        "incumbent": "H1 pending_safety=1.0 plus configured H2 rewards",\n        "candidate_attack_reward": s.strategy.attack,\n        "candidate_cancellation_reward": s.strategy.cancel,\n        "incumbent_attack_reward": s.incumbent.attack,\n        "incumbent_cancellation_reward": s.incumbent.cancel,'
)
rep(
    '            if s.strategy.attack == 0.0 && s.strategy.cancel == 0.0 {',
    '            if s.strategy.attack == s.incumbent.attack && s.strategy.cancel == s.incumbent.cancel {'
)
rep(
    '            vec!["--cancel", "-0.1"],\n        ] {',
    '            vec!["--cancel", "-0.1"],\n            vec!["--incumbent-attack", "NaN"],\n            vec!["--incumbent-cancel", "-0.1"],\n        ] {'
)

# Add a parser regression proving both finalist configurations can be expressed.
rep(
    '    #[test]\n    fn hidden_holes_do_not_enter_observation() {',
    '''    #[test]\n    fn accepts_direct_h2_vs_h2_configuration() {\n        let args = vec![\n            "--attack", "1.0", "--cancel", "0",\n            "--incumbent-attack", "0.5", "--incumbent-cancel", "0",\n        ].into_iter().map(str::to_string).collect::<Vec<_>>();\n        let s = settings(&args).unwrap();\n        assert_eq!((s.strategy.attack, s.strategy.cancel), (1.0, 0.0));\n        assert_eq!((s.incumbent.attack, s.incumbent.cancel), (0.5, 0.0));\n    }\n\n    #[test]\n    fn hidden_holes_do_not_enter_observation() {'''
)

p.write_text(s)
