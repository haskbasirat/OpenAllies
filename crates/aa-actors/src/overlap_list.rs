//! The Actor-owned +110/+114 overlap list. Constructor nulls and native node
//! lifetime have one typed owner, moved with ActorShell through staging.
//! Handles are local allocation identities, never native pointer values.
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId(usize);
impl NodeId {
    pub fn raw(self) -> usize {
        self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub reference: u32,
    pub expiry_bits: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Node {
    pub entry: Entry,
    pub next: Option<NodeId>,
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("overlap list allocation identity exhausted")]
    Exhausted,
    #[error("overlap list node is not currently owned")]
    Missing,
    #[error("overlap list internal link is invalid")]
    Link,
}
pub type Result<T> = std::result::Result<T, Error>;

/// There is no inline/native twenty-unit cap. Clone supports the existing
/// diagnostic shell snapshots; live handoffs move this owner, never clone it.
#[derive(Clone, Debug)]
pub struct State {
    head: Option<NodeId>,
    tail: Option<NodeId>,
    next_id: usize,
    nodes: BTreeMap<NodeId, Node>,
}
impl Default for State {
    fn default() -> Self {
        Self::constructed()
    }
}
impl State {
    /// The actual 4D6D70 initialized null head/tail; not an inferred world list.
    pub fn constructed() -> Self {
        Self {
            head: None,
            tail: None,
            next_id: 1,
            nodes: BTreeMap::new(),
        }
    }
    pub fn head(&self) -> Option<NodeId> {
        self.head
    }
    pub fn tail(&self) -> Option<NodeId> {
        self.tail
    }
    pub fn allocated_nodes(&self) -> usize {
        self.nodes.len()
    }
    pub fn node(&self, id: NodeId) -> Result<&Node> {
        self.nodes.get(&id).ok_or(Error::Missing)
    }
    pub fn node_by_raw(&self, raw: usize) -> Result<&Node> {
        self.node(NodeId(raw))
    }
    /// Successful native allocation path of 4D68C2..4D68FE. Duplicate GTIDs
    /// are legitimate separate nodes; the original does not deduplicate here.
    pub fn prepend(&mut self, entry: Entry) -> Result<NodeId> {
        let id = NodeId(self.next_id);
        let next_id = self.next_id.checked_add(1).ok_or(Error::Exhausted)?;
        let node = Node {
            entry,
            next: self.head,
        };
        self.nodes.insert(id, node);
        self.next_id = next_id;
        self.head = Some(id);
        if self.tail.is_none() {
            self.tail = Some(id);
        }
        Ok(id)
    }
    pub fn remove(&mut self, id: NodeId) -> Result<Entry> {
        let mut prior = None;
        let mut current = self.head;
        let mut remaining = self.nodes.len();
        while let Some(key) = current {
            if remaining == 0 {
                return Err(Error::Link);
            }
            remaining -= 1;
            let node = *self.node(key)?;
            if key == id {
                if let Some(prior) = prior {
                    self.nodes.get_mut(&prior).ok_or(Error::Link)?.next = node.next;
                } else {
                    self.head = node.next;
                }
                if self.tail == Some(id) {
                    self.tail = prior;
                }
                self.nodes.remove(&id).ok_or(Error::Missing)?;
                return Ok(node.entry);
            }
            prior = Some(key);
            current = node.next;
        }
        Err(Error::Missing)
    }
    /// Native FCOMP/test-AH5/JP removes every entry not ordered strictly after
    /// current time. Integer bit ordering preserves NaN/infinity/subnormals and
    /// signed zero without depending on the host SIMD flush/denormal mode.
    pub fn expire_at(&mut self, world_time_bits: u32) -> Result<usize> {
        let mut current = self.head;
        let mut removed = 0;
        while let Some(id) = current {
            let node = *self.node(id)?;
            current = node.next;
            if !ordered_less(world_time_bits, node.entry.expiry_bits) {
                self.remove(id)?;
                removed += 1;
            }
        }
        Ok(removed)
    }
}

/// Exact ordered binary32 comparison. Native x87 compares represented values;
/// no arithmetic rounding takes place. NaNs are unordered, signed zeros equal.
fn ordered_less(left: u32, right: u32) -> bool {
    let a = left & 0x7fff_ffff;
    let b = right & 0x7fff_ffff;
    if a > 0x7f80_0000 || b > 0x7f80_0000 || (a == 0 && b == 0) {
        return false;
    }
    match (left >> 31, right >> 31) {
        (0, 0) => left < right,
        (1, 1) => left > right,
        (1, 0) => true,
        _ => false,
    }
}

/// Actual4D10A0 reads the authoritative low/high actor-state owner. No raw
/// shell-word mirror or component-presence guess is part of this predicate.
pub fn eligible(state_low: u32, state_high: u32) -> bool {
    state_low & 0x0600_0000 == 0 && state_high & 0x14 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_native_expiry_prepend_and_eligibility_match_both_images_all_pc() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-live-map-startup/native-actor-overlap-list.json"
        ))
        .unwrap();
        let cases = fixture["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 342);
        let word = |v: &serde_json::Value| v.as_u64().unwrap() as u32;
        let denormal: serde_json::Value=serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-live-map-startup/native-actor-overlap-denormals.json"
        )).unwrap();
        assert_eq!(denormal["cases"].as_array().unwrap().len(), 48);
        for case in cases.iter().chain(denormal["cases"].as_array().unwrap()) {
            let input = &case["input"];
            let mut state = State::constructed();
            for e in input["entries"].as_array().unwrap().iter().rev() {
                state
                    .prepend(Entry {
                        reference: word(&e[0]),
                        expiry_bits: word(&e[1]),
                    })
                    .unwrap();
            }
            let flags = word(&input["flags"]);
            let active = flags & (1 << 29) == 0
                && eligible(word(&input["state_low"]), word(&input["state_high"]))
                && flags & (1 << 10) != 0;
            if active {
                state.expire_at(word(&input["time_bits"])).unwrap();
                if !input["stop_before_query"].as_bool().unwrap() {
                    for reference in input["discovered"].as_array().unwrap() {
                        state
                            .prepend(Entry {
                                reference: word(reference),
                                expiry_bits: 0,
                            })
                            .unwrap();
                    }
                }
            }
            let mut out = vec![];
            let mut node = state.head();
            let mut last = None;
            while let Some(id) = node {
                let n = state.node(id).unwrap();
                out.push(vec![n.entry.reference, n.entry.expiry_bits]);
                last = Some(id);
                node = n.next;
            }
            assert_eq!(last, state.tail());
            let expected = case["output"]["entries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| vec![word(&e[0]), word(&e[1])])
                .collect::<Vec<_>>();
            assert_eq!(out, expected, "{case}");
            assert_eq!(state.allocated_nodes(), out.len());
            assert_eq!(
                state.head().is_none(),
                case["output"]["head_null"].as_bool().unwrap()
            );
            assert_eq!(
                state.tail().is_none(),
                case["output"]["tail_null"].as_bool().unwrap()
            );
        }
    }

    #[test]
    fn node_identity_lifetime_and_duplicate_references_are_owned_without_small_cap() {
        let mut state = State::constructed();
        let mut ids = vec![];
        for _ in 0..1024 {
            ids.push(
                state
                    .prepend(Entry {
                        reference: 7,
                        expiry_bits: 0,
                    })
                    .unwrap(),
            );
        }
        assert_eq!(state.allocated_nodes(), 1024);
        assert_eq!(state.tail(), Some(ids[0]));
        let middle = ids[400];
        assert_eq!(state.remove(middle).unwrap().reference, 7);
        assert!(state.node(middle).is_err());
        assert_eq!(state.expire_at(0).unwrap(), 1023);
        assert_eq!(state.head(), None);
        assert_eq!(state.tail(), None);
        let next = state
            .prepend(Entry {
                reference: 7,
                expiry_bits: 0,
            })
            .unwrap();
        assert!(next > ids[1023]);
    }
}
