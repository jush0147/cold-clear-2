//! Observable garbage queue primitives, not a complete TL S2 timing engine.
//!
//! Only packet size and its CURRENT activation state are accepted. Hidden hole
//! coordinates, future activation timestamps and RNG state are deliberately not
//! part of this transport. The replay adapter must establish what was visible.
//! Opening double-cancel, Surge packet scheduling and board insertion are not
//! implemented here. Callers must not label these primitives exact S2 search.

use std::collections::VecDeque;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GarbagePacket {
    pub lines: u32,
    pub active: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GarbageQueue { packets: VecDeque<GarbagePacket> }

impl TryFrom<Vec<GarbagePacket>> for GarbageQueue {
    type Error = String;
    fn try_from(packets: Vec<GarbagePacket>) -> Result<Self, Self::Error> {
        let mut total = 0u32;
        for packet in &packets {
            if packet.lines == 0 { return Err("garbage packets must have a positive line count".into()); }
            total = total.checked_add(packet.lines).ok_or("garbage line count overflow")?;
        }
        Ok(Self { packets: packets.into_iter().collect() })
    }
}
impl Serialize for GarbageQueue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> { self.packets.serialize(serializer) }
}
impl<'de> Deserialize<'de> for GarbageQueue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let packets = Vec::<GarbagePacket>::deserialize(deserializer)?;
        Self::try_from(packets).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NormalLockResolution {
    pub remaining: GarbageQueue,
    pub cancelled: u32,
    pub outgoing: u32,
    pub risen: u32,
}

impl GarbageQueue {
    pub fn is_empty(&self) -> bool { self.packets.is_empty() }
    pub fn total_lines(&self) -> u32 { self.packets.iter().map(|p| p.lines).sum() }
    pub fn active_lines(&self) -> u32 { self.packets.iter().filter(|p| p.active).map(|p| p.lines).sum() }

    /// One-to-one cancellation of already observable, cancelable packets.
    /// Returns unused attack. Pending packets can be canceled without rising.
    pub fn cancel_one_to_one(&mut self, mut attack: u32) -> u32 {
        while attack != 0 {
            let packet = match self.packets.front_mut() { Some(p) => p, None => break };
            let cancelled = attack.min(packet.lines);
            attack -= cancelled;
            packet.lines -= cancelled;
            if packet.lines == 0 { self.packets.pop_front(); }
        }
        attack
    }

    /// Remove a ready FIFO prefix, subject to a supplied cap and combo blocking.
    /// No wall clock is advanced, and inactive packets are never made active.
    /// This does not guess garbage holes or insert blocks into a search board.
    pub fn take_active_prefix(&mut self, cleared_lines: u32, cap: u32) -> u32 {
        if cleared_lines != 0 { return 0; }
        let mut taken = 0;
        while taken < cap {
            let packet = match self.packets.front_mut() { Some(p) if p.active => p, _ => break };
            let count = (cap - taken).min(packet.lines);
            packet.lines -= count;
            taken += count;
            if packet.lines == 0 { self.packets.pop_front(); }
        }
        taken
    }

    /// Pure normal-cancel scenario. The explicitly named policy prevents an
    /// opening-phase replay from silently being treated as one-to-one S2 play.
    pub fn resolve_one_to_one(&self, attack: u32, cleared_lines: u32, cap: u32) -> NormalLockResolution {
        let mut remaining = self.clone();
        let outgoing = remaining.cancel_one_to_one(attack);
        let cancelled = attack - outgoing;
        let risen = remaining.take_active_prefix(cleared_lines, cap);
        NormalLockResolution { remaining, cancelled, outgoing, risen }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn queue(active: u32, pending: u32) -> GarbageQueue {
        let mut packets = vec![];
        if active != 0 { packets.push(GarbagePacket { lines: active, active: true }); }
        if pending != 0 { packets.push(GarbagePacket { lines: pending, active: false }); }
        GarbageQueue::try_from(packets).unwrap()
    }
    #[test]
    fn pending_is_cancelable_but_does_not_rise() {
        let before = queue(0, 8);
        let r = before.resolve_one_to_one(3, 0, 8);
        assert_eq!((r.cancelled, r.outgoing, r.risen), (3, 0, 0));
        assert_eq!(r.remaining.total_lines(), 5);
        assert_eq!(r.remaining.active_lines(), 0);
        assert_eq!(before.total_lines(), 8);
    }
    #[test]
    fn clears_block_rise_and_nonclears_obey_cap() {
        assert_eq!(queue(10, 0).resolve_one_to_one(0, 1, 8).risen, 0);
        let r = queue(10, 0).resolve_one_to_one(0, 0, 8);
        assert_eq!(r.risen, 8);
        assert_eq!(r.remaining.total_lines(), 2);
    }
    #[test]
    fn cancelling_across_packet_boundaries_preserves_activation() {
        let r = queue(3, 7).resolve_one_to_one(5, 0, 8);
        assert_eq!(r.cancelled, 5);
        assert_eq!(r.risen, 0);
        assert_eq!(r.remaining, queue(0, 5));
    }
    #[test]
    fn excess_attack_is_outgoing_not_negative_garbage() {
        let r = queue(3, 2).resolve_one_to_one(9, 0, 8);
        assert_eq!((r.cancelled, r.outgoing, r.risen), (5, 4, 0));
        assert!(r.remaining.is_empty());
    }
    #[test]
    fn inactive_prefix_is_not_bypassed_or_autoactivated() {
        let q = GarbageQueue::try_from(vec![GarbagePacket { lines: 2, active: false }, GarbagePacket { lines: 3, active: true }]).unwrap();
        let r = q.resolve_one_to_one(0, 0, 8);
        assert_eq!(r.remaining, q);
        assert_eq!(r.risen, 0);
    }
    #[test]
    fn invalid_counts_and_hidden_metadata_are_rejected() {
        assert!(GarbageQueue::try_from(vec![GarbagePacket { lines: 0, active: true }]).is_err());
        assert!(GarbageQueue::try_from(vec![GarbagePacket { lines: u32::MAX, active: true }, GarbagePacket { lines: 1, active: true }]).is_err());
        assert!(serde_json::from_str::<GarbageQueue>(r#"[{"lines":3,"active":true,"hole":4}]"#).is_err());
        assert!(serde_json::from_str::<GarbageQueue>(r#"[{"lines":3}]"#).is_err());
    }
    #[test]
    fn packet_model_matches_independent_per_line_model_30618_cases() {
        for active in 0..=8u32 {
            for pending in 0..=8u32 {
                for attack in 0..=20u32 {
                    for cap in 0..=8u32 {
                        for cleared in 0..=1u32 {
                            let q = queue(active, pending);
                            let r = q.resolve_one_to_one(attack, cleared, cap);
                            let mut rows: VecDeque<bool> = std::iter::repeat(true).take(active as usize).chain(std::iter::repeat(false).take(pending as usize)).collect();
                            let mut left = attack;
                            let mut cancelled = 0;
                            while left > 0 && !rows.is_empty() { rows.pop_front(); left -= 1; cancelled += 1; }
                            let mut risen = 0;
                            while cleared == 0 && risen < cap && rows.front() == Some(&true) { rows.pop_front(); risen += 1; }
                            assert_eq!((r.cancelled, r.outgoing, r.risen), (cancelled, left, risen));
                            assert_eq!(r.remaining.total_lines(), rows.len() as u32);
                            assert_eq!(r.remaining.active_lines(), rows.iter().filter(|&&b| b).count() as u32);
                            assert_eq!(r.remaining.total_lines() + r.cancelled + r.risen, active + pending);
                        }
                    }
                }
            }
        }
    }
}
