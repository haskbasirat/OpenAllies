//! GActor's 31-slot component selection and lifecycle loops. This is not the
//! complete GWorld actor factory: spatial, ownership and component leaf behavior
//! remain explicit caller boundaries. Failures do not roll back prior effects.
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Original enum registration 0x497FA0; table at GActor+0x60.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ComponentSlot {
    Body,
    Projectile,
    Character,
    Mover,
    Mind,
    Performer,
    Properties,
    Organization,
    Element,
    Economy,
    Denizen,
    Supply,
    Wall,
    Structure,
    Settlement,
    Construction,
    Recruit,
    Experience,
    Builder,
    MobileDeployment,
    Defender,
    Research,
    Detection,
    Aircraft,
    Transport,
    Lair,
    Flare,
    MetaGame,
    Portal,
    City,
    Hq,
}
pub const ALL_SLOTS: [ComponentSlot; 31] = [
    ComponentSlot::Body,
    ComponentSlot::Projectile,
    ComponentSlot::Character,
    ComponentSlot::Mover,
    ComponentSlot::Mind,
    ComponentSlot::Performer,
    ComponentSlot::Properties,
    ComponentSlot::Organization,
    ComponentSlot::Element,
    ComponentSlot::Economy,
    ComponentSlot::Denizen,
    ComponentSlot::Supply,
    ComponentSlot::Wall,
    ComponentSlot::Structure,
    ComponentSlot::Settlement,
    ComponentSlot::Construction,
    ComponentSlot::Recruit,
    ComponentSlot::Experience,
    ComponentSlot::Builder,
    ComponentSlot::MobileDeployment,
    ComponentSlot::Defender,
    ComponentSlot::Research,
    ComponentSlot::Detection,
    ComponentSlot::Aircraft,
    ComponentSlot::Transport,
    ComponentSlot::Lair,
    ComponentSlot::Flare,
    ComponentSlot::MetaGame,
    ComponentSlot::Portal,
    ComponentSlot::City,
    ComponentSlot::Hq,
];
const ASSEMBLY_ORDER: [usize; 31] = [
    15, 0, 2, 1, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 4,
];
impl ComponentSlot {
    pub const fn index(self) -> usize {
        self as usize
    }
    pub fn from_index(index: usize) -> Option<Self> {
        ALL_SLOTS.get(index).copied()
    }
}

/// Exact raw actor+0x100/+0x104 state words. Only the component-selection bit
/// predicates are interpreted here; other meanings are deliberately opaque.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateBits {
    pub low: u32,
    pub high: u32,
}
impl StateBits {
    pub const fn construction(self) -> bool {
        self.low & (1 << 21) != 0
    }
    pub const fn selection_special(self) -> bool {
        self.low & ((1 << 25) | (1 << 26)) != 0 || self.high & (1 << 4) != 0
    }
}

