use cold_clear_2::{analysis, replay_check, tbp::Start};
use serde_json::{json, Value};

fn fixtures() -> Value { serde_json::from_str(include_str!("fixtures/replay-v19.json")).unwrap() }
fn input(c: &Value) -> Value {
    let mut start=c["start"].clone();
    start["board"]=Value::Array(c["rows"].as_array().unwrap().iter().map(|row|Value::Array(row.as_str().unwrap().chars().map(|ch|if ch=='.'{Value::Null}else{json!(ch.to_string())}).collect())).collect());
    json!({"start":start,"placement":c["placement"]})
}
#[test]
fn reconstructed_recorded_locks_match_movement_scoring_and_counters() {
    let f=fixtures();let mut failures=Vec::new();
    for c in f["cases"].as_array().unwrap() {
        let id=c["index"].as_u64().unwrap();
        let result=replay_check::check(serde_json::from_value(input(c)).unwrap());
        let out=match result {Ok(o)=>o,Err(e)=>{failures.push(format!("{id}: {e}"));continue;}};
        let e=&c["expected"];
        let actual=json!({"lines":out.lines,"combo":out.consecutive_clears,"b2b":if out.back_to_back{out.b2b_count as u32+1}else{0},"garbage_cleared":out.garbage_cleared,"raw_attack":out.packets,"surge":out.attack.surge_released,"pc":out.perfect_clear});
        if actual!=*e{failures.push(format!("{id}: expected {e}, got {actual}"));}
        if !c["after"].is_null() {
            let mut rows:Vec<String>=out.board.iter().map(|row|row.iter().map(|c|if c.is_some(){'X'}else{'.'}).collect()).collect();
            while rows.last().map(|s|s=="..........").unwrap_or(false){rows.pop();}
            if json!(rows)!=c["after"]{failures.push(format!("{id}: post-clear board differs"));}
        }
    }
    assert!(failures.is_empty(),"{} fixture failures:\n{}",failures.len(),failures.join("\n"));
}
#[test]
fn pending_packets_change_actual_search_scores() {
    let start=json!({"board":[],"queue":["I","O","T","L","J","S"],"hold":"Z","combo":0,"back_to_back":false,"b2b_count":0,"randomizer":{"type":"unknown"}});
    let request=|incoming:Value|serde_json::from_value(json!({"start":start,"incoming":incoming,"pieces_placed":30,"garbage_sent":0,"activation_model":"snapshot","iterations":10})).unwrap();
    let clear=analysis::analyze(request(json!([]))).unwrap();
    let danger=analysis::analyze(request(json!([{"lines":8,"active":true}]))).unwrap();
    assert!(!clear.candidates.is_empty());assert!(!danger.candidates.is_empty());
    assert!(danger.pending_garbage_in_search);
    assert!(danger.candidates[0].mean_score < clear.candidates[0].mean_score);
}
#[test]
fn hidden_replay_state_is_rejected() {
    let start=json!({"board":[],"queue":["I","O","T","L","J","S"],"hold":null,"combo":0,"back_to_back":false,"b2b_count":0,"randomizer":{"type":"unknown"},"seed":42});
    assert!(serde_json::from_value::<Start>(start).is_err());
}
