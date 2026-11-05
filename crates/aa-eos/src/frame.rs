//! Datagrams over EOS P2P packets.
//!
//! A P2P packet holds at most `EOS_P2P_MAX_PACKET_SIZE` bytes (1170), and UDP,
//! which the session was written for, holds 65,535 by fragmenting below it. This
//! does the same job: each packet starts with a tag, a datagram that fits goes
//! in one, a longer one in numbered parts. As with IP fragments, one lost part
//! loses the datagram, and the session's own sequencing and channels above cope.
//!
//! Every packet comes from another player's machine, so nothing here trusts one:
//! a malformed packet is dropped and counted.
use std::collections::{HashMap, VecDeque};

/// `EOS_P2P_MAX_PACKET_SIZE`.
pub const MAX_PACKET: usize = eos_sys::EOS_P2P_MAX_PACKET_SIZE as usize;
/// The largest datagram: as UDP's.
pub const MAX_DATAGRAM: usize = 65_535;

const WHOLE: u8 = 0;
const PART: u8 = 1;
/// Tag, datagram number (2), part index, part count.
const PART_HEADER: usize = 5;
const PART_DATA: usize = MAX_PACKET - PART_HEADER;
const MAX_PARTS: usize = MAX_DATAGRAM.div_ceil(PART_DATA);
/// Datagrams being put together per peer before the oldest gives up: only a
/// lost part leaves one unfinished, so it never holds a finished datagram back.
const OPEN_PER_PEER: usize = 16;

/// Splits outgoing datagrams.
#[derive(Default)]
pub struct Framer {
    next: u16,
}

impl Framer {
    /// The packets of `data`, each given to `out`. `false` when it is longer
    /// than a datagram can be, and nothing was sent.
    pub fn frame(&mut self, data: &[u8], mut out: impl FnMut(&[u8])) -> bool {
        if data.len() > MAX_DATAGRAM {
            return false;
        }
        let mut packet = Vec::with_capacity(MAX_PACKET);
        if data.len() < MAX_PACKET {
            packet.push(WHOLE);
            packet.extend_from_slice(data);
            out(&packet);
            return true;
        }
        let id = self.next;
        self.next = self.next.wrapping_add(1);
        let count = data.len().div_ceil(PART_DATA);
        for (i, part) in data.chunks(PART_DATA).enumerate() {
            packet.clear();
            packet.push(PART);
            packet.extend_from_slice(&id.to_le_bytes());
            packet.push(i as u8);
            packet.push(count as u8);
            packet.extend_from_slice(part);
            out(&packet);
        }
        true
    }
}

struct Open {
    id: u16,
    parts: Vec<Option<Vec<u8>>>,
    missing: usize,
}

/// Puts incoming packets back into datagrams, per peer.
#[derive(Default)]
pub struct Reassembler {
    peers: HashMap<String, VecDeque<Open>>,
    /// Packets dropped as malformed.
    pub malformed: u64,
}

impl Reassembler {
    /// One packet of `peer`; a datagram when this completes one.
    pub fn accept(&mut self, peer: &str, packet: &[u8]) -> Option<Vec<u8>> {
        match packet.first() {
            Some(&WHOLE) => Some(packet[1..].to_vec()),
            Some(&PART) if packet.len() > PART_HEADER => {
                let id = u16::from_le_bytes([packet[1], packet[2]]);
                let index = packet[3] as usize;
                let count = packet[4] as usize;
                if count == 0 || count > MAX_PARTS || index >= count {
                    self.malformed += 1;
                    return None;
                }
                self.part(peer, id, index, count, &packet[PART_HEADER..])
            }
            _ => {
                self.malformed += 1;
                None
            }
        }
    }

    fn part(&mut self, peer: &str, id: u16, index: usize, count: usize, data: &[u8]) -> Option<Vec<u8>> {
        let open = self.peers.entry(peer.to_string()).or_default();
        let at = match open.iter().position(|o| o.id == id) {
            Some(at) => at,
            None => {
                if open.len() == OPEN_PER_PEER {
                    open.pop_front();
                }
                open.push_back(Open { id, parts: vec![None; count], missing: count });
                open.len() - 1
            }
        };
        let o = &mut open[at];
        if o.parts.len() != count {
            self.malformed += 1;
            return None;
        }
        if o.parts[index].is_none() {
            o.parts[index] = Some(data.to_vec());
            o.missing -= 1;
        }
        if o.missing > 0 {
            return None;
        }
        let done = open.remove(at)?;
        let mut whole = Vec::with_capacity(done.parts.iter().flatten().map(Vec::len).sum());
        for part in done.parts.iter().flatten() {
            whole.extend_from_slice(part);
        }
        Some(whole)
    }

