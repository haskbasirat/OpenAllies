//! Bounded GThing construction (4EF2A0) and identity phase (4EE460).
//! No spatial placement, derived GActor, owner or world-publication defaults.
use crate::registry::{ActorId, IdentityRegistry, RegistryError};
use aa_numeric::{Binary, Precision};
use aa_simulation::NativeRng;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, thiserror::Error)]
pub enum CreationError {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error(transparent)]
    Registry(#[from] RegistryError),
    #[error("native weighted variant selection ran past its supplied entries")]
    VariantRange,
    #[error("actor creation requires the unported blocking/spatial callback branch")]
    BlockingCallback,
    #[error("GObject initialization requires an Object or Actor shell")]
    ObjectRequired,
}
#[derive(Clone, Debug)]
pub struct ObjectDefinition {
    pub variations_enabled: bool,
    pub variation_weights_bits: Vec<u32>,
    pub total_variation_weight_bits: u32,
    pub scale_min_bits: u32,
    pub scale_max_bits: u32,
}
#[derive(Clone, Debug)]
pub struct ActorCreationRecord<D> {
    pub base: CreationRecord<D>,
    pub variant: i32,
    pub owner_word: u32,
    pub word_30: u32,
    pub request_nonblocking: bool,
}

fn rng_fraction(rng: &mut NativeRng, pc: Precision) -> aa_numeric::Result<Binary> {
    let value = rng.next_u32();
    let signed = Binary::from_i32(value as i32);
    let unsigned = if value & 0x80000000 != 0 {
        signed.add(&Binary::from_f64_bits(0x41f0000000000000)?, pc)?
    } else {
        signed
    };
    unsigned.mul(&Binary::from_f64_bits(0x3df0000000100000)?, pc)
}

/// Original49B900 weighted aspect selector, preserving the full extended
/// RNG fraction and strict-negative stopping test. The supplied total is the
/// native precomputed field+128, not a sum recomputed in a different order.
pub fn select_variant(
    definition: &ObjectDefinition,
    rng: &mut NativeRng,
    pc: Precision,
) -> Result<i32, CreationError> {
    let mut remaining = rng_fraction(rng, pc)?.mul(
        &Binary::from_f32_bits(definition.total_variation_weight_bits)?,
        pc,
    )?;
    for (index, weight) in definition.variation_weights_bits.iter().enumerate() {
        remaining = remaining.sub(&Binary::from_f32_bits(*weight)?, pc)?;
        if remaining.is_negative_nonzero() {
            return i32::try_from(index).map_err(|_| CreationError::VariantRange);
        }
    }
    Err(CreationError::VariantRange)
}

/// Original49B440 random scale. The range subtraction stores binary32 before
/// consuming RNG; the fraction, multiply and addition stay extended until3C.
pub fn random_scale(
    definition: &ObjectDefinition,
    rng: &mut NativeRng,
    pc: Precision,
) -> Result<u32, CreationError> {
    let minimum = Binary::from_f32_bits(definition.scale_min_bits)?;
    let range = Binary::from_f32_bits(
        Binary::from_f32_bits(definition.scale_max_bits)?
            .sub(&minimum, pc)?
            .store_f32()?,
    )?;
    Ok(rng_fraction(rng, pc)?
        .mul(&range, pc)?
        .add(&minimum, pc)?
        .store_f32()?)
}

/// Original new-actor ctor chain4EF2A0 ->4EA5E0 ->4D6BB0. No component
/// factories run here. Spatial changes requested by record+52 are a precise
/// unsupported precondition and rejected before effects, not silently skipped.
pub fn construct_actor<D: Clone>(
    record: &mut ActorCreationRecord<D>,
    definition: &ObjectDefinition,
    world_begin_bits: u32,
    rng: &mut NativeRng,
    directions: DirectionTable,
    pc: Precision,
) -> Result<crate::loading::LoadShell<D>, CreationError> {
    construct_actor_with_components::<D, ()>(
        record,
        definition,
        world_begin_bits,
        rng,
        directions,
        pc,
    )
}

