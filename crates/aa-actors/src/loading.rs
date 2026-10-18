//! Separate native load constructors. Saved fields are applied afterward by the
//! stream decoder; no creation RNG, facing validation or current-time default.
use crate::components::{Components, StateBits};
use std::collections::BTreeMap;

/// Sole Actor+148/+14C/+150 vector owner. Native4D6D70 reserves capacity9
/// without allocating its data;4D69A0 materializes nine null slots on first use.
/// Rust allocation capacity is deliberately not exposed as a native field.
#[derive(Clone, Debug)]
pub struct ActorZoneVector {
    zones: aa_zones::component::ActorZones,
    capacity_150: u32,
}
impl Default for ActorZoneVector {
    fn default() -> Self {
        Self {
            zones: aa_zones::component::ActorZones::default(),
            capacity_150: 9,
        }
    }
}
impl ActorZoneVector {
    pub fn zones(&self) -> &aa_zones::component::ActorZones {
        &self.zones
    }
    pub fn zones_mut(&mut self) -> &mut aa_zones::component::ActorZones {
        &mut self.zones
    }
    pub fn native_capacity(&self) -> u32 {
        self.capacity_150
    }
    pub fn len(&self) -> usize {
        self.zones.slots.len()
    }
    pub fn is_empty(&self) -> bool {
        self.zones.slots.is_empty()
    }
    ///4D1B80's null-zone destruction branch releases this exact allocation.
    /// A positive Zone needs its actual virtual destructor before its slot clears.
    pub fn release_null_for_destructor(&mut self)->Result<(), &'static str> {
        if self.zones.slots.iter().any(Option::is_some) {
            return Err("4D1B80 positive Zone destructor is required");
        }
        self.zones.slots.clear();
        self.capacity_150=0;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadKind {
    Thing,
    Object,
    Actor,
}

#[derive(Clone, Debug)]
pub struct ThingShell<D> {
    pub definition: D,
    pub flags: u32,
    pub word_c: u32,
    pub word_10: u32,
    pub name_utf16: Vec<u16>,
    /// 4EF020 leaves this uninitialized; virtual Load supplies it. Do not
    /// substitute current world time or zero before decoding the body.
    pub word_1c: Option<u32>,
    pub position_bits: [u32; 3],
    pub facing_bits: [u32; 3],
}
#[derive(Clone, Debug)]
pub struct ObjectShell {
    pub associated_handle: u32,
    pub scale_bits: u32,
    pub variant: i32,
    pub word_44: u32,
    pub word_48: u32,
    pub byte_4c: u8,
    pub words_50_to_54: [u32; 2],
    ///Sole authoritative Object+58/+5C allocation; moved into live storage.
    pub memory: crate::detection_memory::State,
}
#[derive(Clone, Debug)]
pub struct ActorShell<C = ()> {
    /// Empty component slots and native state words; loading component tails
    /// later requires their real codecs. Unit placeholders are never installed.
    pub components: Components<C>,
    pub string_dc_utf16: Vec<u16>,
    pub string_e0_utf16: Vec<u16>,
    /// Authoritative native Actor +108/+10C build-target list. Logical node
    /// handles belong to this Actor; native pointer words are not duplicated.
    pub actor_list_108: aa_body::actor_property_list::State<String>,
    /// Native overlap nodes at+110/+114 have this sole authoritative owner.
    pub overlap_list_110: crate::overlap_list::State,
    /// Authoritative zone allocation; raw148/14C/150 words are absent.
    pub zones_148: ActorZoneVector,
    /// Exact remaining initialized class words keyed by native byte offset.
    /// Unknown semantic names and untouched padding are not invented.
    pub words: BTreeMap<u32, u32>,
}
#[derive(Clone, Debug)]
pub struct LoadShell<D, C = ()> {
    pub kind: LoadKind,
    pub thing: ThingShell<D>,
    pub object: Option<ObjectShell>,
    pub actor: Option<ActorShell<C>>,
}
impl<C> ActorShell<C> {
    /// Move already-owned components into a richer runtime payload without
    /// rerunning constructors or resetting any saved Actor state.
    pub fn map_components<U>(
        self,
        map: impl FnMut(crate::components::ComponentSlot, C) -> U,
    ) -> ActorShell<U> {
        ActorShell {
            components: self.components.map_values(map),
            string_dc_utf16: self.string_dc_utf16,
            string_e0_utf16: self.string_e0_utf16,
            actor_list_108: self.actor_list_108,
            overlap_list_110: self.overlap_list_110,
            zones_148: self.zones_148,
            words: self.words,
        }
    }
}
impl<D, C> LoadShell<D, C> {
    pub fn map_components<U>(
        self,
        map: impl FnMut(crate::components::ComponentSlot, C) -> U,
    ) -> LoadShell<D, U> {
        LoadShell {
            kind: self.kind,
            thing: self.thing,
            object: self.object,
            actor: self.actor.map(|actor| actor.map_components(map)),
        }
    }
}

/// 4EF020 ->4EA690 ->4D6D70 successful construction state. The original base
/// constructor registers the ID before derived initialization. The caller must
/// use registry.register_saved against already-loaded generations and preserve
/// partial state if later decoding fails. This callback-free value constructor
/// covers successful allocation only; it does not model pointer/vtable layout.
pub fn construct_shell<D>(kind: LoadKind, definition: D) -> LoadShell<D> {
    construct_shell_with_components::<D, ()>(kind, definition)
}

/// Same proved native constructor values with caller-owned typed component
/// slots. Every slot remains empty until its real component constructor runs.
pub fn construct_shell_with_components<D, C>(kind: LoadKind, definition: D) -> LoadShell<D, C> {
    let thing = ThingShell {
        definition,
        flags: 0x80000000,
        word_c: 0,
        word_10: 0,
        name_utf16: Vec::new(),
        word_1c: None,
        position_bits: [0; 3],
        facing_bits: [0; 3],
    };
    let object = (kind != LoadKind::Thing).then_some(ObjectShell {
        associated_handle: 0,
        scale_bits: 0,
        variant: -1,
        word_44: 0,
        word_48: 0,
        byte_4c: 0,
        words_50_to_54: [0; 2],
        memory: crate::detection_memory::State::from_object_constructor(),
    });
    let actor = (kind == LoadKind::Actor).then(|| {
        let mut words = BTreeMap::new();
        for index in [
            0x39, 0x3a, 0x3c, 0x3d, 0x3e, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e,
            0x51, 0x55, 0x56, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e,
        ] {
            words.insert(index * 4, 0);
        }
        for (index, value) in [(0x4f, 0x38), (0x50, 5), (0x57, 0xffffffff)] {
            words.insert(index * 4, value);
        }
        ActorShell {
            components: Components::new(StateBits::default()),
            string_dc_utf16: Vec::new(),
            string_e0_utf16: Vec::new(),
            actor_list_108: aa_body::actor_property_list::State::new(),
            overlap_list_110: crate::overlap_list::State::constructed(),
            zones_148: ActorZoneVector::default(),
            words,
        }
    });
    LoadShell {
        kind,
        thing,
        object,
        actor,
    }
}
