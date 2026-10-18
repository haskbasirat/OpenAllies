//! Native concrete Mind constructor fields, embedded state metadata and element
//! ability jitter. Padding and unwritten fields are absent, never defaulted.
use crate::{Error, Mind, MindId, Result, State, StateId};
use aa_numeric::{Binary, Precision};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Base,
    Element,
    Regiment,
    Aircraft,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbeddedState {
    pub offset: u32,
    pub name: &'static str,
    pub native_vtable: u32,
    pub native_metadata: u32,
    pub discard_on_suspend: bool,
    pub initial_status: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reference {
    EmptyUtf16,
    /// Original allocated backing store with logical length0; no entries exist.
    EmptyWordVector {
        initial_capacity: usize,
    },
    /// Evidence identifier for a nested native subobject, not a callable address.
    NativeSubobject {
        vtable: u32,
    },
}
#[derive(Clone, Debug)]
pub struct Constructed {
    pub kind: Kind,
    /// Move this core into the single runtime Mind arena; do not retain a clone.
    pub core: Mind,
    pub update_head: Option<u64>,
    pub states: Vec<EmbeddedState>,
    /// Additional initialized fields at native offsets. State status/header words
    /// belong to states/Arena rather than a second copy in this map.
    pub words: BTreeMap<u32, u32>,
    pub bytes: BTreeMap<u32, u8>,
    pub references: BTreeMap<u32, Reference>,
}
/// Authoritative remainder after moving the core and embedded states into Arena.
/// Component storage owns this value; Arena alone owns stack/status state.
#[derive(Clone, Debug)]
pub struct Installed {
    pub kind: Kind,
    pub mind_id: MindId,
    pub state_bindings: BTreeMap<u32, StateId>,
    pub update_head: Option<u64>,
    pub words: BTreeMap<u32, u32>,
    pub bytes: BTreeMap<u32, u8>,
    pub references: BTreeMap<u32, Reference>,
}
impl Constructed {
    /// Move constructed state into the actual arena. IDs are supplied by the
    /// runtime allocator. Collision checks precede all arena mutations; no RNG,
    /// constructors or lifecycle callbacks are repeated during publication.
    pub fn install(
        self,
        mind_id: MindId,
        bindings: &BTreeMap<u32, StateId>,
        arena: &mut crate::Arena,
    ) -> Result<Installed> {
        if arena.minds.contains_key(&mind_id) {
            return Err(Error::World("Mind arena identity already occupied".into()));
        }
        let states = self.bind_states(mind_id, bindings)?;
        if states.keys().any(|id| arena.states.contains_key(id)) {
            return Err(Error::World("state arena identity already occupied".into()));
        }
        let state_bindings = self
            .states
            .iter()
            .map(|s| (s.offset, bindings[&s.offset]))
            .collect();
        arena.minds.insert(mind_id, self.core);
        arena.states.extend(states);
        Ok(Installed {
            kind: self.kind,
            mind_id,
            state_bindings,
            update_head: self.update_head,
            words: self.words,
            bytes: self.bytes,
            references: self.references,
        })
    }
    pub fn word(&self, offset: u32) -> Result<u32> {
        self.words
            .get(&offset)
            .copied()
            .ok_or_else(|| Error::World(format!("Mind field {offset:#x} was not initialized")))
    }
    /// Supply real arena IDs allocated by the runtime. Native pointers and source
    /// offsets are not reused as portable identities. No arena writes occur here.
    pub fn bind_states(
        &self,
        parent: MindId,
        bindings: &BTreeMap<u32, StateId>,
    ) -> Result<BTreeMap<StateId, State>> {
        let mut used = BTreeSet::new();
        let mut result = BTreeMap::new();
        for item in &self.states {
            let id = *bindings.get(&item.offset).ok_or_else(|| {
                Error::World(format!("missing state binding at {:#x}", item.offset))
            })?;
            if !used.insert(id) {
                return Err(Error::World("duplicate constructor state identity".into()));
            }
            result.insert(
                id,
                State {
                    parent,
                    discard_on_suspend: item.discard_on_suspend,
                    status: item.initial_status,
                },
            );
        }
        Ok(result)
    }
}
pub trait World {
    /// Actual shared native world RNG draw; called exactly once for Element only.
    fn random_word(&mut self) -> Result<u32>;
    fn performer_ability_count(&self, actor: u32) -> Result<usize>;
    /// Writes real ability+1C. Errors retain the RNG draw and previous writes.
    fn set_ability_jitter(&mut self, actor: u32, index: usize, bits: u32) -> Result<()>;
}
fn numeric<T>(value: aa_numeric::Result<T>) -> Result<T> {
    value.map_err(|e| Error::World(format!("Mind constructor arithmetic: {e}")))
}
pub fn construct(
    kind: Kind,
    actor: u32,
    world: &mut impl World,
    precision: Precision,
) -> Result<Constructed> {
    let (states, mut words, bytes, references) = template(kind);
    if kind == Kind::Element {
        let draw = world.random_word()?;
        let mut number = Binary::from_i32(draw as i32);
        if draw & 0x80000000 != 0 {
            number = numeric(number.add(
                &numeric(Binary::from_f64_bits(0x41f0000000000000))?,
                precision,
            ))?;
        }
        let unit = numeric(number.mul(
            &numeric(Binary::from_f64_bits(0x3df0000000100000))?,
            precision,
        ))?;
        let twice = numeric(unit.add(&unit, precision))?;
        let jitter = numeric(numeric(twice.sub(&Binary::from_i32(1), precision))?.store_f32())?;
        words.insert(0x1b8, jitter);
        let mut index = 0;
        while index < world.performer_ability_count(actor)? {
            world.set_ability_jitter(actor, index, jitter)?;
            index += 1;
        }
    }
    Ok(Constructed {
        kind,
        core: Mind::base(actor),
        update_head: None,
        states,
        words,
        bytes,
        references,
    })
}

type Template = (
    Vec<EmbeddedState>,
    BTreeMap<u32, u32>,
    BTreeMap<u32, u8>,
    BTreeMap<u32, Reference>,
);
// The following constants are reconstructed original constructor stores and
// registered state metadata, verified against every native constructor fixture.

fn template(kind: Kind) -> Template {
    match kind {
        Kind::Base => (
            vec![],
            [].into_iter().collect(),
            [].into_iter().collect(),
            [].into_iter().collect(),
        ),
        Kind::Element => (
            vec![
                EmbeddedState {
                    offset: 0x34,
                    name: "uai_unload",
                    native_vtable: 0xa34774,
                    native_metadata: 0xb3d308,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x48,
                    name: "uai_deploy",
                    native_vtable: 0xa348bc,
                    native_metadata: 0xb3d320,
                    discard_on_suspend: true,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x68,
                    name: "uai_sleep",
                    native_vtable: 0xa34798,
                    native_metadata: 0xb3d338,
                    discard_on_suspend: true,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x7c,
                    name: "uai_move",
                    native_vtable: 0xa354a4,
                    native_metadata: 0xb3d350,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0xf8,
                    name: "uai_kill",
                    native_vtable: 0xa34af0,
                    native_metadata: 0xb3d368,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x10c,
                    name: "uai_bombard",
                    native_vtable: 0xa35480,
                    native_metadata: 0xb3d380,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x12c,
                    name: "uai_engage",
                    native_vtable: 0xa34b14,
                    native_metadata: 0xb3d3b0,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x148,
                    name: "uai_repair",
                    native_vtable: 0xa34b68,
                    native_metadata: 0xb3d3c8,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x164,
                    name: "uai_join",
                    native_vtable: 0xa34b8c,
                    native_metadata: 0xb3d3e0,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x178,
                    name: "uai_reattach",
                    native_vtable: 0xa347bc,
                    native_metadata: 0xb3d3f8,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x188,
                    name: "uai_unpack",
                    native_vtable: 0xa347e0,
                    native_metadata: 0xb3d410,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
            ],
            [
                (0x44, 0x00000000),
                (0x58, 0x00000000),
                (0x5c, 0x00000000),
                (0x60, 0x00000000),
                (0x64, 0x00000000),
                (0x78, 0x00000000),
                (0x8c, 0x00000000),
                (0x90, 0x00000000),
                (0x94, 0x00000000),
                (0x98, 0x00000000),
                (0x9c, 0xbf800000),
                (0xa0, 0x00000000),
                (0xa4, 0x00000000),
                (0xa8, 0x00000000),
                (0xac, 0x00000000),
                (0xb0, 0x00000000),
                (0xb4, 0x00000000),
                (0xb8, 0x00000000),
                (0xbc, 0x00000000),
                (0xc0, 0x00000000),
                (0xc4, 0x00000000),
                (0xc8, 0x00000000),
                (0xcc, 0xbf800000),
                (0xd0, 0x00000000),
                (0xd8, 0x00000000),
                (0xdc, 0x00000000),
                (0xe0, 0x00000000),
                (0xe4, 0x00000000),
                (0xec, 0xffffffff),
                (0xf0, 0x00000000),
                (0xf4, 0x00000000),
                (0x108, 0x00000000),
                (0x11c, 0x00000000),
                (0x124, 0x00000000),
                (0x128, 0x00000000),
                (0x13c, 0x00000000),
                (0x140, 0x00000003),
                (0x15c, 0x00000000),
                (0x174, 0x00000000),
                (0x198, 0x00000000),
                (0x19c, 0x00000000),
                (0x1a0, 0x00000000),
                (0x1a4, 0xbf800000),
                (0x1a8, 0x00000001),
                (0x1ac, 0x00000000),
                (0x1b0, 0x00000000),
                (0x1b4, 0x00000000),
            ]
            .into_iter()
            .collect(),
            [(0xe8, 1), (0x144, 0), (0x158, 0), (0x160, 0)]
                .into_iter()
                .collect(),
            [
                (0xd4, Reference::NativeSubobject { vtable: 0xa34738 }),
                (0x120, Reference::EmptyUtf16),
            ]
            .into_iter()
            .collect(),
        ),
        Kind::Regiment => (
            vec![
                EmbeddedState {
                    offset: 0x34,
                    name: "cai_sleep",
                    native_vtable: 0xa329f0,
                    native_metadata: 0xb3cf8c,
                    discard_on_suspend: true,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x54,
                    name: "cai_rampage",
                    native_vtable: 0xa32a14,
                    native_metadata: 0xb3cfa4,
                    discard_on_suspend: true,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x68,
                    name: "cai_move",
                    native_vtable: 0xa33804,
                    native_metadata: 0xb3cfbc,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0xc0,
                    name: "cai_kill",
                    native_vtable: 0xa32b5c,
                    native_metadata: 0xb3cfd4,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0xd4,
                    name: "cai_bombard",
                    native_vtable: 0xa3348c,
                    native_metadata: 0xb3cfec,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0xf8,
                    name: "cai_siege",
                    native_vtable: 0xa32cc8,
                    native_metadata: 0xb3d004,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x120,
                    name: "cai_chasing",
                    native_vtable: 0xa32b80,
                    native_metadata: 0xb3d01c,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x134,
                    name: "cai_construct",
                    native_vtable: 0xa32a38,
                    native_metadata: 0xb3d034,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x160,
                    name: "cai_repair",
                    native_vtable: 0xa32ba4,
                    native_metadata: 0xb3d04c,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x17c,
                    name: "cai_guard",
                    native_vtable: 0xa32bc8,
                    native_metadata: 0xb3d064,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x1a8,
                    name: "cai_regroup",
                    native_vtable: 0xa32a5c,
                    native_metadata: 0xb3d07c,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x1b8,
                    name: "cai_reform",
                    native_vtable: 0xa32a80,
                    native_metadata: 0xb3d094,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x1cc,
                    name: "cai_engage",
                    native_vtable: 0xa33140,
                    native_metadata: 0xb3d0ac,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x1f0,
                    name: "cai_retreat",
                    native_vtable: 0xa32aa4,
                    native_metadata: 0xb3d0c4,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x20c,
                    name: "cai_rout",
                    native_vtable: 0xa32ac8,
                    native_metadata: 0xb3d0dc,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x220,
                    name: "cai_explore",
                    native_vtable: 0xa32bec,
                    native_metadata: 0xb3d0f4,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x24c,
                    name: "cai_join",
                    native_vtable: 0xa32ec4,
                    native_metadata: 0xb3d10c,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x270,
                    name: "cai_exhausted",
                    native_vtable: 0xa32aec,
                    native_metadata: 0xb3d124,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x280,
                    name: "cai_unpack",
                    native_vtable: 0xa32b10,
                    native_metadata: 0xb3d13c,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x2a4,
                    name: "cai_pack",
                    native_vtable: 0xa32b34,
                    native_metadata: 0xb3d154,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
            ],
            [
                (0x48, 0x00000000),
                (0x50, 0x00000000),
                (0x78, 0x00000000),
                (0x7c, 0x00000000),
                (0x80, 0x00000000),
                (0x84, 0x00000000),
                (0x88, 0x00000000),
                (0x8c, 0x00000000),
                (0x90, 0xbf800000),
                (0x94, 0x00000000),
                (0x98, 0x00000000),
                (0xa0, 0x00000000),
                (0xa4, 0x00000000),
                (0xa8, 0x00000000),
                (0xac, 0x00000000),
                (0xb4, 0xffffffff),
                (0xb8, 0x00000000),
                (0xbc, 0x00000000),
                (0xd0, 0x00000000),
                (0xe4, 0x00000000),
                (0xec, 0x00000000),
                (0xf0, 0x00000000),
                (0x108, 0x00000000),
                (0x10c, 0x00000000),
                (0x110, 0x00000000),
                (0x114, 0x00000000),
                (0x11c, 0x00000000),
                (0x130, 0x00000000),
                (0x148, 0x00000000),
                (0x14c, 0x00000000),
                (0x150, 0x00000000),
                (0x154, 0x00000000),
                (0x158, 0xbf800000),
                (0x15c, 0xffffffff),
                (0x174, 0x00000000),
                (0x18c, 0x00000000),
                (0x194, 0x00000000),
                (0x198, 0x00000000),
                (0x19c, 0x00000000),
                (0x1a0, 0x00000000),
                (0x1a4, 0xbf800000),
                (0x1c8, 0x00000000),
                (0x1dc, 0x00000000),
                (0x1e0, 0x00000000),
                (0x1e4, 0x00000000),
                (0x1ec, 0x00000000),
                (0x200, 0x00000000),
                (0x204, 0x00000000),
                (0x208, 0x00000000),
                (0x21c, 0x00000000),
                (0x230, 0x00000000),
                (0x234, 0x00000000),
                (0x238, 0x00000000),
                (0x23c, 0x00000000),
                (0x240, 0xc61c4000),
                (0x244, 0x00000000),
                (0x248, 0x00000001),
                (0x25c, 0x00000000),
                (0x268, 0x00000000),
                (0x26c, 0x00000001),
                (0x290, 0x00000000),
                (0x294, 0x00000000),
                (0x298, 0x00000000),
                (0x29c, 0xbf800000),
                (0x2a0, 0x00000000),
                (0x2b4, 0x00000000),
                (0x2b8, 0x00000000),
                (0x2bc, 0x00000000),
                (0x2c0, 0x00000000),
            ]
            .into_iter()
            .collect(),
            [
                (0x44, 0),
                (0x4c, 0),
                (0x64, 0),
                (0xb0, 1),
                (0xf4, 1),
                (0x118, 1),
                (0x119, 1),
                (0x144, 0),
                (0x170, 0),
                (0x178, 0),
                (0x190, 0),
                (0x1e8, 0),
                (0x260, 0),
                (0x261, 0),
            ]
            .into_iter()
            .collect(),
            [
                (0x9c, Reference::NativeSubobject { vtable: 0xa34738 }),
                (0xe8, Reference::EmptyUtf16),
                (
                    0x264,
                    Reference::EmptyWordVector {
                        initial_capacity: 1,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        ),
        Kind::Aircraft => (
            vec![
                EmbeddedState {
                    offset: 0x34,
                    name: "aai_move",
                    native_vtable: 0xa326c0,
                    native_metadata: 0xb3ce68,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x9c,
                    name: "aai_guard",
                    native_vtable: 0xa324b4,
                    native_metadata: 0xb3ce80,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0xb4,
                    name: "aai_recon",
                    native_vtable: 0xa324d8,
                    native_metadata: 0xb3ce98,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0xd0,
                    name: "aai_escort",
                    native_vtable: 0xa3266c,
                    native_metadata: 0xb3ceb0,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0xe8,
                    name: "aai_attack",
                    native_vtable: 0xa325dc,
                    native_metadata: 0xb3cec8,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x12c,
                    name: "aai_join",
                    native_vtable: 0xa32600,
                    native_metadata: 0xb3cee0,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x15c,
                    name: "aai_takeoff",
                    native_vtable: 0xa32624,
                    native_metadata: 0xb3cef8,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x174,
                    name: "aai_landing",
                    native_vtable: 0xa32648,
                    native_metadata: 0xb3cf10,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x188,
                    name: "aai_queued",
                    native_vtable: 0xa3246c,
                    native_metadata: 0xb3cf28,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
                EmbeddedState {
                    offset: 0x198,
                    name: "aai_transport",
                    native_vtable: 0xa32490,
                    native_metadata: 0xb3cf40,
                    discard_on_suspend: false,
                    initial_status: 0,
                },
            ],
            [
                (0x48, 0x00000000),
                (0x4c, 0x00000000),
                (0x50, 0x00000000),
                (0x54, 0x00000000),
                (0x58, 0x00000000),
                (0x5c, 0xbf800000),
                (0x60, 0x00000000),
                (0x68, 0x00000000),
                (0x6c, 0x00000000),
                (0x70, 0x00000000),
                (0x74, 0x00000000),
                (0x78, 0x00000000),
                (0x84, 0x00000000),
                (0x88, 0x00000000),
                (0x8c, 0x00000000),
                (0x90, 0x00000000),
                (0x98, 0xffffffff),
                (0xac, 0x00000000),
                (0xb0, 0x00000000),
                (0xc8, 0x00000000),
                (0xcc, 0x00000000),
                (0xe0, 0x00000000),
                (0xe4, 0x00000000),
                (0xf8, 0x00000000),
                (0x100, 0x00000000),
                (0x104, 0x00000000),
                (0x10c, 0x00000000),
                (0x110, 0x00000000),
                (0x114, 0x00000000),
                (0x118, 0x00000000),
                (0x11c, 0x00000001),
                (0x120, 0x00000000),
                (0x124, 0xc2c80000),
                (0x128, 0xc2c80000),
                (0x13c, 0x00000000),
                (0x140, 0x00000000),
                (0x144, 0x00000000),
                (0x148, 0x00000000),
                (0x14c, 0x00000000),
                (0x150, 0x00000000),
                (0x154, 0x00000000),
                (0x170, 0x00000000),
                (0x184, 0x00000000),
                (0x1ac, 0x00000000),
                (0x1b0, 0x00000000),
                (0x1b4, 0x00000000),
                (0x1b8, 0x00000000),
                (0x1bc, 0x00000000),
                (0x1c0, 0x00000000),
                (0x1c4, 0xbf800000),
                (0x1c8, 0x00000000),
                (0x1cc, 0x00000000),
                (0x1d4, 0x00000000),
            ]
            .into_iter()
            .collect(),
            [
                (0x44, 0),
                (0x64, 0),
                (0x7c, 0),
                (0x94, 1),
                (0xc4, 0),
                (0xfc, 0),
                (0x108, 0),
                (0x158, 0),
                (0x16c, 0),
                (0x1a8, 0),
                (0x1a9, 0),
                (0x1d0, 0),
            ]
            .into_iter()
            .collect(),
            [(0x80, Reference::NativeSubobject { vtable: 0xa34738 })]
                .into_iter()
                .collect(),
        ),
    }
}
