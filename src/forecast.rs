//! Explicit hypothetical garbage scenarios for snapshot analysis.
//! Hole positions and activation delays here are assumptions, NEVER replay RNG.
use crate::data::Board;
use crate::tetrio::garbage::GarbagePacket;
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Hash)]
struct Packet { lines:u32, ready_at:u32, hole:u8 }
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Hash)]
pub struct Forecast {
    pub enabled:bool,
    pub topped_out:bool,
    pub elapsed_frames:u32,
    pub pieces_placed:u32,
    pub sent:u32,
    frames_per_piece:u32,
    opener_phase_pieces:u32,
    authority_clock:bool,
    authority_frame:u32,
    garbage_margin_frames:u32,
    garbage_multiplier_bits:u64,
    garbage_increase_per_second_bits:u64,
    len:usize,
    packets:[Packet;16],
}
impl Forecast {
    /// Zero-gravity snapshot; no PPS-derived future activation timestamps.
    pub fn snapshot(packets: &[GarbagePacket], pieces: u32, sent: u32, scenario: u32) -> Result<Self, String> {
        let mut f = Self::new(packets, pieces, sent, 1, 600, scenario)?;
        f.frames_per_piece = 0;
        Ok(f)
    }

    pub fn new(packets:&[GarbagePacket], pieces_placed:u32, sent:u32, frames_per_piece:u32, delay:u32, scenario:u32) -> Result<Self,String> {
        if delay>600 {return Err("invalid explicit timing assumption".into());}
        let timed: Vec<_> = packets.iter()
            .map(|p| (p.lines, if p.active { 0 } else { delay }))
            .collect();
        Self::new_timed(&timed, pieces_placed, sent, frames_per_piece, scenario)
    }

    /// Pending-aware forecast with an explicit remaining activation delay per
    /// already-observable packet. The caller owns the observation boundary;
    /// hidden future packets and hole coordinates must never be supplied.
    pub fn new_timed(packets:&[(u32,u32)], pieces_placed:u32, sent:u32, frames_per_piece:u32, scenario:u32) -> Result<Self,String> {
        if packets.len()>16 {return Err("forecast supports at most 16 observable packets".into());}
        if !(1..=600).contains(&frames_per_piece) {return Err("invalid explicit timing assumption".into());}
        if sent>1_000_000 || pieces_placed>1_000_000 {return Err("history counter exceeds analysis bound".into());}
        let mut f=Self{enabled:true,pieces_placed,sent,frames_per_piece,opener_phase_pieces:14,..Self::default()};
        for (i,&(lines,ready_in_frames)) in packets.iter().enumerate() {
            if lines==0 || lines>1000 {return Err("packet line count must be 1..1000".into());}
            if ready_in_frames>600 {return Err("packet activation delay exceeds analysis bound".into());}
            // Each scenario samples one clean hole per observed packet. Sample
            // columns cover all ten possibilities equally across ten scenarios.
            f.packets[i]=Packet{lines,ready_at:ready_in_frames,hole:((scenario+3*i as u32)%10) as u8};
        }
        f.len=packets.len();Ok(f)
    }

    pub fn new_timed_with_clock(
        packets:&[(u32,u32)],
        pieces_placed:u32,
        sent:u32,
        frames_per_piece:u32,
        authority_frame:u32,
        garbage_multiplier:f64,
        garbage_margin_frames:u32,
        garbage_increase_per_second:f64,
        scenario:u32,
    ) -> Result<Self,String> {
        if !garbage_multiplier.is_finite() || garbage_multiplier <= 0.0 || garbage_multiplier > 100.0 {
            return Err("invalid authority garbage multiplier".into());
        }
        if !garbage_increase_per_second.is_finite() || garbage_increase_per_second < 0.0 || garbage_increase_per_second > 10.0 {
            return Err("invalid authority garbage increase rate".into());
        }
        let mut f=Self::new_timed(packets,pieces_placed,sent,frames_per_piece,scenario)?;
        f.authority_clock=true;
        f.authority_frame=authority_frame;
        f.garbage_margin_frames=garbage_margin_frames;
        f.garbage_multiplier_bits=garbage_multiplier.to_bits();
        f.garbage_increase_per_second_bits=garbage_increase_per_second.to_bits();
        Ok(f)
    }

    pub fn next_attack_multiplier(&self)->f64 {
        if !self.authority_clock {
            return 1.0;
        }
        let current=f64::from_bits(self.garbage_multiplier_bits);
        if self.frames_per_piece==0 {
            return current;
        }
        let lock_frame=self.authority_frame
            .saturating_add(self.elapsed_frames)
            .saturating_add(self.frames_per_piece.saturating_sub(1));
        let threshold=self.garbage_margin_frames.saturating_add(1);
        let active_at_snapshot=self.authority_frame.saturating_sub(threshold);
        let active_at_lock=lock_frame.saturating_sub(threshold);
        let extra_frames=active_at_lock.saturating_sub(active_at_snapshot);
        let rate=f64::from_bits(self.garbage_increase_per_second_bits);
        current + extra_frames as f64 * rate / 60.0
    }

    pub fn authority_clock_enabled(&self)->bool { self.authority_clock }
    pub fn set_opener_phase_pieces(&mut self, pieces:u32) { self.opener_phase_pieces=pieces; }