    /// Forgets a peer's unfinished datagrams.
    pub fn forget(&mut self, peer: &str) {
        self.peers.remove(peer);
    }

    /// Forgets everything (the session ended).
    pub fn clear(&mut self) {
        self.peers.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packets(data: &[u8]) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        assert!(Framer::default().frame(data, |p| out.push(p.to_vec())));
        out
    }

    #[test]
    fn a_datagram_that_fits_is_one_packet() {
        for len in [0, 1, 1153, MAX_PACKET - 1] {
            let data = vec![7u8; len];
            let p = packets(&data);
            assert_eq!(p.len(), 1, "{len}");
            assert!(p[0].len() <= MAX_PACKET);
            assert_eq!(Reassembler::default().accept("a", &p[0]), Some(data));
        }
    }

    #[test]
    fn every_size_up_to_the_udp_maximum_comes_back_whole() {
        for len in [MAX_PACKET, MAX_PACKET + 1, 4096, 20_000, MAX_DATAGRAM] {
            let data: Vec<u8> = (0..len).map(|i| (i * 31) as u8).collect();
            let p = packets(&data);
            assert!(p.iter().all(|x| x.len() <= MAX_PACKET), "{len}");
            let mut r = Reassembler::default();
            let mut got = None;
            for x in &p {
                assert!(got.is_none());
                got = r.accept("a", x);
            }
            assert_eq!(got.as_deref(), Some(data.as_slice()), "{len}");
        }
    }

    #[test]
    fn longer_than_a_datagram_is_refused() {
        let data = vec![0u8; MAX_DATAGRAM + 1];
        assert!(!Framer::default().frame(&data, |_| panic!("nothing is sent")));
    }

    #[test]
    fn parts_in_any_order_and_repeated() {
        let data: Vec<u8> = (0..5000).map(|i| i as u8).collect();
        let mut p = packets(&data);
        p.reverse();
        let first = p[0].clone();
        p.insert(1, first);
        let mut r = Reassembler::default();
        let got: Vec<_> = p.iter().filter_map(|x| r.accept("a", x)).collect();
        assert_eq!(got, vec![data]);
    }

    #[test]
    fn a_lost_part_loses_only_its_datagram() {
        let mut f = Framer::default();
        let mut one = Vec::new();
        let mut two = Vec::new();
        let a = vec![1u8; 3000];
        let b = vec![2u8; 3000];
        f.frame(&a, |p| one.push(p.to_vec()));
        f.frame(&b, |p| two.push(p.to_vec()));
        one.pop();
        let mut r = Reassembler::default();
        let got: Vec<_> = one.iter().chain(two.iter()).filter_map(|x| r.accept("a", x)).collect();
        assert_eq!(got, vec![b]);
    }

    #[test]
    fn peers_do_not_mix() {
        let data: Vec<u8> = (0..4000).map(|i| i as u8).collect();
        let p = packets(&data);
        let mut r = Reassembler::default();
        assert_eq!(r.accept("a", &p[0]), None);
        assert_eq!(r.accept("b", &p[1]), None);
        assert_eq!(r.accept("b", &p[2]), None);
        assert_eq!(r.accept("b", &p[3]), None);
        assert_eq!(r.accept("a", &p[1]), None);
        assert_eq!(r.accept("a", &p[2]), None);
        assert_eq!(r.accept("a", &p[3]), Some(data));
    }

    #[test]
    fn unfinished_datagrams_are_bounded_and_the_oldest_goes() {
        let mut r = Reassembler::default();
        for id in 0..1000u16 {
            let mut x = vec![PART];
            x.extend_from_slice(&id.to_le_bytes());
            x.extend_from_slice(&[0, 2, 9]);
            assert_eq!(r.accept("a", &x), None);
        }
        assert_eq!(r.peers["a"].len(), OPEN_PER_PEER);
    }

    #[test]
    fn malformed_packets_are_dropped_and_counted() {
        let mut r = Reassembler::default();
        let bad: [&[u8]; 6] = [&[], &[9, 1, 2], &[PART], &[PART, 0, 0, 0, 0, 1], &[PART, 0, 0, 5, 2, 1], &[PART, 0, 0, 0, 255, 1]];
        for x in bad {
            assert_eq!(r.accept("a", x), None, "{x:?}");
        }
        assert_eq!(r.malformed, bad.len() as u64);
        // the same number claiming another count
        assert_eq!(r.accept("a", &[PART, 7, 0, 0, 2, 1]), None);
        assert_eq!(r.accept("a", &[PART, 7, 0, 0, 3, 1]), None);
        assert_eq!(r.malformed, bad.len() as u64 + 1);
    }
}
