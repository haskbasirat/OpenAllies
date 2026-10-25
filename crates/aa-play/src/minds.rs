//! aa-mind's native Mind arena hosted for aa-play actors (core owner).
//!
//! A hosted Mind is the concrete Mind 0x4CF480 case 4 builds
//! (`aa_mind::construct`): its core (state stack, pending and current
//! commands) lives in `Minds::arena`, and its embedded states are bound to
//! arena ids. Its native memory is kept by Mind offset: the constructor's
//! words and bytes, then whatever the hosted Thinks write. An embedded
//! state's field +k is Mind offset + k. The Mind's Updatable (Mind+8)
//! schedules on queue 4 (`crate::scheduler::THINK`; 0x50AD80 cancels the
//! pending think, then adds one). Commands handed to a hosted Mind
//! (0x4D9B60) are kept in `Minds::commands` with their reference counts.
//!
//! The aircraft Mind (AAI.cpp, `crate::combat::air`) is the first hosted
//! kind. Element and regiment Minds are still aa-play's own (movement and
//! combat).
use crate::world::ActorId;
use aa_mind::{Arena, MindId, StateId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The concrete Mind class of a hosted Mind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostedKind {
    /// Vtable 0xA32710 (AAI.cpp).
    Aircraft,
}

/// One hosted Mind's binding and native memory.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hosted {
    pub kind: HostedKind,
    /// Embedded state at each Mind offset.
    pub states: BTreeMap<u32, StateId>,
    /// Mind words and bytes by offset (absent fields read 0).
    pub words: BTreeMap<u32, u32>,
    pub bytes: BTreeMap<u32, u8>,
}

/// A command object (the 0x4D9E20 context a hosted Mind keeps).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HostedCommand {
    /// The predefined actor command index (aa-rules PREDEFINED_ACTOR_COMMANDS;
    /// command+8 is that definition).
    pub command: u32,
    /// +0xC flags: 1 target actor, 4 point.
    pub flags: u32,
    /// +0x10 target GTID; +0x18 / +0x1C point (binary32).
    pub target: u32,
    pub point: [u32; 2],
    /// +0x3C trailing (queued) byte.
    pub trailing: bool,
    /// +4 reference count.
    pub refs: u32,
}

/// Every hosted Mind (`Books::minds`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Minds {
    pub arena: Arena,
    pub hosted: BTreeMap<ActorId, Hosted>,
    /// State id -> (actor, Mind offset).
    pub owners: BTreeMap<StateId, (ActorId, u32)>,
    pub next_state: u32,
    pub commands: BTreeMap<u32, HostedCommand>,
    pub next_command: u32,
    /// World service errors met by hosted Minds (kept for diagnostics, at
    /// most 64).
    pub notes: Vec<String>,
    /// [B3CE5C]: the aircraft Mind default (0x517D00) is running.
    #[serde(default)]
    pub resetting: bool,
}

/// The arena id of `a`'s Mind.
pub fn mind_id(a: ActorId) -> MindId {
    MindId(a)
}

/// Mind +0x24 / +0x28 / +0x2C / +0x30: `Mind::words_24_30`.
pub fn core_word(offset: u32) -> Option<usize> {
    match offset {
        0x24 | 0x28 | 0x2c | 0x30 => Some(((offset - 0x24) / 4) as usize),
        _ => None,
    }
}

impl Minds {
    pub fn is_hosted(&self, a: ActorId, kind: HostedKind) -> bool {
        self.hosted.get(&a).is_some_and(|h| h.kind == kind)
    }
    /// Mind word at `offset`. +0x24..+0x30 live in the arena core
    /// (`Mind::words_24_30`, which the aa-mind Thinks read).
    pub fn word(&self, a: ActorId, offset: u32) -> u32 {
        if let Some(i) = core_word(offset) {
            return self.arena.minds.get(&mind_id(a)).map_or(0, |m| m.words_24_30[i]);
        }
        self.hosted.get(&a).and_then(|h| h.words.get(&offset)).copied().unwrap_or(0)
    }
    pub fn set_word(&mut self, a: ActorId, offset: u32, bits: u32) {
        if let Some(i) = core_word(offset) {
            if let Some(m) = self.arena.minds.get_mut(&mind_id(a)) {
                m.words_24_30[i] = bits;
            }
            return;
        }
        if let Some(h) = self.hosted.get_mut(&a) {
            h.words.insert(offset, bits);
        }
    }
    pub fn byte(&self, a: ActorId, offset: u32) -> u8 {
        self.hosted.get(&a).and_then(|h| h.bytes.get(&offset)).copied().unwrap_or(0)
    }
    pub fn set_byte(&mut self, a: ActorId, offset: u32, value: u8) {
        if let Some(h) = self.hosted.get_mut(&a) {
            h.bytes.insert(offset, value);
        }
    }
    /// The embedded state at Mind+`offset`.
    pub fn state(&self, a: ActorId, offset: u32) -> Option<StateId> {
        self.hosted.get(&a).and_then(|h| h.states.get(&offset)).copied()
    }
    /// The Mind and offset of a state.
    pub fn owner(&self, state: StateId) -> Option<(ActorId, u32)> {
        self.owners.get(&state).copied()
    }
    /// The offset of the state on top of `a`'s stack.
    pub fn top_offset(&self, a: ActorId) -> Option<u32> {
        let top = self.arena.minds.get(&mind_id(a))?.stack.front().copied().flatten()?;
        self.owner(top).map(|(_, o)| o)
    }
    pub fn note(&mut self, what: impl std::fmt::Display) {
        if self.notes.len() < 64 {
            self.notes.push(what.to_string());
        }
    }

    /// Install a constructed Mind for `a` (0x4CF480 case 4).
    pub fn install(&mut self, a: ActorId, kind: HostedKind, built: aa_mind::construct::Constructed) -> Result<(), String> {
        let mut bindings = BTreeMap::new();
        for s in &built.states {
            self.next_state += 1;
            bindings.insert(s.offset, StateId(self.next_state));
        }
        let installed = built.install(mind_id(a), &bindings, &mut self.arena).map_err(|e| e.to_string())?;
        for (&offset, &id) in &installed.state_bindings {
            self.owners.insert(id, (a, offset));
        }
        self.hosted.insert(a, Hosted { kind, states: installed.state_bindings, words: installed.words, bytes: installed.bytes });
        Ok(())
    }

    /// A command object for `command` (0x4D9E20 + the target / point).
    pub fn new_command(&mut self, command: u32, target: Option<u32>, point: Option<[u32; 2]>) -> u32 {
        self.next_command += 1;
        let id = self.next_command;
        let mut flags = 0;
        if target.is_some() {
            flags |= 1;
        }
        if point.is_some() {
            flags |= 4;
        }
        self.commands.insert(id, HostedCommand { command, flags, target: target.unwrap_or(0), point: point.unwrap_or([0; 2]), trailing: false, refs: 0 });
        id
    }
}
