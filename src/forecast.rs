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
    len:usize,
    packets:[Packet;16],
}
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct Resolution {
    pub cancelled: u32, pub outgoing: u32, pub risen: u32, pub overflow: bool,
}
impl Forecast {
    /// An information-set key must not expose hypothetical hole coordinates.
    pub(crate) fn observed_key(mut self) -> Self {
        for packet in &mut self.packets { packet.hole = 0; }
        self
    }

    pub fn new(packets:&[GarbagePacket], pieces_placed:u32, sent:u32, frames_per_piece:u32, delay:u32, scenario:u32) -> Result<Self,String> {
        if packets.len()>16 {return Err("forecast supports at most 16 observable packets".into());}
        if !(1..=600).contains(&frames_per_piece) || delay>600 {return Err("invalid explicit timing assumption".into());}
        if sent>1_000_000 || pieces_placed>1_000_000 {return Err("history counter exceeds analysis bound".into());}
        let mut f=Self{enabled:true,pieces_placed,sent,frames_per_piece,..Self::default()};
        for (i,p) in packets.iter().enumerate() {
            if p.lines==0 || p.lines>1000 {return Err("packet line count must be 1..1000".into());}
            // Each scenario samples one clean hole per observed packet. Sample
            // columns cover all ten possibilities equally across ten scenarios.
            f.packets[i]=Packet{lines:p.lines,ready_at:if p.active {0} else {delay},hole:((scenario+3*i as u32)%10) as u8};
        }
        f.len=packets.len();Ok(f)
    }
    /// Clock-free snapshot. Unconfirmed arrival times are not invented.
    /// all_ready is an alternative pressure assumption, not actual timing.
    pub fn at_snapshot(packets:&[GarbagePacket], pieces_placed:u32, sent:u32, scenario:u32, all_ready:bool) -> Result<Self,String> {
        let mut f=Self::new(packets,pieces_placed,sent,1,0,scenario)?;
        f.frames_per_piece=0;
        for (i,p) in packets.iter().enumerate() {
            f.packets[i].ready_at=if p.active || all_ready {0} else {u32::MAX};
        }
        Ok(f)
    }
    pub(crate) fn can_rise_in_snapshot(&self) -> bool {
        self.packets[..self.len].iter().any(|p|p.lines>0 && p.ready_at<=self.elapsed_frames)
    }
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
    pub fn resolve(&mut self,board:&mut Board,attacks:&[u32],cleared:u32) -> Resolution {
        let mut result = Resolution::default();
        if !self.enabled || self.topped_out {return result;}
        self.elapsed_frames=self.elapsed_frames.saturating_add(self.frames_per_piece);
        for &attack in attacks {
            let bonus=if self.pieces_placed<14 && self.remaining()>=self.sent {attack} else {0};
            // Ordinary attack is consumed before opening-phase extra cancel;
            // the extra cancel is never sent to an opponent.
            let ordinary=self.consume(attack);
            result.cancelled += ordinary + self.consume(bonus);
            result.outgoing += attack - ordinary;
            self.sent=self.sent.saturating_add(attack-ordinary);
        }
        if cleared==0 {
            for _ in 0..8 {
                if self.len==0 || self.packets[0].ready_at>self.elapsed_frames {break;}
                let hole=self.packets[0].hole;
                self.consume(1);
                if board.cols.iter().any(|&c|c>>39!=0) {self.topped_out=true;result.overflow=true;break;}
                for (x,col) in board.cols.iter_mut().enumerate(){*col=(*col<<1)|u64::from(x!=hole as usize);}
                board.garbage_rows=(board.garbage_rows<<1)|1;
                result.risen += 1;
            }
        }
        self.pieces_placed=self.pieces_placed.saturating_add(1);
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
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
}
