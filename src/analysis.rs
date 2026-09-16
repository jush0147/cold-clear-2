//! Snapshot-only pending-aware analysis. This is a scenario approximation,
//! not a certified real-time TETR.IO emulator or a live-game automation API.
use std::{collections::HashMap,sync::Arc};
use serde::{Deserialize,Serialize};
use crate::{bot::BotConfig,data::Placement,forecast::Forecast,tbp::{Start,Randomizer},tetrio::garbage::GarbagePacket,try_create_bot};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub start:Start,
    pub incoming:Vec<GarbagePacket>,
    pub pieces_placed:u32,
    pub garbage_sent:u32,
    /// Explicit pace assumption, not actual future replay timing.
    pub frames_per_piece:u32,
    /// Explicit delay assumption for currently inactive packets.
    pub pending_delay_frames:u32,
    /// Total work iterations across ten equally weighted hole scenarios.
    pub iterations:u32,
}
#[derive(Serialize)]
pub struct Candidate {pub placement:Placement,pub mean_score:f64,pub worst_score:f32,pub scenarios:u32}
#[derive(Serialize)]
pub struct Report {
    pub candidates:Vec<Candidate>,pub nodes:u64,pub scenarios:u32,
    pub pending_garbage_in_search:bool,pub rules_parity_verified:bool,
    pub frames_per_piece:u32,pub pending_delay_frames:u32,
    pub assumptions:Vec<&'static str>,
}
pub fn analyze(request:Request)->Result<Report,String>{
    request.start.validate()?;
    if request.start.queue.len()!=6 {return Err("exactly current + five NEXT pieces are required".into());}
    if !matches!(request.start.randomizer,Randomizer::Unknown){return Err("hidden randomizer state is not accepted by replay analysis".into());}
    if !(10..=10000).contains(&request.iterations){return Err("iterations must be 10..10000".into());}
    let mut scores:HashMap<Placement,(f64,f32,u32)>=HashMap::new();let mut nodes=0;
    let config=Arc::new(BotConfig::default());
    for scenario in 0..10 {
        let start=Start{board:request.start.board,queue:request.start.queue.clone(),hold:request.start.hold,
            combo:request.start.combo,back_to_back:request.start.back_to_back,b2b_count:request.start.b2b_count,randomizer:Randomizer::Unknown};
        let forecast=Forecast::new(&request.incoming,request.pieces_placed,request.garbage_sent,request.frames_per_piece,request.pending_delay_frames,scenario)?;
        let mut bot=try_create_bot(start,config.clone())?;bot.set_forecast(forecast);
        let iterations=request.iterations/10+u32::from(scenario<request.iterations%10);
        for _ in 0..iterations {nodes+=bot.do_work().nodes;}
        for (placement,score) in bot.ranked_suggestions(){
            let entry=scores.entry(placement).or_insert((0.0,f32::INFINITY,0));entry.0+=score as f64;entry.1=entry.1.min(score);entry.2+=1;
        }
    }
    let mut candidates:Vec<_>=scores.into_iter().filter(|(_,v)|v.2==10).map(|(placement,(sum,worst,count))|Candidate{placement,mean_score:sum/count as f64,worst_score:worst,scenarios:count}).collect();
    candidates.sort_by(|a,b|b.mean_score.total_cmp(&a.mean_score));
    Ok(Report{candidates,nodes,scenarios:10,pending_garbage_in_search:true,rules_parity_verified:false,
        frames_per_piece:request.frames_per_piece,pending_delay_frames:request.pending_delay_frames,
        assumptions:vec!["Only already observable incoming packets are modeled; no opponent board or future attacks.",
        "Unknown holes use ten equally weighted clean-hole scenarios, not replay RNG or future events.",
        "All inactive packets use the supplied delay estimate; placements use the supplied fixed pace.",
        "Per-scenario future search can be optimistic about information revealed later; scores are heuristic, not win probabilities."]})
}
