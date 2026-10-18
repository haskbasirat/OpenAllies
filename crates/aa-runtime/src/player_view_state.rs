//! Sole owned Player+50 payload constructed by4ED6D0 and retired by4ED210.
//! The same selection List moves with this owner. Native+C is unwritten;
//! wildcard-group strings are resolved at the reached constructor call, never
//! taken from a profile label or replaced with an empty localization result.
use crate::player_selection::List;
use aa_actors::ActorId;

pub const WILDCARD_KEY: &str = "static_group_wildcard_name";

/// Native+20 is constructor-null. A positive group needs its actual producer;
/// there is no public constructor from a presence boolean or arbitrary handle.
#[derive(Debug)]
pub struct CurrentGroup {
    target: Option<PrivateGroupHandle>,
}
#[derive(Debug)]
struct PrivateGroupHandle;
impl CurrentGroup {
    pub fn is_null(&self) -> bool {
        self.target.is_none()
    }
    pub fn clear(&mut self) {
        self.target = None;
    }
}

///4ED160's actual0x24-byte group. The member vector has count0/capacity1 and
/// owns the initial null slot written by4E1020->4EFB10. Its allocation capacity
/// is storage metadata, never a selection/gameplay limit. Unknown byte padding
/// at19..1B/21..23 is deliberately not represented as initialized data.
#[derive(Debug)]
pub struct StaticGroup {
    key: Vec<u16>,
    title: Vec<u16>,
    text: Vec<u16>,
    members: Vec<ActorId>,
    initial_null_slot: Option<ActorId>,
    capacity_hint: usize,
    flag_18: bool,
    word_1c: u32,
    flag_20: bool,
}
impl StaticGroup {
    pub fn key(&self) -> &[u16] {
        &self.key
    }
    pub fn title(&self) -> &[u16] {
        &self.title
    }
    pub fn text(&self) -> &[u16] {
        &self.text
    }
    pub fn members(&self) -> &[ActorId] {
        &self.members
    }
    pub fn capacity_hint(&self) -> usize {
        self.capacity_hint
    }
    pub fn initial_null_slot(&self) -> Option<ActorId> {
        self.initial_null_slot
    }
    pub fn flag_18(&self) -> bool {
        self.flag_18
    }
    pub fn word_1c(&self) -> u32 {
        self.word_1c
    }
    pub fn flag_20(&self) -> bool {
        self.flag_20
    }
    fn construct(title: Vec<u16>) -> Result<Self, String> {
        let mut members = Vec::new();
        members
            .try_reserve_exact(1)
            .map_err(|_| "4E1020 static-group member allocation failed")?;
        Ok(Self {
            key: Vec::new(),
            title,
            text: Vec::new(),
            members,
            initial_null_slot: None,
            capacity_hint: 1,
            flag_18: false,
            word_1c: 0,
            flag_20: false,
        })
    }
    fn destroy(&mut self) -> Result<(), String> {
        //4E28D0 would resolve each nonempty member and call4CE850. There is no
        //positive-member producer in this owner yet, so never fabricate that
        //service if a later producer adds members without binding retirement.
        if !self.members.is_empty() {
            return Err(
                "4E28D0 static-group member removal requires its actual Actor4CE850 owner".into(),
            );
        }
        self.flag_18 = true;
        self.word_1c = 0;
        self.members = Vec::new();
        self.capacity_hint = 0;
        self.initial_null_slot = None;
        //The native string references release in text,title,key order. Rust
        //owns the corresponding UTF16 contents; no independently mutable copy.
        self.text.clear();
        self.title.clear();
        self.key.clear();
        Ok(())
    }
}

/// Actual sibling fields lent by the Session's owning Player. These remain
/// writable for the existing selection/decree kernels; there is one List.
#[derive(Debug)]
pub struct State {
    pub kingdom: Option<ActorId>,
    word_0c: Option<u32>,
    pub mask_words_10_1c: [u32; 4],
    pub current_group: CurrentGroup,
    pub look_at_28_2c: [u32; 2],
    pub selected: List,
    wildcard: Option<StaticGroup>,
}
impl State {
    /// Successful native allocation domain of4ED6D0. The resolver is called
    /// exactly once with the actual key, before4ED160's member allocation.
    /// Allocation-null/SEH-unwind parity is outside this constructor domain;
    /// an unavailable source or allocation is an error, never a null success.
    pub fn construct(
        kingdom: Option<ActorId>,
        resolve: impl FnOnce(&[u16]) -> Result<Vec<u16>, String>,
    ) -> Result<Self, String> {
        let mut value = Self {
            kingdom,
            word_0c: None,
            mask_words_10_1c: [0; 4],
            current_group: CurrentGroup { target: None },
            look_at_28_2c: [0; 2],
            selected: List::from_vworld_constructor(),
            wildcard: None,
        };
        let key: Vec<u16> = WILDCARD_KEY.encode_utf16().collect();
        let title = resolve(&key)?;
        value.wildcard = Some(StaticGroup::construct(title)?);
        Ok(value)
    }
    pub fn word_0c(&self) -> Option<u32> {
        self.word_0c
    }
    pub fn wildcard(&self) -> Option<&StaticGroup> {
        self.wildcard.as_ref()
    }
    ///4ED210: group destruction precedes selection-node retirement. Unlike
    ///4ED0D0, this does NOT deselect actors or change their Object4C bytes.
    ///An error leaves every earlier mutation in this same owner observable.
    pub fn destroy(&mut self) -> Result<(), String> {
        if let Some(group) = self.wildcard.as_mut() {
            group.destroy()?;
            self.wildcard = None;
        }
        while self.selected.head().is_some() {
            self.selected.retire_head_for_view_destructor()?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "player_view_state_tests.rs"]
mod tests;
