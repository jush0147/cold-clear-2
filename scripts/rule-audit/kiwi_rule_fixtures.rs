use cold_clear_2::{data::{Board,GameState,Piece,Placement,TetrioRules},forecast::Forecast,movegen::find_moves_with_clutch};
use enumset::EnumSet;
use serde_json::Value;
#[test]
fn pinned_tetrp_differential_fixtures() {
    let cases:Vec<Value>=serde_json::from_str(include_str!("../kiwi-rule-fixtures.json")).unwrap();
    for (index,c) in cases.iter().enumerate() {
        match c["kind"].as_str().unwrap() {
            "cancel"=>{
                let amounts:Vec<u32>=serde_json::from_value(c["amounts"].clone()).unwrap();
                let attacks:Vec<u32>=serde_json::from_value(c["attacks"].clone()).unwrap();
                let packets:Vec<_>=amounts.iter().map(|&n|(n,0)).collect();
                let mut f=Forecast::new_timed(&packets,c["placed"].as_u64().unwrap() as u32,c["sent"].as_u64().unwrap() as u32,24,0).unwrap();
                f.set_opener_phase_pieces(c["limit"].as_u64().unwrap() as u32);
                f.resolve(&mut Board::default(),&attacks,1);
                assert_eq!(f.remaining() as u64,c["expected_remaining"].as_u64().unwrap(),"cancel case {index}: {c}");
                assert_eq!(f.sent as u64,c["expected_sent"].as_u64().unwrap(),"sent case {index}: {c}");
            }
            "rescue"=>{
                let board:Board=serde_json::from_value(c["board"].clone()).unwrap();
                let piece:Piece=serde_json::from_value(c["piece"].clone()).unwrap();
                let allow=c["clutch"].as_bool().unwrap()&&c["lastClear"].as_bool().unwrap();
                let actual=!find_moves_with_clutch(&board,piece,allow).is_empty();
                assert_eq!(actual,c["expected_playing"].as_bool().unwrap(),"spawn rescue case {index}");
            }
            "clear_rescue"=>{
                let board:Board=serde_json::from_value(c["board"].clone()).unwrap();
                let rules:TetrioRules=serde_json::from_value(c["rules"].clone()).unwrap();
                let placement:Placement=serde_json::from_value(c["placement"].clone()).unwrap();
                let mut state=GameState{board,rules,bag:EnumSet::all(),reserve:Piece::T,back_to_back:false,b2b_count:0,combo:1,forecast:Forecast::default()};
                let info=state.advance(Piece::I,placement);
                assert_eq!(info.lines_cleared as u64,c["expected_lines"].as_u64().unwrap());
                assert_eq!(state.combo as u64,c["expected_combo"].as_u64().unwrap());
                let expected:Board=serde_json::from_value(c["expected_board"].clone()).unwrap();
                assert_eq!(state.board,expected,"clear board case {index}");
                let actual=!find_moves_with_clutch(&state.board,Piece::I,rules.clutch&&state.combo>0).is_empty();
                assert_eq!(actual,c["expected_playing"].as_bool().unwrap(),"clear rescue case {index}");
            }
            _=>panic!("Unknown audit fixture"),
        }
    }
    println!("{} pinned Tetrp differential fixtures passed",cases.len());
}
