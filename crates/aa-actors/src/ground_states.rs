//! Original ground-component constructor state. Saved Load applies afterwards;
//! these constructors do not schedule work or run component InitCreate methods.
use crate::ActorId;
use aa_body::{Body, F32};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error(transparent)]
    Body(#[from] aa_body::Error),
    #[error("required component definition is invalid: {0}")]
    Definition(&'static str),
    #[error("native component allocation failed")]
    Allocation,
    #[error("native component field {0:#x} was read before initialization")]
    UninitializedField(u32),
}
pub type Result<T> = std::result::Result<T, Error>;

/// The portable owner is the central actor registry identity. Native +0C is
/// the Updatable intrusive-list head; a new component has no scheduled entries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentHeader {
    pub owner: ActorId,
    pub scheduled_head: Option<u64>,
}
impl ComponentHeader {
    pub fn new(owner: ActorId) -> Self {
        Self {
            owner,
            scheduled_head: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyState {
    pub header: ComponentHeader,
    pub body: Body,
}
impl BodyState {
    /// 561960 uses null-context health/defense getters, each stored binary32.
    pub fn new(owner: ActorId, maximum: F32, defense: F32) -> Result<Self> {
        Ok(Self {
            header: ComponentHeader::new(owner),
            body: Body::from_resolved(maximum, defense)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MoverDefinition {
    pub speed: F32,
    pub acceleration: F32,
    pub turn_radius: F32,
    pub turn_type: i32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoverState {
    pub header: ComponentHeader,
    /// Definition +228 via null-context45D6F0, then native +28.
    pub speed: F32,
    pub acceleration: F32,
    pub turn_radius: F32,
    /// +34 is untouched for turn types other than0,1,2. Absence is intentional.
    pub angular_speed: Option<F32>,
    /// Exact remaining initialized words, keyed by native component byte offset.
    /// Untouched +48,+58,+5C,+6C and padding have no fabricated values.
    pub words: BTreeMap<u32, u32>,
    pub bytes: BTreeMap<u32, u8>,
}
impl MoverState {
    pub fn new(owner: ActorId, definition: MoverDefinition, precision: Precision) -> Result<Self> {
        let (turn_radius, angular_speed) = match definition.turn_type {
            0 => (F32::ZERO, Some(F32::ZERO)),
            1 | 2 => (
                definition.turn_radius,
                Some(F32(Binary::from_f32_bits(definition.speed.0)?
                    .div(&Binary::from_f32_bits(definition.turn_radius.0)?, precision)?
                    .store_f32()?)),
            ),
            _ => (definition.turn_radius, None),
        };
        let mut words = BTreeMap::new();
        for offset in [
            0x10, 0x14, 0x18, 0x1c, 0x20, 0x24, 0x38, 0x3c, 0x40, 0x44, 0x60, 0x64, 0x68, 0x70,
            0x74, 0x78, 0x7c, 0x80, 0x84,
        ] {
            words.insert(offset, 0);
        }
        words.insert(0x50, F32::ONE.0);
        Ok(Self {
            header: ComponentHeader::new(owner),
            speed: definition.speed,
            acceleration: definition.acceleration,
            turn_radius,
            angular_speed,
            words,
            bytes: [0x4c, 0x4d, 0x54, 0x55, 0x56, 0x88]
                .into_iter()
                .map(|offset| (offset, 0))
                .collect(),
        })
    }
    /// Operational movement must reject the native uninitialized angular branch.
    pub fn require_angular_speed(&self) -> Result<F32> {
        self.angular_speed.ok_or(Error::Definition(
            "Mover turn type leaves angular speed uninitialized",
        ))
    }
    /// Borrow a procedural movement snapshot from the one authoritative
    /// component. +48 remains None until a native target command writes it.
    pub fn movement_state(&self) -> Result<aa_movement::MoverState> {
        let raw = |offset| {
            self.words
                .get(&offset)
                .copied()
                .ok_or(Error::UninitializedField(offset))
        };
        let f = |offset| raw(offset).map(f32::from_bits);
        let byte = |offset| {
            self.bytes
                .get(&offset)
                .map(|v| *v != 0)
                .ok_or(Error::UninitializedField(offset))
        };
        Ok(aa_movement::MoverState {
            destination: [f(0x10)?, f(0x14)?, f(0x18)?],
            last_step_speed: f(0x1c)?,
            speed: f(0x20)?,
            requested_speed: f(0x24)?,
            acceleration: f32::from_bits(self.acceleration.0),
            turn_rate: f32::from_bits(self.require_angular_speed()?.0),
            desired_direction: [f(0x38)?, f(0x3c)?, f(0x40)?],
            target_id: raw(0x44)?,
            target_range: self.words.get(&0x48).copied().map(f32::from_bits),
            ignore_terrain_speed_limit: byte(0x4c)?,
            speed_factor: f(0x50)?,
            instant_turn: byte(0x54)?,
            turning: byte(0x55)?,
            falling: byte(0x56)?,
        })
    }
    /// Commit all procedural changes, including partial changes after an error.
    /// Call this on success AND failure; original movement is not transactional.
    pub fn apply_movement_state(&mut self, state: &aa_movement::MoverState) {
        for (base, vector) in [(0x10, state.destination), (0x38, state.desired_direction)] {
            for (index, value) in vector.into_iter().enumerate() {
                self.words.insert(base + index as u32 * 4, value.to_bits());
            }
        }
        for (offset, value) in [
            (0x1c, state.last_step_speed),
            (0x20, state.speed),
            (0x24, state.requested_speed),
            (0x50, state.speed_factor),
        ] {
            self.words.insert(offset, value.to_bits());
        }
        self.acceleration = F32(state.acceleration.to_bits());
        self.angular_speed = Some(F32(state.turn_rate.to_bits()));
        self.words.insert(0x44, state.target_id);
        match state.target_range {
            Some(value) => {
                self.words.insert(0x48, value.to_bits());
            }
            None => {
                self.words.remove(&0x48);
            }
        }
        for (offset, value) in [
            (0x4c, state.ignore_terrain_speed_limit),
            (0x54, state.instant_turn),
            (0x55, state.turning),
            (0x56, state.falling),
        ] {
            self.bytes.insert(offset, u8::from(value));
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterState<D> {
    pub header: ComponentHeader,
    /// Native Character+14 / Animator+4 start timestamp. Legacy field name.
    pub elapsed: F32,
    /// Native Character+18 / Animator+8 paused timestamp. Legacy field name.
    pub resumed_at: F32,
    pub rate: F32,
    pub animation: Option<D>,
    /// Native Character+24 is the paused flag, not general activity.
    pub active: bool,
    pub word_28: u32,
    pub variant: i32,
}
impl<D: Clone> CharacterState<D> {
    ///563670 selects the first resolved Character view animation, when present.
    pub fn new(owner: ActorId, animations: &[Option<D>]) -> Self {
        Self {
            header: ComponentHeader::new(owner),
            elapsed: F32::ZERO,
            resumed_at: F32::ZERO,
            rate: F32::ONE,
            animation: animations.first().cloned().flatten(),
            active: false,
            word_28: 0,
            variant: -2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementState<D> {
    pub header: ComponentHeader,
    /// Native +10 owning regiment and +14 selected regiment element definition.
    pub organization: Option<ActorId>,
    pub element_definition: Option<D>,
    pub slot_index: i32,
    pub flag_1c: bool,
    pub word_20: u32,
    pub time_24: F32,
    pub history: Vec<[u32; 7]>,
}
impl<D> ElementState<D> {
    pub fn new(owner: ActorId) -> Result<Self> {
        let mut history = Vec::new();
        history
            .try_reserve_exact(1)
            .map_err(|_| Error::Allocation)?;
        Ok(Self {
            header: ComponentHeader::new(owner),
            organization: None,
            element_definition: None,
            slot_index: -1,
            flag_1c: false,
            word_20: 0,
            time_24: F32(0xbf800000),
            history,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutDefinition<D> {
    /// Source layout +8 definition; this is not a renderer layout identifier.
    pub definition: D,
    /// Source layout parameters +0C, count definition+20, in original order.
    pub parameters: Vec<Option<D>>,
    /// Layout definition+2C is the native member-array initial capacity.
    pub slots: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationState<D> {
    pub header: ComponentHeader,
    pub layout: LayoutDefinition<D>,
    pub layout_word_14: u32,
    pub members: Vec<ActorId>,
    pub words_34: Vec<u32>,
    pub words_50: Vec<u32>,
    pub offsets_78: Vec<[F32; 2]>,
    /// Native +B4 world time, +B8 null-context definition getter45D4F0(+308).
    pub created_at: F32,
    pub definition_value_308: F32,
    pub position_xy: [F32; 2],
    pub words: BTreeMap<u32, u32>,
    pub bytes: BTreeMap<u32, u8>,
}
fn empty_with_capacity<T>(capacity: usize) -> Result<Vec<T>> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(capacity)
        .map_err(|_| Error::Allocation)?;
    Ok(result)
}
impl<D> OrganizationState<D> {
    pub fn new(
        owner: ActorId,
        layout: LayoutDefinition<D>,
        created_at: F32,
        definition_value_308: F32,
        position_xy: [F32; 2],
    ) -> Result<Self> {
        // Original signed dimensions must be valid before allocator-size shifts.
        if layout.slots > i32::MAX as usize || layout.parameters.len() > i32::MAX as usize {
            return Err(Error::Definition(
                "Organization layout count outside native signed range",
            ));
        }
        let mut words = BTreeMap::new();
        for offset in [
            0x40, 0x44, 0x48, 0x64, 0x68, 0x6c, 0x70, 0x74, 0x90, 0x94, 0x98, 0x9c, 0xa0, 0xa4,
            0xac, 0xc4, 0xc8, 0xcc, 0xd0, 0xd4,
        ] {
            words.insert(offset, 0);
        }
        for offset in [0x5c, 0x60, 0x84, 0x88] {
            words.insert(offset, F32::ONE.0);
        }
        words.insert(0xa8, 10);
        words.insert(0xd8, 0x7f7fffff);
        Ok(Self {
            header: ComponentHeader::new(owner),
            members: empty_with_capacity(layout.slots)?,
            words_34: empty_with_capacity(layout.slots)?,
            words_50: empty_with_capacity(layout.slots)?,
            offsets_78: empty_with_capacity(layout.slots)?,
            layout,
            layout_word_14: 0,
            created_at,
            definition_value_308,
            position_xy,
            words,
            bytes: [0x4c, 0x4d, 0x8c, 0x8d, 0x8e, 0xb0]
                .into_iter()
                .map(|offset| (offset, 0))
                .collect(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceVector {
    pub values: Vec<F32>,
    /// Native4CD440 reserves8 only when the registered resource count iszero.
    pub initial_capacity: usize,
}
impl ResourceVector {
    fn new(resource_count: usize) -> Result<Self> {
        if resource_count > i32::MAX as usize {
            return Err(Error::Definition(
                "resource count outside native signed range",
            ));
        }
        let initial_capacity = if resource_count == 0 {
            8
        } else {
            resource_count
        };
        let mut values = empty_with_capacity(initial_capacity)?;
        values.resize(resource_count, F32::ZERO);
        Ok(Self {
            values,
            initial_capacity,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EconomyState {
    pub header: ComponentHeader,
    pub flag_10: bool,
    pub flag_11: bool,
    /// Native resource-vector objects at+14,+24,+34,+44, in that order.
    pub resources: [ResourceVector; 4],
}
impl EconomyState {
    pub fn new(owner: ActorId, registered_resource_count: usize) -> Result<Self> {
        Ok(Self {
            header: ComponentHeader::new(owner),
            flag_10: false,
            flag_11: false,
            resources: [
                ResourceVector::new(registered_resource_count)?,
                ResourceVector::new(registered_resource_count)?,
                ResourceVector::new(registered_resource_count)?,
                ResourceVector::new(registered_resource_count)?,
            ],
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectionState {
    pub header: ComponentHeader,
    pub word_10: u32,
    /// Native45D790 null-context getter loads definition+1CC then stores+14.
    pub radius: F32,
}
impl DetectionState {
    pub fn new(owner: ActorId, radius: F32) -> Self {
        Self {
            header: ComponentHeader::new(owner),
            word_10: 0,
            radius,
        }
    }
}