/// Same native constructor sequence with actual caller-owned component storage.
/// Empty typed slots are created directly, without an intermediate cloned actor.
pub fn construct_actor_with_components<D: Clone, C>(
    record: &mut ActorCreationRecord<D>,
    definition: &ObjectDefinition,
    world_begin_bits: u32,
    rng: &mut NativeRng,
    directions: DirectionTable,
    pc: Precision,
) -> Result<crate::loading::LoadShell<D, C>, CreationError> {
    if record.request_nonblocking {
        return Err(CreationError::BlockingCallback);
    }
    let base = construct(&mut record.base, world_begin_bits, rng, directions, pc)?;
    let mut shell = crate::loading::construct_shell_with_components(
        crate::loading::LoadKind::Actor,
        base.definition,
    );
    shell.thing.flags = base.flags;
    shell.thing.word_c = base.word_c;
    shell.thing.word_10 = base.word_10;
    shell.thing.name_utf16 = base.name_utf16;
    shell.thing.word_1c = Some(base.created_at_bits);
    shell.thing.position_bits = base.position_bits;
    shell.thing.facing_bits = base.facing_bits;
    if definition.variations_enabled {
        shell.object.as_mut().unwrap().variant = if record.variant == -1 {
            select_variant(definition, rng, pc)?
        } else {
            record.variant
        };
    }
    let actor = shell.actor.as_mut().unwrap();
    if shell.thing.flags & (1 << 23) == 0 {
        record.owner_word = 0;
    }
    actor.words.insert(0xe8, record.owner_word);
    actor.words.insert(0x158, record.word_30);
    Ok(shell)
}

/// Original GObject InitCreate4E9FC0: identity first, then random scale.
/// Registration survives a later numerical failure, matching partial native
/// initialization. This stops before Actor InitCreate owner/component work.
pub fn initialize_object<D, C>(
    registry: &mut IdentityRegistry<crate::loading::LoadShell<D, C>>,
    saved_id: ActorId,
    shell: crate::loading::LoadShell<D, C>,
    definition: &ObjectDefinition,
    rng: &mut NativeRng,
    pc: Precision,
) -> Result<ActorId, CreationError> {
    if shell.object.is_none() {
        return Err(CreationError::ObjectRequired);
    }
    let is_actor = shell.kind == crate::loading::LoadKind::Actor;
    let id = initialize_identity(registry, saved_id, is_actor, shell)?;
    let scale = random_scale(definition, rng, pc)?;
    if let Some(object) = registry.value_mut(id).unwrap().object.as_mut() {
        object.scale_bits = scale;
    }
    Ok(id)
}

const DIRECTION_COUNT: usize = 2048;
const TABLES: &[u8; 3 * DIRECTION_COUNT * 8] =
    include_bytes!("../../../research/phase5-actors/direction-tables.bin");
const TOLERANCE: u32 = 0x38d1_b717;
const ONE: u32 = 0x3f80_0000;
const MINUS_ONE: u32 = 0xbf80_0000;

/// Exact original2048 normalized XY table, initialized independently of the
/// actor's current arithmetic PC. Explicit startup precision avoids assuming
/// that table creation and later simulation share one global FPU state.
#[derive(Clone, Copy, Debug)]
pub struct DirectionTable {
    startup_precision: Precision,
}
impl DirectionTable {
    pub const fn original(startup_precision: Precision) -> Self {
        Self { startup_precision }
    }
    pub fn at(self, index: usize) -> Option<[u32; 2]> {
        if index >= DIRECTION_COUNT {
            return None;
        }
        let table = match self.startup_precision {
            Precision::Single24 => 0,
            Precision::Double53 => 1,
            Precision::Extended64 => 2,
        };
        let offset = (table * DIRECTION_COUNT + index) * 8;
        Some([
            u32::from_le_bytes(TABLES[offset..offset + 4].try_into().unwrap()),
            u32::from_le_bytes(TABLES[offset + 4..offset + 8].try_into().unwrap()),
        ])
    }
}

/// Flags must be the resolved native definition+0x1C word. `key` is caller-owned
/// verified definition identity, never an inferred actor GTID or map index.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThingDefinition<D> {
    pub key: D,
    pub flags: u32,
}
/// Only fields actually consumed by the proved base constructor/identity phase.
/// Other native creation-record fields and derived-record types are not guessed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationRecord<D> {
    pub definition: ThingDefinition<D>,
    pub position_bits: [u32; 3],
    pub direction_bits: [u32; 2],
    pub name_utf16: Vec<u16>,
    pub saved_id: ActorId,
}
/// Completed base fields before identity initialization. Native+0x14 is still0;
/// after registration, Entry::id owns that field without a second divergent ID.
/// Native vtable/pointers and CString allocator representation are not serialized.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnregisteredThing<D> {
    pub definition: D,
    pub flags: u32,
    pub word_c: u32,
    pub word_10: u32,
    pub name_utf16: Vec<u16>,
    pub created_at_bits: u32,
    pub position_bits: [u32; 3],
    pub facing_bits: [u32; 3],
}

fn sum_squares(xy: [u32; 2], pc: Precision) -> aa_numeric::Result<Binary> {
    let x = Binary::from_f32_bits(xy[0])?;
    let y = Binary::from_f32_bits(xy[1])?;
    x.mul(&x, pc)?.add(&y.mul(&y, pc)?, pc)
}
fn absolute(value: Binary) -> Binary {
    if value.is_negative_nonzero() {
        value.negated()
    } else {
        value
    }
}

