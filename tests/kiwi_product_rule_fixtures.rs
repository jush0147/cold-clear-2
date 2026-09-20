use cold_clear_2::{
    data::{Board,GameState,Piece,Placement,TetrioRules},
    forecast::Forecast,
    movegen::find_moves_with_clutch,
};
use enumset::EnumSet;
use serde_json::Value;

#[test]
fn pinned_tetrp_product_rule_fixtures() {
    let cases:Vec<Value>=serde_json::from_str(include_str!("../kiwi-product-rule-fixtures.json")).unwrap();
    let mut cancel=0;let mut rescue=0;let mut clear=0;let mut garbage=0;let mut variants=0;
    for (index,c) in cases.iter().enumerate() {
        match c["kind"].as_str().unwrap() {
            "cancel"=>{
                cancel+=1;
                let amounts:Vec<u32>=serde_json::from_value(c["amounts"].clone()).unwrap();
                let attacks:Vec<u32>=serde_json::from_value(c["attacks"].clone()).unwrap();
                let packets:Vec<_>=amounts.iter().map(|&n|(n,0)).collect();
                let mut f=Forecast::new_timed(
                    &packets,c["placed"].as_u64().unwrap() as u32,
                    c["sent"].as_u64().unwrap() as u32,24,0
                ).unwrap();
                f.set_opener_phase_pieces(c["limit"].as_u64().unwrap() as u32);
                f.resolve(&mut Board::default(),&attacks,1);
                assert_eq!(f.remaining() as u64,c["expected_remaining"].as_u64().unwrap(),"cancel {index}: {c}");
                assert_eq!(f.sent as u64,c["expected_sent"].as_u64().unwrap(),"sent {index}: {c}");
            }
            "spawn_rescue"=>{
                rescue+=1;
                let board:Board=serde_json::from_value(c["board"].clone()).unwrap();
                let piece:Piece=serde_json::from_value(c["piece"].clone()).unwrap();
                let allow=c["clutch"].as_bool().unwrap()&&c["lastClear"].as_bool().unwrap();
                let actual=!find_moves_with_clutch(&board,piece,allow).is_empty();
                assert_eq!(actual,c["expected_playing"].as_bool().unwrap(),"spawn rescue {index}: {c}");
                if !actual {
                    assert!(matches!(c["authority_reason"].as_str(),Some("topout")|Some("garbagesmash")),
                        "terminal authority reason is a bounded fixture, not a Kiwi reason claim: {c}");
                }
            }
            "clear_rescue"=>{
                clear+=1;
                let board:Board=serde_json::from_value(c["board"].clone()).unwrap();
                let rules:TetrioRules=serde_json::from_value(c["rules"].clone()).unwrap();
                let placement:Placement=serde_json::from_value(c["placement"].clone()).unwrap();
                let mut state=GameState{board,rules,bag:EnumSet::all(),reserve:Piece::T,
                    back_to_back:false,b2b_count:0,combo:1,forecast:Forecast::default()};
                let info=state.advance(Piece::I,placement);
                assert_eq!(info.lines_cleared as u64,c["expected_lines"].as_u64().unwrap(),"clear lines {index}");
                assert_eq!(state.combo as u64,c["expected_combo"].as_u64().unwrap(),"clear combo {index}");
                let expected:Board=serde_json::from_value(c["expected_board"].clone()).unwrap();
                assert_eq!(state.board,expected,"clear board {index}");
                let actual=!find_moves_with_clutch(&state.board,Piece::I,rules.clutch&&state.combo>0).is_empty();
                assert_eq!(actual,c["expected_playing"].as_bool().unwrap(),"clear rescue {index}: {c}");
            }
            "garbage_boundary"=>{
                garbage+=1;
                let mut board:Board=serde_json::from_value(c["before"].clone()).unwrap();
                let packet=[(1u32,0u32)];
                let mut f=Forecast::new_timed(&packet,30,0,1,0).unwrap();
                f.resolve(&mut board,&[],0);
                let expected_ok=c["expected_ok"].as_bool().unwrap();
                assert_eq!(!f.topped_out,expected_ok,"garbage storage boundary {index}: {c}");
            }
            "rule_variant"=>{
                variants+=1;
                let actual:TetrioRules=serde_json::from_value(c["actual"].clone()).unwrap();
                actual.validate().unwrap();
                let roundtrip=serde_json::to_value(actual).unwrap();
                for (k,v) in c["requested"].as_object().unwrap() {
                    assert_eq!(&roundtrip[k],v,"transported rule {k} fixture {index}");
                }
            }
            other=>panic!("unknown fixture kind {other}"),
        }
    }
    println!("product rule fixtures passed: total={} cancel={} spawn_rescue={} clear_rescue={} garbage_boundary={} rule_variant={}",
        cases.len(),cancel,rescue,clear,garbage,variants);
}