#[derive(Clone, Debug)]
pub struct Components<T> {
    slots: [Option<T>; 31],
    pub state: StateBits,
}
/// Disjoint borrowed slot view. The focused slot retains its original presence
/// but its payload is held separately; `get` refuses to fabricate access to it.
pub struct OtherComponents<'a, T> {
    before: &'a mut [Option<T>],
    after: &'a mut [Option<T>],
    focus: ComponentSlot,
    focused_present: bool,
    pub state: &'a mut StateBits,
}
impl<T> OtherComponents<'_, T> {
    pub fn focused_slot(&self) -> ComponentSlot {
        self.focus
    }
    pub fn present(&self, slot: ComponentSlot) -> bool {
        if slot == self.focus {
            self.focused_present
        } else {
            self.get(slot).is_some()
        }
    }
    pub fn get(&self, slot: ComponentSlot) -> Option<&T> {
        let i = slot.index();
        if i < self.focus.index() {
            self.before[i].as_ref()
        } else if i > self.focus.index() {
            self.after[i - self.focus.index() - 1].as_ref()
        } else {
            None
        }
    }
    pub fn get_mut(&mut self, slot: ComponentSlot) -> Option<&mut T> {
        let i = slot.index();
        if i < self.focus.index() {
            self.before[i].as_mut()
        } else if i > self.focus.index() {
            self.after[i - self.focus.index() - 1].as_mut()
        } else {
            None
        }
    }
    pub fn reborrow(&mut self) -> OtherComponents<'_, T> {
        OtherComponents {
            before: &mut *self.before,
            after: &mut *self.after,
            focus: self.focus,
            focused_present: self.focused_present,
            state: &mut *self.state,
        }
    }
}
impl<T> Components<T> {
    /// Borrows an existing component separately without removing its Option or
    /// replacing the authoritative table. Presence remains correct in the view.
    pub fn split_slot(&mut self, slot: ComponentSlot) -> (Option<&mut T>, OtherComponents<'_, T>) {
        let (before, tail) = self.slots.split_at_mut(slot.index());
        let (focus, after) = tail.split_at_mut(1);
        let focused_present = focus[0].is_some();
        (
            focus[0].as_mut(),
            OtherComponents {
                before,
                after,
                focus: slot,
                focused_present,
                state: &mut self.state,
            },
        )
    }
    /// Caller supplies native-derived state; this does not infer an actor kind
    /// or turn authored fields into runtime defaults.
    pub fn new(state: StateBits) -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            state,
        }
    }
    /// Consumes and moves actual slot payloads while preserving every state bit
    /// and slot position. No component constructors or lifecycle methods run.
    pub fn map_values<U>(self, mut map: impl FnMut(ComponentSlot, T) -> U) -> Components<U> {
        let mut index = 0;
        Components {
            state: self.state,
            slots: self.slots.map(|value| {
                let slot = ALL_SLOTS[index];
                index += 1;
                value.map(|value| map(slot, value))
            }),
        }
    }
    pub fn get(&self, slot: ComponentSlot) -> Option<&T> {
        self.slots[slot.index()].as_ref()
    }
    pub fn get_mut(&mut self, slot: ComponentSlot) -> Option<&mut T> {
        self.slots[slot.index()].as_mut()
    }
    pub fn insert(&mut self, slot: ComponentSlot, value: T) -> Option<T> {
        self.slots[slot.index()].replace(value)
    }
    pub fn remove(&mut self, slot: ComponentSlot) -> Option<T> {
        self.slots[slot.index()].take()
    }
    pub fn present(&self, slot: ComponentSlot) -> bool {
        self.slots[slot.index()].is_some()
    }
    pub fn iter(&self) -> impl Iterator<Item = (ComponentSlot, &T)> {
        ALL_SLOTS
            .into_iter()
            .filter_map(|slot| self.get(slot).map(|value| (slot, value)))
    }
    /// Complete 0x4CFB00 valid-slot dependency loop. Requirements are obtained
    /// from each live component, not guessed from a generic schema.
    pub fn validate(&mut self, hooks: &mut impl Requirements<T>) -> Result<(), MissingRequirement> {
        for slot in ALL_SLOTS {
            if self.present(slot)
                && let Some(required) = hooks.required_component(slot, self)
                && !self.present(required)
            {
                return Err(MissingRequirement { slot, required });
            }
        }
        Ok(())
    }
    /// 0x4D21A0 initial-mode selection, dependency check and construction
    /// notification. Does NOT implement owner/network publication or the
    /// runtime-reassembly mode's newly-created-list initialization.
    ///
    /// Definition mask is the verified definition+0x174 value. Constructors
    /// may alter state or slots; every later condition rereads the current table.
    /// Returned slots record successful installations in native creation order.
    pub fn assemble_initial(
        &mut self,
        definition_mask: u32,
        hooks: &mut impl AssemblyHooks<T>,
    ) -> Result<Vec<ComponentSlot>, AssemblyError> {
        let mut created = Vec::new();
        for index in ASSEMBLY_ORDER {
            let slot = ALL_SLOTS[index];
            if self.present(slot) {
                continue;
            }
            let eligible = if slot == ComponentSlot::Mind {
                !self.state.construction() // Deliberately independent of mask bit4.
            } else if definition_mask & (1 << index) == 0 {
                false
            } else {
                match slot {
                    ComponentSlot::Construction
                    | ComponentSlot::Body
                    | ComponentSlot::Properties
                    | ComponentSlot::Wall
                    | ComponentSlot::Structure
                    | ComponentSlot::Detection => true,
                    ComponentSlot::Element
                    | ComponentSlot::Economy
                    | ComponentSlot::MobileDeployment => {
                        !self.state.construction() || self.state.selection_special()
                    }
                    ComponentSlot::City => hooks.city_allowed(self),
                    _ => !self.state.construction() && !self.state.selection_special(),
                }
            };
            if eligible {
                hooks.construct(slot, self);
                if self.present(slot) {
                    created.push(slot);
                } else if slot == ComponentSlot::Construction {
                    return Err(AssemblyError::ConstructionAllocation);
                }
            }
        }
        self.validate(hooks)?;
        if self.present(ComponentSlot::Construction) {
            hooks.construction_ready(self);
        }
        Ok(created)
    }
    /// Original 0x4D71B7 component loop only. Prior base/owner/stance/nested actor
    /// construction must already have succeeded. Stops on first false callback.
    pub fn init_create<R>(
        &mut self,
        record: &R,
        hooks: &mut impl LifecycleHooks<T, R>,
    ) -> Result<(), ComponentSlot> {
        for slot in ALL_SLOTS {
            if self.present(slot) && !hooks.init_create(slot, record, self) {
                return Err(slot);
            }
        }
        Ok(())
    }
    /// Original 0x4D7287..4D72F3 component loops only, after base spatial and
    /// required-object initialization. Mind (4, actor+0x70) is initialized LAST;
    /// post-existing callbacks then run ascending over the current table.
    pub fn init_existing<R>(
        &mut self,
        hooks: &mut impl LifecycleHooks<T, R>,
    ) -> Result<(), ComponentSlot> {
        for slot in ALL_SLOTS
            .into_iter()
            .filter(|s| *s != ComponentSlot::Mind)
            .chain([ComponentSlot::Mind])
        {
            if self.present(slot) && !hooks.init_existing(slot, self) {
                return Err(slot);
            }
        }
        for slot in ALL_SLOTS {
            if self.present(slot) {
                hooks.post_existing(slot, self);
            }
        }
        Ok(())
    }
    /// Component loop of 0x4CF300 after base PostCreate and before owner/city
    /// notifications. The native flag is passed through without reinterpretation.
    pub fn post_create<R>(&mut self, record: &R, flag: u32, hooks: &mut impl LifecycleHooks<T, R>) {
        for slot in ALL_SLOTS {
            if self.present(slot) {
                hooks.post_create(slot, record, flag, self);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
#[error("component {slot:?} requires missing {required:?}")]
pub struct MissingRequirement {
    pub slot: ComponentSlot,
    pub required: ComponentSlot,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum AssemblyError {
    #[error(transparent)]
    Missing(#[from] MissingRequirement),
    #[error(
        "construction factory returned no component; allocator-success parity precondition failed"
    )]
    ConstructionAllocation,
}
pub trait Requirements<T> {
    /// Native virtual+0x20: -1 becomes None; other values must be a valid slot.
    fn required_component(
        &mut self,
        slot: ComponentSlot,
        components: &mut Components<T>,
    ) -> Option<ComponentSlot>;
}
pub trait AssemblyHooks<T>: Requirements<T> {
    /// Implement the actual selected constructor and install its result. Mind
    /// chooses among native definition-dependent subclasses; a null result is
    /// permitted except for Construction's allocator-success precondition.
    fn construct(&mut self, slot: ComponentSlot, components: &mut Components<T>);
    /// Explicit 0x4D2100(29) -> city definition predicate, not a guessed default.
    fn city_allowed(&mut self, components: &mut Components<T>) -> bool;
    /// Native 0x4B2830 leaf, after all requirements pass.
    fn construction_ready(&mut self, components: &mut Components<T>);
}
/// Component callbacks are trusted engine code. Mutations remain on failure;
/// callers must not automatically retry or treat this as an atomic transaction.
pub trait LifecycleHooks<T, R> {
    fn init_create(
        &mut self,
        slot: ComponentSlot,
        record: &R,
        components: &mut Components<T>,
    ) -> bool;
    fn init_existing(&mut self, slot: ComponentSlot, components: &mut Components<T>) -> bool;
    fn post_existing(&mut self, slot: ComponentSlot, components: &mut Components<T>);
    fn post_create(
        &mut self,
        slot: ComponentSlot,
        record: &R,
        flag: u32,
        components: &mut Components<T>,
    );
}