/// Native pre-normalization accepts only abs(length-1)<epsilon. Unlike a
/// normalization replacement, rejection MUTATES the supplied creation record.
fn validate_direction(direction: &mut [u32; 2], pc: Precision) -> aa_numeric::Result<()> {
    let delta = sum_squares(*direction, pc)?
        .sqrt(pc)?
        .sub(&Binary::from_f32_bits(ONE)?, pc)?;
    if absolute(delta).compare(&Binary::from_f32_bits(TOLERANCE)?) != Ordering::Less {
        *direction = [0, MINUS_ONE];
    }
    Ok(())
}
fn normalize(
    direction: [u32; 2],
    three_dimensional: bool,
    pc: Precision,
) -> aa_numeric::Result<[u32; 3]> {
    let sum = sum_squares(direction, pc)?;
    let one = Binary::from_f32_bits(ONE)?;
    let tolerance = Binary::from_f32_bits(TOLERANCE)?;
    // Original4453A0 includes the zero Z square, even though GThing sets Z=0.
    let sum = if three_dimensional {
        sum.add(&Binary::from_u32(0), pc)?
    } else {
        sum
    };
    let length = if three_dimensional {
        let length = sum.sqrt(pc)?;
        if length.compare(&tolerance) != Ordering::Greater {
            return Ok([0, 0, 0]);
        }
        length
    } else {
        if absolute(sum.sub(&one, pc)?).compare(&tolerance) != Ordering::Greater {
            return Ok([direction[0], direction[1], 0]);
        }
        sum.sqrt(pc)?
    };
    let reciprocal = one.div(&length, pc)?;
    Ok([
        reciprocal
            .mul(&Binary::from_f32_bits(direction[0])?, pc)?
            .store_f32()?,
        reciprocal
            .mul(&Binary::from_f32_bits(direction[1])?, pc)?
            .store_f32()?,
        0,
    ])
}

/// Original constructor fields/facing. The caller supplies world+E0 and the
/// exact current world+D4 RNG state; this function never chooses a seed.
/// Unpaired UTF16/NULs and raw position/time bits are copied without alteration.
/// The native logging/string-pool representation and harmless initial visual
/// notification are not represented as gameplay side effects.
///
/// Nonfinite direction input is outside the finite numerical parity domain;
/// it returns an error only if that branch consumes the supplied direction.
/// Axis-locked and random branches do not inspect it. Effects are not rolled
/// back if a later numeric error follows a random draw or record mutation.
pub fn construct<D: Clone>(
    record: &mut CreationRecord<D>,
    world_begin_bits: u32,
    world_rng: &mut NativeRng,
    directions: DirectionTable,
    pc: Precision,
) -> aa_numeric::Result<UnregisteredThing<D>> {
    let flags = 0x8000_0000
        | (record.definition.flags & 0x03ff_ff00)
        | (record.definition.flags & 0x2000_0000);
    let xy = if flags & (1 << 17) != 0 {
        [0, MINUS_ONE]
    } else if flags & (1 << 15) != 0 {
        let index = (world_rng.next_u32() % DIRECTION_COUNT as u32) as usize;
        directions
            .at(index)
            .expect("unsigned modulo is within original table")
    } else {
        validate_direction(&mut record.direction_bits, pc)?;
        record.direction_bits
    };
    let facing_bits = normalize(xy, flags & (1 << 12) != 0, pc)?;
    Ok(UnregisteredThing {
        definition: record.definition.key.clone(),
        flags,
        word_c: 0,
        word_10: 0,
        name_utf16: record.name_utf16.clone(),
        created_at_bits: world_begin_bits,
        position_bits: record.position_bits,
        facing_bits,
    })
}

/// Complete valid-record identity branch of original4EE460. A nonzero saved
/// ID seeds its free slot's generation before saved registration; otherwise the
/// normal allocator is used. Native publication and component init happen later.
///
/// `is_actor` must come from the actual derived class's IsActor classification;
/// the base GThing constructor alone is not an actor. Corrupt/occupied saved
/// records are rejected by the registry's documented safe preconditions rather
/// than reproducing the original assertion/partially poisoned table behavior.
pub fn initialize_identity<T>(
    registry: &mut IdentityRegistry<T>,
    saved_id: ActorId,
    is_actor: bool,
    payload: T,
) -> Result<ActorId, RegistryError> {
    if saved_id.is_null() {
        registry.allocate(is_actor, payload)
    } else {
        registry.seed_saved_generation(saved_id)?;
        registry.register_saved(saved_id, is_actor, payload)?;
        Ok(saved_id)
    }
}