    pub fn remaining(&self)->u32 {self.packets[..self.len].iter().map(|p|p.lines).sum()}
    fn consume(&mut self,mut lines:u32)->u32 {
        let mut consumed=0;
        while lines>0 && self.len>0 {
            let n=lines.min(self.packets[0].lines);lines-=n;consumed+=n;self.packets[0].lines-=n;
            if self.packets[0].lines==0 {self.pop();}
        }
        consumed
    }
    fn pop(&mut self){self.packets.copy_within(1..self.len,0);self.len-=1;self.packets[self.len]=Packet::default();}
    pub fn resolve(&mut self,board:&mut Board,attacks:&[u32],cleared:u32) {
        if !self.enabled || self.topped_out {return;}
        self.elapsed_frames=self.elapsed_frames.saturating_add(self.frames_per_piece);
        for &attack in attacks {
            let (cancelled, outgoing) = crate::ko_support::cancel_plan(attack, self.remaining(), self.pieces_placed, self.sent, self.opener_phase_pieces);
            self.consume(cancelled);
            self.sent=self.sent.saturating_add(outgoing);
        }
        if cleared==0 {
            for _ in 0..8 {
                if self.len==0 || self.packets[0].ready_at>self.elapsed_frames {break;}
                let hole=self.packets[0].hole;
                self.consume(1);
                // Pinned Tetrp pushLine rejects only when the storage top row is FULL.
                // A partially occupied top row is shifted out by the authority; using
                // any() here made the forecast spuriously garbagesmash early.
                if board.cols.iter().all(|&c|c>>39!=0) {self.topped_out=true;break;}
                for (x,col) in board.cols.iter_mut().enumerate(){*col=(*col<<1)|u64::from(x!=hole as usize);}
                board.garbage_rows=(board.garbage_rows<<1)|1;
            }
        }
        self.pieces_placed=self.pieces_placed.saturating_add(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn garbage_top_boundary_matches_tetrp_full_row_rule(){
        let p=[GarbagePacket{lines:1,active:true}];
        let mut partial=Board::default();partial.cols[0]=1u64<<39;
        let mut f=Forecast::new(&p,20,0,1,0,0).unwrap();
        f.resolve(&mut partial,&[],0);
        assert!(!f.topped_out);
        let mut full=Board::default();
        for c in &mut full.cols {*c|=1u64<<39;}
        let mut f=Forecast::new(&p,20,0,1,0,0).unwrap();
        f.resolve(&mut full,&[],0);
        assert!(f.topped_out);
    }
    #[test]
    fn opening_bonus_cancels_but_never_becomes_outgoing(){
        let p=[GarbagePacket{lines:5,active:true}];let mut f=Forecast::new(&p,0,0,12,20,0).unwrap();
        f.resolve(&mut Board::default(),&[3],2);assert_eq!((f.remaining(),f.sent),(0,0));
        let mut f=Forecast::new(&p,14,0,12,20,0).unwrap();f.resolve(&mut Board::default(),&[3],2);assert_eq!(f.remaining(),2);
    }
    #[test]
    fn active_rises_with_provenance_but_a_clear_blocks_it(){
        let p=[GarbagePacket{lines:10,active:true}];let mut f=Forecast::new(&p,30,0,12,20,4).unwrap();let mut b=Board::default();
        f.resolve(&mut b,&[],1);assert_eq!(b,Board::default());
        f.resolve(&mut b,&[],0);assert_eq!(f.remaining(),2);assert_eq!(b.cols[4],0);assert_eq!(b.cols[0],255);assert_eq!(b.garbage_rows,255);
    }
    #[test]
    fn forecast_clock_and_queue_are_part_of_state(){
        let p=[GarbagePacket{lines:3,active:false}];let mut f=Forecast::new(&p,30,0,12,20,0).unwrap();let original=f;let mut b=Board::default();
        f.resolve(&mut b,&[],0);assert_eq!(f.remaining(),3);assert_ne!(f,original);
        f.resolve(&mut b,&[],0);assert_eq!(f.remaining(),0);assert_ne!(b,Board::default());
    }
    #[test]
    fn per_packet_remaining_delays_are_preserved(){
        let p=[(2,5),(3,13)];
        let mut f=Forecast::new_timed(&p,30,0,6,4).unwrap();
        let mut b=Board::default();
        f.resolve(&mut b,&[],0);
        assert_eq!(f.remaining(),3);
        assert_ne!(b,Board::default());
        let after_first=b;
        f.resolve(&mut b,&[],0);
        assert_eq!(f.remaining(),3);
        assert_eq!(b,after_first);
        f.resolve(&mut b,&[],0);
        assert_eq!(f.remaining(),0);
    }
    #[test]
    fn authority_clock_scales_at_the_lock_frame() {
        let mut f=Forecast::new_timed_with_clock(&[],30,0,30,10800,1.0,10800,0.008,0).unwrap();
        // Snapshot at frame 10800, next lock at 10829. Tetrp starts increasing
        // after frame 10801, so 28 increments have occurred by the lock frame.
        let expected=1.0 + 28.0 * 0.008 / 60.0;
        assert!((f.next_attack_multiplier()-expected).abs()<1e-12);
        f.resolve(&mut Board::default(),&[],0);
        let expected2=1.0 + 58.0 * 0.008 / 60.0;
        assert!((f.next_attack_multiplier()-expected2).abs()<1e-12);
    }

    #[test]
    fn authority_clock_uses_snapshot_multiplier_after_margin() {
        let f=Forecast::new_timed_with_clock(&[],30,0,20,12000,1.0265333333333333,10800,0.008,0).unwrap();
        let expected=1.0265333333333333 + 19.0 * 0.008 / 60.0;
        assert!((f.next_attack_multiplier()-expected).abs()<1e-12);
        assert!(f.authority_clock_enabled());
    }
}
