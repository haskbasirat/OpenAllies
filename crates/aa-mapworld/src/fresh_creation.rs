//! Fresh creation of GObject (class 2) and DynamicDoodad (class 9) things:
//! the class-specific work below `KKC_GWorld::CreateGThing` (`4F9C10`), and
//! the entry points CreateGActor (`4FA030`) and CreateOrganization (`4FA150`).
//!
//! `4F9C10`'s own control flow (instance-name check, class switch, the
//! "Actor/Group class requires a GACD" and "Kingdom class requires a GKCD"
//! refusals, the InitCreate / InitExisting / PostCreate / registration order
//! and the destroy on failure) is `aa_actors::world_creation::create`; this
//! module is its [`CreateWorld`] for classes 2 and 9:
//!
//! * constructor `4DCB30` (DynamicDoodad) / `4EA5E0` (GObject) over the GThing
//!   constructor `4EF2A0` (`aa_actors::creation::construct`, including the
//!   random facing draw of flag bit 15 and the direction check that rewrites
//!   the parameter block), `4EE9F0`'s index move / scene transform, the
//!   variant (`49B900` draw when the requested variant is -1) and the
//!   DynamicDoodad components of definition +174 (Disturbable, `590160`) and
//!   +178 (Breakable, `58FE10`) built by `4DC920`;
//! * InitCreate `4DCBA0` / `4E9FC0`: identity `4EE460` (requested ID: `4EFB70`
//!   then `4F0190`; none: `4EFDB0`, "no IDs left" when 0x8000 are live), the
//!   random scale `49B440`, then each component's virtual +4;
//! * InitExisting `4DC730` / `4EBE50`: radius, thickness warning (`4EE700`),
//!   placement `4EFA40` (snap `46EE00`, placement test `4EF6A0`, index
//!   membership `4EE4C0`, terrain height `4F0E50`), blocking notification
//!   `4EB6E0`, tile footprint `4EA150`, tile coordinates, world list
//!   `4F1F70`, visibility `4EBC70`, then each component's virtual +4 again;
//! * PostCreate `4EBA40` (ground paint `4EB500`), registration `4F9510` /
//!   `4F4C40`, scene registration `50C220`, creation FX `42A590`.
//!
//! The component virtual +4 is `5904A0` (one world RNG draw, delay = fraction
//! x component definition +10, deferred-queue add `50ACC0`) and, for
//! Breakable, `58F960` (the same, then `50B250` unlinks every pending event
//! of the component, most recent first, while its +18 state is 0 or 1).
//!
//! RNG draws (GWorld+D4, `85D840`) in order: facing (flag bit 15), variant
//! (requested -1 with aspects_as_variations), scale, then per component one
//! draw in InitCreate and one in InitExisting.
//!
//! World subsystems owned by other ports are [`FreshWorld`] methods named after
//! the original function. Evidence: research/phase8-conversion/
//! native_fresh_creation.py -> native-fresh-creation.json (original code
//! under Unicorn), replayed by crates/aa-mapworld/tests/fresh_creation.rs.
use crate::class_conversion::{ObjectCreationParams, ReplacementWorld, VariationSource};
use aa_actors::components::ComponentSlot;
use aa_actors::creation::{self, CreationRecord, DirectionTable, ObjectDefinition, ThingDefinition};
use aa_actors::registry::ActorId;
use aa_actors::world_creation::{self, CreateRequest, CreateWorld};
use aa_numeric::{Binary, Precision};
use aa_simulation::NativeRng;
use serde::Serialize;
use std::cmp::Ordering;

/// Breakable constructor `58FE10` copies the two words at B279C0/B279C4 into
/// +24/+28. No instruction writes either address (initialized .data zero).
pub const B279C0_WORDS: [u32; 2] = [0, 0];

/// A definition shape (`4EA010`: definition+38 "A", `4EA020`: +50 "B").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ShapeRef {
    A,
    B,
}

/// The fields of definition+38 the creation reads: +0 kind, +4 radius,
/// +10 / +14 extents (kinds 2 and 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Shape {
    pub kind: u32,
    pub radius_bits: u32,
    pub extent_10_bits: u32,
    pub extent_14_bits: u32,
}

/// What the original reads from the definition object (oracle `reads`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ObjectDefinitionInputs<D> {
    pub key: D,
    /// +8 IDS (log argument).
    pub ids_utf16: Vec<u16>,
    /// +10 (the `load_warning_thing_doesnt_fit` argument).
    pub display_utf16: Vec<u16>,
    /// `[+18]+1C`.
    pub runtime_class: u32,
    /// Definition virtual +30 (ObjectData family: true).
    pub is_object_data: bool,
    /// +1C.
    pub flags: u32,
    /// +24 (radius factor of InitExisting).
    pub radius_factor_bits: u32,
    /// +28 / +2C.
    pub scale_min_bits: u32,
    pub scale_max_bits: u32,
    /// +34 byte, definition virtual +24 (`462E50`): an available city.
    pub city: bool,
    /// +38.
    pub shape_a: Shape,
    /// +108.
    pub variations_enabled: bool,
    /// +10C entries' +2C, count +110.
    pub aspect_weights_bits: Vec<u32>,
    /// +128.
    pub total_weight_bits: u32,
    /// +174 / +178 component definitions (their +10 period), class 9.
    pub component_periods_bits: [Option<u32>; 2],
}

/// A DynamicDoodad component (`4DC920`): slot 0 Disturbable (0x1C bytes,
/// `590160`), slot 1 Breakable (0x30 bytes, `58FE10`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FreshComponent<E> {
    pub slot: u8,
    /// Component definition +10 (the delay factor of `5904A0`).
    pub period_bits: u32,
    /// Updatable +4 list: pending deferred events, oldest first.
    pub scheduled: Vec<E>,
    /// Disturbable +18 = [0]; Breakable +18..+2C = [0, 0, 0, B279C0,
    /// B279C4, 1.0f].
    pub words: Vec<u32>,
}

/// The created thing (native field offsets).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FreshObject<D, E> {
    /// Runtime class (vtable A2A2A0 GObject, A28E90 DynamicDoodad).
    pub class: u32,
    pub definition: D,
    /// +8.
    pub flags: u32,
    pub word_c: u32,
    pub word_10: u32,
    /// +14 GTID (0 until InitCreate).
    pub gtid: u32,
    /// +18.
    pub name_utf16: Vec<u16>,
    /// +1C (GWorld+E0 at construction).
    pub created_at_bits: u32,
    /// +20..+28.
    pub position_bits: [u32; 3],
    /// +2C..+34.
    pub facing_bits: [u32; 3],
    /// +38 = scale x definition+24 (InitExisting).
    pub radius_bits: u32,
    /// +3C (InitCreate draw).
    pub scale_bits: u32,
    /// +40.
    pub variant: u32,
    /// +44 / +48 tile coordinates.
    pub tile: [i32; 2],
    /// +4C (only this byte is written; the others keep the allocation).
    pub byte_4c: u8,
    /// +50..+5C.
    pub words_50_to_5c: [u32; 4],
    /// +60 / +64 (class 9).
    pub components: [Option<FreshComponent<E>>; 2],
}

/// A spatial-index descriptor (`4CF210` A / `4EA0F0` B): shape, facing x, y,
/// position x, y (and a zero word).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Descriptor {
    pub shape: ShapeRef,
    pub facing_bits: [u32; 2],
    pub position_bits: [u32; 2],
}

/// Placement failure reasons of `4EF6A0` (strings A2AB04..A2A978).
pub fn placement_reason(code: u32) -> &'static str {
    match code {
        1 => "collision with tile",
        2 => "collision with thing",
        3 => "collision with ether",
        4 => "invalid structure placement",
        5 => "settlement zone cannot be placed here",
        6 => "independent building in settlement",
        7 => "indestructible object in settlement zone",
        _ => "unknown",
    }
}

/// Diagnostics the creation writes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum CreationLog {
    /// `4EFDB0` (404F60, GTID.cpp 0xAB) "KKC_GTIDManager::Add() - no IDs left".
    NoIdsLeft,
    /// `4EBE50` (404EA0, GObject.cpp 0x213) "Static object %s is not thick
    /// enough to block tiles".
    NotThickEnough { ids_utf16: Vec<u16> },
    /// `4EF6A0` (404F60, GThing.cpp 299) "creating thing %s failed (reason:
    /// %s) at %f,%f" (x, y widened from binary32).
    PlacementFailed {
        ids_utf16: Vec<u16>,
        code: u32,
        reason: &'static str,
        position_bits: [u32; 2],
    },
    /// `4EF6A0` map warning `load_warning_thing_doesnt_fit|sff` (5A1040 on
    /// GWorld+28) with definition+10, x, y; the placement then succeeds.
    ThingDoesntFit {
        display_utf16: Vec<u16>,
        position_bits: [u32; 2],
    },
    /// `4F9C10` (404ED0) log lines (instance name used, class refusals).
    Create { line: u32, format: &'static str, argument: String },
}

/// World services of the creation path; each is another owner's subsystem,
/// named after the original function. `Event` identifies a deferred-queue
/// node. Methods receiving the object get it mutably where the original
/// callee writes object fields.
pub trait FreshWorld<D> {
    type Event: Clone + PartialEq + std::fmt::Debug;
    type Error: std::fmt::Display;
    /// The definition inputs of `key`.
    fn definition(&mut self, key: &D) -> Result<ObjectDefinitionInputs<D>, Self::Error>;
    /// GWorld+D4.
    fn rng(&mut self) -> &mut NativeRng;
    /// GWorld+E0.
    fn begin_time_bits(&self) -> u32;
    /// `[B39AF8]` (startup precision of the direction table).
    fn directions(&self) -> DirectionTable;
    /// `[GWorld+30]+8` / `+4`.
    fn tile_size_bits(&self) -> u32;
    fn tile_shift(&self) -> u32;
    /// `[GWorld+28]+A8`.
    fn map_mode(&self) -> u32;
    /// `50C270` / `50C220` gate: GWorld+D8 != 6 and `[GWorld+130]+4` != 0.
    fn scene_present(&self) -> bool;
    /// `4154D0` on GWorld+F0 then the IDMGR liveness test (4F9C10).
    fn instance_name_gtid(&mut self, name: &[u16]) -> Result<Option<u32>, Self::Error>;
    fn gtid_live(&self, gtid: u32) -> Result<bool, Self::Error>;
    /// `4EFB70` + `4F0190`: register the requested saved ID.
    fn register_requested_id(&mut self, id: u32) -> Result<(), Self::Error>;
    /// `4EFDB0`: allocate; `None` when 0x8000 IDs are live.
    fn allocate_id(&mut self) -> Result<Option<u32>, Self::Error>;
    /// `4EE9F0` with flag bit 26: `5539B0(obj, newA, oldA, newB, oldB)`.
    fn index_move(
        &mut self,
        object: &mut FreshObject<D, Self::Event>,
        descriptors: [Descriptor; 4],
    ) -> Result<(), Self::Error>;
    /// `43B7D0` (scene transform, through `50C270`).
    fn scene_transform(&mut self, object: &FreshObject<D, Self::Event>) -> Result<(), Self::Error>;
    /// Shape A `46EE00(&xy, tile size)`: the snapped x, y.
    fn snap(&mut self, shape: &Shape, xy: [u32; 2], tile_size_bits: u32) -> Result<[u32; 2], Self::Error>;
    /// `4F4590(obj, 0, 0, 0, 0)`: 0 = clear, else a reason code.
    fn placement_test(&mut self, object: &FreshObject<D, Self::Event>) -> Result<u32, Self::Error>;
    /// `5536B0(obj, B, A)`: insert; the original sets object flag bit 26.
    fn index_insert(
        &mut self,
        object: &mut FreshObject<D, Self::Event>,
        b: Descriptor,
        a: Descriptor,
    ) -> Result<(), Self::Error>;
    /// `553830(obj, B, A)`: remove; the original clears flag bit 26.
    fn index_remove(
        &mut self,
        object: &mut FreshObject<D, Self::Event>,
        b: Descriptor,
        a: Descriptor,
    ) -> Result<(), Self::Error>;
    /// `4F0E50(&xy, flags)`: the extended ST0 result.
    fn terrain_height(&mut self, xy: [u32; 2], flags: u32) -> Result<Binary, Self::Error>;
    /// `4EB6E0` with flag bit 10: blocking query and per-actor notification.
    fn blocking_notify(&mut self, object: &FreshObject<D, Self::Event>) -> Result<(), Self::Error>;
    /// `4EA150`.
    fn tile_footprint(&mut self, object: &FreshObject<D, Self::Event>) -> Result<(), Self::Error>;
    /// `4F1F70` with flag bit 8: insert at the head of GWorld+8.
    fn world_list_insert(&mut self, object: &FreshObject<D, Self::Event>) -> Result<(), Self::Error>;
    /// `4EBC70`.
    fn visibility_refresh(&mut self, object: &FreshObject<D, Self::Event>) -> Result<(), Self::Error>;
    /// `4EB500(params)`.
    fn ground_paint(
        &mut self,
        object: &FreshObject<D, Self::Event>,
        params: &ObjectCreationParams<D>,
    ) -> Result<(), Self::Error>;
    /// `4F4C40`: latch B3CC40, the available-cities list GWorld+260/+264.
    fn city_assert_latch(&self) -> bool;
    fn available_cities_contains(&self, gtid: u32) -> Result<bool, Self::Error>;
    /// `855D30` assertion dialog; its result becomes the latch.
    fn assertion_dialog(&mut self, expression: &'static str, line: u32) -> Result<bool, Self::Error>;
    fn set_city_assert_latch(&mut self, value: bool);
    /// Head insert into GWorld+260 (tail +264 when empty).
    fn add_available_city(&mut self, gtid: u32) -> Result<(), Self::Error>;
    /// `43D160` (scene add, through `50C220`).
    fn scene_add(&mut self, object: &FreshObject<D, Self::Event>) -> Result<(), Self::Error>;
    /// `42A590(definition+BC, obj, 0)`.
    fn creation_fx(&mut self, object: &FreshObject<D, Self::Event>) -> Result<(), Self::Error>;
    /// `50ACC0` -> GWorld+1E8 virtual +C(updatable, delay, 0): a new node.
    fn schedule(&mut self, gtid: u32, slot: u8, delay_bits: u32) -> Result<Self::Event, Self::Error>;
    /// `4F1A70`: unlink one node.
    fn unlink(&mut self, event: Self::Event) -> Result<(), Self::Error>;
    fn log(&mut self, entry: CreationLog) -> Result<(), Self::Error>;
    /// `4F5490`: destroy an object whose initialization failed.
    fn destroy(&mut self, object: FreshObject<D, Self::Event>) -> Result<(), Self::Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum FreshError {
    #[error("creation world service failed: {0}")]
    World(String),
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("native arithmetic outside the proven domain: {0}")]
    Domain(&'static str),
    #[error("fresh creation path not ported: {0}")]
    Unsupported(&'static str),
    #[error("native null dereference in 4F9C10: {0}")]
    NullDereference(&'static str),
}

fn world<E: std::fmt::Display>(e: E) -> FreshError {
    FreshError::World(e.to_string())
}
fn f32b(bits: u32) -> Result<Binary, FreshError> {
    Binary::from_f32_bits(bits).map_err(FreshError::from)
}
/// `_ftol2` (96CC4C): truncation toward zero; out-of-range is refused.
fn ftol(bits: u32) -> Result<i32, FreshError> {
    f32b(bits)?
        .truncate_i32()
        .map_err(|_| FreshError::Domain("_ftol2 of a value outside i32"))
}

/// One world RNG draw as the native fraction (`85D840`, fild, +2^32 when
/// negative, x the binary64 at 9FDA98), at the caller's precision.
fn rng_fraction(rng: &mut NativeRng, pc: Precision) -> Result<Binary, FreshError> {
    let value = rng.next_u32();
    let signed = Binary::from_i32(value as i32);
    let unsigned = if value & 0x8000_0000 != 0 {
        signed.add(&Binary::from_f64_bits(0x41f0_0000_0000_0000)?, pc)?
    } else {
        signed
    };
    Ok(unsigned.mul(&Binary::from_f64_bits(0x3df0_0000_0010_0000)?, pc)?)
}

fn object_definition<D>(def: &ObjectDefinitionInputs<D>) -> ObjectDefinition {
    ObjectDefinition {
        variations_enabled: def.variations_enabled,
        variation_weights_bits: def.aspect_weights_bits.clone(),
        total_variation_weight_bits: def.total_weight_bits,
        scale_min_bits: def.scale_min_bits,
        scale_max_bits: def.scale_max_bits,
    }
}

fn descriptor<D, E>(object: &FreshObject<D, E>, shape: ShapeRef) -> Descriptor {
    Descriptor {
        shape,
        facing_bits: [object.facing_bits[0], object.facing_bits[1]],
        position_bits: [object.position_bits[0], object.position_bits[1]],
    }
}

/// Constructor `4DCB30` (class 9) / `4EA5E0` (class 2) on the parameter block.
pub fn construct<D: Clone, W: FreshWorld<D>>(
    params: &mut ObjectCreationParams<D>,
    def: &ObjectDefinitionInputs<D>,
    world_: &mut W,
    pc: Precision,
) -> Result<FreshObject<D, W::Event>, FreshError> {
    if !matches!(def.runtime_class, 2 | 9) {
        return Err(FreshError::Unsupported("constructor for a class other than 2 and 9"));
    }
    // 4EF2A0 (GThing) and 4EE9F0's normalization.
    let mut record = CreationRecord {
        definition: ThingDefinition {
            key: def.key.clone(),
            flags: def.flags,
        },
        position_bits: params.position_bits,
        direction_bits: params.direction_bits,
        name_utf16: params.name_utf16.clone(),
        saved_id: ActorId::from_raw(params.requested_id),
    };
    let begin = world_.begin_time_bits();
    let directions = world_.directions();
    let thing = creation::construct(&mut record, begin, world_.rng(), directions, pc)?;
    // 4EF2A0 rewrites an invalid direction in the parameter block.
    params.direction_bits = record.direction_bits;
    let mut object = FreshObject {
        class: def.runtime_class,
        definition: thing.definition,
        flags: thing.flags,
        word_c: thing.word_c,
        word_10: thing.word_10,
        gtid: 0,
        name_utf16: thing.name_utf16,
        created_at_bits: thing.created_at_bits,
        position_bits: thing.position_bits,
        facing_bits: thing.facing_bits,
        radius_bits: 0,
        scale_bits: 0,
        variant: u32::MAX,
        tile: [0, 0],
        byte_4c: 0,
        words_50_to_5c: [0; 4],
        components: [None, None],
    };
    // 4EE9F0 after normalization: flag bit 26 moves the index entry (5539B0
    // with the new facing and the old one passed by 4EF2A0, (edi, -1.0)),
    // then 50C270.
    if object.flags & (1 << 26) != 0 {
        return Err(FreshError::Unsupported(
            "4EE9F0 index move during construction (flag bit 26 is never copied from a definition)",
        ));
    }
    if world_.scene_present() {
        world_.scene_transform(&object).map_err(world)?;
    }
    // 4EA5E0: +40 = -1; with aspects_as_variations the requested variant,
    // and a 49B900 draw for -1.
    if def.variations_enabled {
        object.variant = params.variant;
        if object.variant == u32::MAX {
            object.variant = creation::select_variant(&object_definition(def), world_.rng(), pc)
                .map_err(|_| FreshError::Domain("49B900 ran past its aspect list"))?
                as u32;
        }
    }
    if def.runtime_class == 9 {
        // 4DCB30: +60/+64 = 0, then 4DC920(0), 4DC920(1).
        for slot in 0..2u8 {
            create_component(&mut object, def, slot);
        }
    }
    Ok(object)
}

/// `4DC920(slot)`: construct the component when the definition has one and
/// the slot is empty.
fn create_component<D, E>(object: &mut FreshObject<D, E>, def: &ObjectDefinitionInputs<D>, slot: u8) {
    let Some(period_bits) = def.component_periods_bits[slot as usize] else {
        return;
    };
    if object.components[slot as usize].is_some() {
        return;
    }
    let words = if slot == 0 {
        vec![0]
    } else {
        vec![0, 0, 0, B279C0_WORDS[0], B279C0_WORDS[1], 0x3f80_0000]
    };
    object.components[slot as usize] = Some(FreshComponent {
        slot,
        period_bits,
        scheduled: Vec::new(),
        words,
    });
}

/// Component virtual +4: `5904A0` (Disturbable) / `58F960` (Breakable).
fn component_reschedule<D, W: FreshWorld<D>>(
    object: &mut FreshObject<D, W::Event>,
    slot: u8,
    world_: &mut W,
    pc: Precision,
) -> Result<(), FreshError> {
    let gtid = object.gtid;
    let Some(component) = object.components[slot as usize].as_mut() else {
        return Ok(());
    };
    let fraction = rng_fraction(world_.rng(), pc)?;
    let delay = fraction.mul(&f32b(component.period_bits)?, pc)?.store_f32()?;
    let event = world_.schedule(gtid, slot, delay).map_err(world)?;
    component.scheduled.push(event);
    if slot == 1 && matches!(component.words[0], 0 | 1) {
        // 50B250: unlink the head (most recent) until the list is empty.
        while let Some(event) = component.scheduled.pop() {
            world_.unlink(event).map_err(world)?;
        }
    }
    Ok(())
}

/// InitCreate `4DCBA0` (class 9) / `4E9FC0` (class 2). False where the
/// original returns false (no ID left).
pub fn init_create<D: Clone, W: FreshWorld<D>>(
    object: &mut FreshObject<D, W::Event>,
    params: &ObjectCreationParams<D>,
    def: &ObjectDefinitionInputs<D>,
    world_: &mut W,
    pc: Precision,
) -> Result<bool, FreshError> {
    // 4EE460.
    if params.requested_id == 0 {
        match world_.allocate_id().map_err(world)? {
            Some(id) => object.gtid = id,
            None => {
                world_.log(CreationLog::NoIdsLeft).map_err(world)?;
                return Ok(false);
            }
        }
    } else {
        world_.register_requested_id(params.requested_id).map_err(world)?;
        object.gtid = params.requested_id;
    }
    // 49B440.
    object.scale_bits = creation::random_scale(&object_definition(def), world_.rng(), pc)
        .map_err(|_| FreshError::Domain("49B440 random scale"))?;
    if object.class == 9 {
        for slot in 0..2u8 {
            create_component(object, def, slot);
        }
        for slot in 0..2u8 {
            component_reschedule(object, slot, world_, pc)?;
        }
    }
    Ok(true)
}

/// `4EE700`: flags & 0x20000400 == 0x20000400 and ftol(tile size) <= shape
/// A radius x 4.0 (ordered compare).
fn blocks_tiles<D>(flags: u32, def: &ObjectDefinitionInputs<D>, tile_int: i32, pc: Precision) -> Result<bool, FreshError> {
    if flags & 0x2000_0400 != 0x2000_0400 {
        return Ok(false);
    }
    let r4 = f32b(def.shape_a.radius_bits)?.mul(&f32b(0x4080_0000)?, pc)?;
    Ok(Binary::from_i32(tile_int).compare(&r4) != Ordering::Greater)
}

/// InitExisting `4DC730` (class 9) / `4EBE50` (class 2).
pub fn init_existing<D: Clone, W: FreshWorld<D>>(
    object: &mut FreshObject<D, W::Event>,
    params: &mut ObjectCreationParams<D>,
    def: &ObjectDefinitionInputs<D>,
    world_: &mut W,
    pc: Precision,
) -> Result<bool, FreshError> {
    object.byte_4c = 0;
    object.radius_bits = f32b(object.scale_bits)?
        .mul(&f32b(def.radius_factor_bits)?, pc)?
        .store_f32()?;
    let tile_size = world_.tile_size_bits();
    let tile_int = ftol(tile_size)?;
    let quarter = Binary::from_i32(tile_int)
        .mul(&f32b(0x3e80_0000)?, pc)?
        .store_f32()?;
    if blocks_tiles(object.flags, def, tile_int, pc)? && matches!(def.shape_a.kind, 2 | 3) {
        let e10 = f32b(def.shape_a.extent_10_bits)?;
        let e14 = f32b(def.shape_a.extent_14_bits)?;
        let thickness = if e10.compare(&e14) == Ordering::Less { e10 } else { e14 };
        if thickness.compare(&f32b(quarter)?) == Ordering::Less {
            world_
                .log(CreationLog::NotThickEnough {
                    ids_utf16: def.ids_utf16.clone(),
                })
                .map_err(world)?;
        }
    }
    if !place(object, params, def, world_, tile_size)? {
        return Ok(false);
    }
    if object.flags & (1 << 10) != 0 {
        world_.blocking_notify(object).map_err(world)?;
    }
    world_.tile_footprint(object).map_err(world)?;
    let shift = world_.tile_shift() & 0x1f;
    object.tile = [
        ftol(object.position_bits[0])? >> shift,
        ftol(object.position_bits[1])? >> shift,
    ];
    if object.flags & (1 << 8) != 0 {
        world_.world_list_insert(object).map_err(world)?;
    }
    // Virtual +8C (4EA3D0): +58 > 0. Constructor-zeroed for a fresh object.
    if object.words_50_to_5c[2] as i32 > 0 {
        return Err(FreshError::Unsupported("4EB490/4EBB80 for an object with +58 > 0"));
    }
    world_.visibility_refresh(object).map_err(world)?;
    if object.class == 9 {
        for slot in 0..2u8 {
            component_reschedule(object, slot, world_, pc)?;
        }
    }
    Ok(true)
}

/// `4EFA40(params)`: snap, parameter write-back, placement check `4EF6A0`,
/// index membership `4EE4C0`, terrain height.
fn place<D: Clone, W: FreshWorld<D>>(
    object: &mut FreshObject<D, W::Event>,
    params: &mut ObjectCreationParams<D>,
    def: &ObjectDefinitionInputs<D>,
    world_: &mut W,
    tile_size: u32,
) -> Result<bool, FreshError> {
    if object.flags & (1 << 16) != 0 {
        let xy = world_
            .snap(&def.shape_a, [object.position_bits[0], object.position_bits[1]], tile_size)
            .map_err(world)?;
        object.position_bits[0] = xy[0];
        object.position_bits[1] = xy[1];
    }
    params.position_bits = object.position_bits;
    params.direction_bits = [object.facing_bits[0], object.facing_bits[1]];
    // 4EF6A0 with an A25B1C block (virtual 0 false): 4F4590.
    let code = world_.placement_test(object).map_err(world)?;
    if code != 0 {
        let xy = [object.position_bits[0], object.position_bits[1]];
        world_
            .log(CreationLog::PlacementFailed {
                ids_utf16: def.ids_utf16.clone(),
                code,
                reason: placement_reason(code),
                position_bits: xy,
            })
            .map_err(world)?;
        // 4F0460 (map mode != 0), mode 2 and reasons 1/2: warning, accepted.
        if matches!(code, 1 | 2) && world_.map_mode() == 2 {
            world_
                .log(CreationLog::ThingDoesntFit {
                    display_utf16: def.display_utf16.clone(),
                    position_bits: xy,
                })
                .map_err(world)?;
        } else {
            return Ok(false);
        }
    }
    // 4EE4C0: virtual +B0 is 6A71D0 (true) for GObject and DynamicDoodad.
    let virtual_b0 = true;
    let wanted = virtual_b0 || object.flags & 0x4408 != 0;
    let present = object.flags & (1 << 26) != 0;
    let (a, b) = (descriptor(object, ShapeRef::A), descriptor(object, ShapeRef::B));
    if present && !wanted {
        world_.index_remove(object, b, a).map_err(world)?;
    }
    if !present && wanted {
        world_.index_insert(object, b, a).map_err(world)?;
    }
    let height_flags = if object.flags & (1 << 21) != 0 {
        Some(0x123)
    } else if object.flags & (1 << 12) == 0 {
        Some(0x122)
    } else {
        None
    };
    if let Some(flags) = height_flags {
        let z = world_
            .terrain_height([object.position_bits[0], object.position_bits[1]], flags)
            .map_err(world)?;
        object.position_bits[2] = z.store_f32()?;
    }
    Ok(true)
}

/// PostCreate `4EBA40(params, flag)`: `4D1E10` is RET 8; ground paint when
/// the flag's low byte is 0 and the map mode is 0.
pub fn post_create<D, W: FreshWorld<D>>(
    object: &FreshObject<D, W::Event>,
    params: &ObjectCreationParams<D>,
    flag: u32,
    world_: &mut W,
) -> Result<(), FreshError> {
    if flag as u8 == 0 && world_.map_mode() == 0 {
        world_.ground_paint(object, params).map_err(world)?;
    }
    Ok(())
}

/// `4F9510` for GObject / DynamicDoodad: flag bit 31 cleared; class-list
/// virtuals BC / C0 / AC / B4 are false for both; `4F4C40(obj, 1)`.
pub fn register<D, W: FreshWorld<D>>(
    object: &mut FreshObject<D, W::Event>,
    def: &ObjectDefinitionInputs<D>,
    world_: &mut W,
) -> Result<(), FreshError> {
    object.flags &= 0x7fff_ffff;
    if !object.name_utf16.is_empty() {
        return Err(FreshError::Unsupported(
            "4F9510 instance-name registration (4154D0/5C9180/5043F0/4F7B70)",
        ));
    }
    // 4F4C40: virtual B0 (true) and definition virtual 24 (the city byte).
    if def.city {
        if !world_.city_assert_latch() && world_.available_cities_contains(object.gtid).map_err(world)? {
            let latch = world_
                .assertion_dialog("!m_available_cities.Contains(_thing)", 0x1403)
                .map_err(world)?;
            world_.set_city_assert_latch(latch);
        }
        world_.add_available_city(object.gtid).map_err(world)?;
    }
    Ok(())
}

/// Adapter of [`FreshWorld`] to `aa_actors::world_creation::CreateWorld`.
struct Adapter<'a, D, W: FreshWorld<D>> {
    world: &'a mut W,
    pc: Precision,
    params: ObjectCreationParams<D>,
    def: ObjectDefinitionInputs<D>,
    object: Option<FreshObject<D, W::Event>>,
    error: Option<FreshError>,
}
impl<D: Clone, W: FreshWorld<D>> Adapter<'_, D, W> {
    fn fail(&mut self, error: FreshError) -> world_creation::Error {
        let message = error.to_string();
        self.error.get_or_insert(error);
        world_creation::Error::World(message)
    }
    fn object(&mut self) -> world_creation::Result<&mut FreshObject<D, W::Event>> {
        match self.object.as_mut() {
            Some(o) => Ok(o),
            None => Err(world_creation::Error::NullDereference("no constructed object")),
        }
    }
}
type Res<T> = world_creation::Result<T>;
impl<D: Clone, W: FreshWorld<D>> CreateWorld<(), ()> for Adapter<'_, D, W> {
    type Object = ();
    fn instance_name_gtid(&mut self, name: &[u16]) -> Res<Option<u32>> {
        let r = self.world.instance_name_gtid(name).map_err(world);
        r.map_err(|e| self.fail(e))
    }
    fn gtid_live(&self, gtid: u32) -> Res<bool> {
        self.world
            .gtid_live(gtid)
            .map_err(|e| world_creation::Error::World(e.to_string()))
    }
    fn log(&mut self, line: u32, format: &'static str, argument: &str) -> Res<()> {
        let r = self
            .world
            .log(CreationLog::Create {
                line,
                format,
                argument: argument.to_owned(),
            })
            .map_err(world);
        r.map_err(|e| self.fail(e))
    }
    fn construct(&mut self, class: u32, _size: u32, _request: &CreateRequest<(), ()>) -> Res<Option<()>> {
        if !matches!(class, 2 | 9) {
            return Err(self.fail(FreshError::Unsupported("4F9C10 class constructor other than 2 and 9")));
        }
        let mut params = self.params.clone();
        let r = construct(&mut params, &self.def, self.world, self.pc);
        self.params = params;
        match r {
            Ok(o) => {
                self.object = Some(o);
                Ok(Some(()))
            }
            Err(e) => Err(self.fail(e)),
        }
    }
    fn set_state(&mut self, _: (), _: u8, _: bool) -> Res<()> {
        Err(self.fail(FreshError::Unsupported("actor state on a class 2/9 object")))
    }
    fn create_component(&mut self, _: (), _: ComponentSlot) -> Res<()> {
        Err(self.fail(FreshError::Unsupported("actor component on a class 2/9 object")))
    }
    fn definition_flags(&self, _: ()) -> Res<u32> {
        Err(world_creation::Error::World("actor definition flags on a class 2/9 object".into()))
    }
    fn construction_assert_latch(&self) -> Res<bool> {
        Err(world_creation::Error::World("actor construction latch on a class 2/9 object".into()))
    }
    fn set_construction_assert_latch(&mut self, _: bool) -> Res<()> {
        Err(self.fail(FreshError::Unsupported("actor construction latch")))
    }
    fn state_active(&mut self, _: (), _: u8) -> Res<bool> {
        Err(self.fail(FreshError::Unsupported("actor state on a class 2/9 object")))
    }
    fn assertion_dialog(&mut self, _: &'static str, _: u32) -> Res<bool> {
        Err(self.fail(FreshError::Unsupported("actor construction assertion")))
    }
    fn set_organization_layout(&mut self, _: (), _: &()) -> Res<()> {
        Err(self.fail(FreshError::Unsupported("organization layout on a class 2/9 object")))
    }
    fn init_create(&mut self, _: (), _: &CreateRequest<(), ()>) -> Res<bool> {
        let mut object = self.object.take().ok_or(world_creation::Error::NullDereference("E4"))?;
        let r = init_create(&mut object, &self.params, &self.def, self.world, self.pc);
        self.object = Some(object);
        r.map_err(|e| self.fail(e))
    }
    fn init_existing(&mut self, _: (), _: &CreateRequest<(), ()>) -> Res<bool> {
        let mut object = self.object.take().ok_or(world_creation::Error::NullDereference("E8"))?;
        let mut params = self.params.clone();
        let r = init_existing(&mut object, &mut params, &self.def, self.world, self.pc);
        self.params = params;
        self.object = Some(object);
        r.map_err(|e| self.fail(e))
    }
    fn post_create(&mut self, _: (), _: &CreateRequest<(), ()>, flag: u32) -> Res<()> {
        let object = self.object.take().ok_or(world_creation::Error::NullDereference("EC"))?;
        let r = post_create(&object, &self.params, flag, self.world);
        self.object = Some(object);
        r.map_err(|e| self.fail(e))
    }
    fn register(&mut self, _: ()) -> Res<()> {
        let mut object = self.object.take().ok_or(world_creation::Error::NullDereference("4F9510"))?;
        let r = register(&mut object, &self.def, self.world);
        self.object = Some(object);
        r.map_err(|e| self.fail(e))
    }
    fn register_updatable(&mut self, _: ()) -> Res<()> {
        // 50C220 on GWorld+130.
        if self.world.scene_present() {
            let object = self.object.take().ok_or(world_creation::Error::NullDereference("50C220"))?;
            let r = self.world.scene_add(&object).map_err(world);
            self.object = Some(object);
            r.map_err(|e| self.fail(e))?;
        }
        Ok(())
    }
    fn fires_creation_fx(&mut self, _: ()) -> Res<bool> {
        // Virtual B0 of GObject / DynamicDoodad: 6A71D0.
        self.object()?;
        Ok(true)
    }
    fn creation_fx(&mut self, _: ()) -> Res<()> {
        let object = self.object.take().ok_or(world_creation::Error::NullDereference("42A590"))?;
        let r = self.world.creation_fx(&object).map_err(world);
        self.object = Some(object);
        r.map_err(|e| self.fail(e))
    }
    fn destroy(&mut self, _: ()) -> Res<()> {
        let object = self.object.take().ok_or(world_creation::Error::NullDereference("4F5490"))?;
        let r = self.world.destroy(object).map_err(world);
        r.map_err(|e| self.fail(e))
    }
}

/// Result of [`create_thing`]: the created object (None: native null) and
/// the parameter block as the original left it (direction / position
/// written back by `4EF2A0` / `4EFA40`).
#[derive(Debug)]
pub struct Created<D, E> {
    pub object: Option<FreshObject<D, E>>,
    pub params: ObjectCreationParams<D>,
}

/// `4F9C10` on an A25B1C parameter block (virtuals 0 / 4 false): the
/// restoration agent's control flow with this module's class 2 / 9 world.
pub fn create_thing<D: Clone, W: FreshWorld<D>>(
    params: &ObjectCreationParams<D>,
    world_: &mut W,
    pc: Precision,
) -> Result<Created<D, W::Event>, FreshError> {
    let def = world_.definition(&params.definition).map_err(world)?;
    let request = CreateRequest {
        class: def.runtime_class,
        definition_ids: String::from_utf16_lossy(&def.ids_utf16),
        instance_name: params.name_utf16.clone(),
        actor_data: false,
        kingdom_data: false,
        organization_layout: None,
        mode: 0,
        byte_50: false,
        payload: (),
    };
    let mut adapter = Adapter {
        world: world_,
        pc,
        params: params.clone(),
        def,
        object: None,
        error: None,
    };
    let result = world_creation::create(&request, &mut adapter);
    if let Some(error) = adapter.error.take() {
        return Err(error);
    }
    match result {
        Ok(Some(())) => Ok(Created {
            object: adapter.object.take(),
            params: adapter.params,
        }),
        Ok(None) => Ok(Created {
            object: None,
            params: adapter.params,
        }),
        Err(world_creation::Error::NullDereference(what)) => Err(FreshError::NullDereference(what)),
        Err(world_creation::Error::World(what)) => Err(FreshError::World(what)),
    }
}

/// [`ReplacementWorld`] for the class-changed record replacement on top of
/// [`create_thing`]. `created` collects the objects for the caller to insert
/// into its world store.
pub struct FreshReplacement<'a, D, W: FreshWorld<D>> {
    pub world: &'a mut W,
    pub pc: Precision,
    pub created: Vec<FreshObject<D, W::Event>>,
}
impl<D: Clone, W: FreshWorld<D>> VariationSource<D> for FreshReplacement<'_, D, W> {
    type Error = FreshError;
    fn is_object_data(&mut self, d: &D) -> Result<bool, FreshError> {
        Ok(self.world.definition(d).map_err(world)?.is_object_data)
    }
    fn variations_enabled(&mut self, d: &D) -> Result<bool, FreshError> {
        Ok(self.world.definition(d).map_err(world)?.variations_enabled)
    }
    fn aspect_count(&mut self, d: &D) -> Result<i32, FreshError> {
        i32::try_from(self.world.definition(d).map_err(world)?.aspect_weights_bits.len())
            .map_err(|_| FreshError::Domain("aspect count above i32"))
    }
}
impl<D: Clone, W: FreshWorld<D>> ReplacementWorld<D> for FreshReplacement<'_, D, W> {
    type Created = u32;
    fn create_thing(&mut self, params: &ObjectCreationParams<D>) -> Result<Option<u32>, FreshError> {
        let created = create_thing(params, self.world, self.pc)?;
        Ok(created.object.map(|object| {
            let gtid = object.gtid;
            self.created.push(object);
            gtid
        }))
    }
}

/// CreateGActor `4FA030` / CreateOrganization `4FA150` services.
pub trait EntryWorld {
    type Object: Clone;
    type Layout: Clone;
    type Error: std::fmt::Display;
    /// `4F9C10` on the parameter block (with its current position).
    fn create(&mut self, position_bits: [u32; 3]) -> Result<Option<Self::Object>, Self::Error>;
    /// Object virtual B4 (IsGActor).
    fn is_gactor(&mut self, object: &Self::Object) -> Result<bool, Self::Error>;
    /// Latches B3CC43 (4FA030) / B3CC44 (4FA150).
    fn assert_latch(&self, address: u32) -> bool;
    fn set_assert_latch(&mut self, address: u32, value: bool);
    /// `855D30("thing->IsGActor()", GWorld.cpp, line)`.
    fn assertion_dialog(&mut self, line: u32) -> Result<bool, Self::Error>;
    /// 4FA150: definition virtual 38 (IsActorData) and +174.
    fn definition_is_actor_data(&mut self) -> Result<bool, Self::Error>;
    fn definition_flags_174(&mut self) -> Result<u32, Self::Error>;
    /// Params +2C, else definition +2F0.
    fn layout(&mut self) -> Result<Self::Layout, Self::Error>;
    /// `[layout+8]+2C`.
    fn layout_slot_count(&mut self, layout: &Self::Layout) -> Result<i32, Self::Error>;
    /// `4C2BC0(layout)`.
    fn first_slot(&mut self, layout: &Self::Layout) -> Result<u32, Self::Error>;
    /// `578290(layout, slot, &xy, facing)`: placement test; may move xy.
    fn slot_placement(
        &mut self,
        layout: &Self::Layout,
        slot: u32,
        xy: &mut [u32; 2],
        facing_bits: [u32; 2],
    ) -> Result<bool, Self::Error>;
    /// Object +7C Organization: its layout data {+18 leader, +2C slot
    /// count} and `+1C[+28[slot]] != 0`.
    fn organization(&mut self, object: &Self::Object) -> Result<bool, Self::Error>;
    fn leader_slot(&mut self, object: &Self::Object) -> Result<i32, Self::Error>;
    fn organization_slot_count(&mut self, object: &Self::Object) -> Result<i32, Self::Error>;
    fn slot_defined(&mut self, object: &Self::Object, slot: i32) -> Result<bool, Self::Error>;
    /// `57AD30(slot, flag)` on the Organization (member spawn, 4FBF40).
    fn spawn_member(&mut self, object: &Self::Object, slot: i32, flag: u8) -> Result<(), Self::Error>;
    /// `57A730`: members created.
    fn member_count(&mut self, object: &Self::Object) -> Result<u32, Self::Error>;
    /// 404F60 "org %d failed to create any elements - destroying" (0x8A0).
    fn log_no_members(&mut self, object: &Self::Object) -> Result<(), Self::Error>;
    /// `4F5490`.
    fn destroy(&mut self, object: Self::Object) -> Result<(), Self::Error>;
}

/// `4FA030`: create, then the IsGActor assertion (latch B3CC43, line 0x85E).
pub fn create_gactor<W: EntryWorld>(world_: &mut W, position_bits: [u32; 3]) -> Result<Option<W::Object>, FreshError> {
    let Some(object) = world_.create(position_bits).map_err(world)? else {
        return Ok(None);
    };
    gactor_assert(world_, &object, 0xb3cc43, 0x85e)?;
    Ok(Some(object))
}

fn gactor_assert<W: EntryWorld>(world_: &mut W, object: &W::Object, latch: u32, line: u32) -> Result<(), FreshError> {
    if !world_.assert_latch(latch) && !world_.is_gactor(object).map_err(world)? {
        let value = world_.assertion_dialog(line).map_err(world)?;
        world_.set_assert_latch(latch, value);
    }
    Ok(())
}

/// The parameter-block fields `4FA150` reads and writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct OrganizationParams {
    /// +8..+10 (written: slot position, z = 0).
    pub position_bits: [u32; 3],
    /// +14 / +18.
    pub facing_bits: [u32; 2],
    /// +51.
    pub byte_51: u8,
}

/// `4FA150`.
pub fn create_organization<W: EntryWorld>(
    world_: &mut W,
    params: &mut OrganizationParams,
) -> Result<Option<W::Object>, FreshError> {
    if world_.definition_is_actor_data().map_err(world)?
        && world_.definition_flags_174().map_err(world)? & (1 << 7) != 0
    {
        let layout = world_.layout().map_err(world)?;
        if world_.layout_slot_count(&layout).map_err(world)? > 0 {
            let slot = world_.first_slot(&layout).map_err(world)?;
            let mut xy = [params.position_bits[0], params.position_bits[1]];
            if !world_
                .slot_placement(&layout, slot, &mut xy, params.facing_bits)
                .map_err(world)?
            {
                return Ok(None);
            }
            // 4BE740: (x, y, 0).
            params.position_bits = [xy[0], xy[1], 0];
        }
    }
    let Some(object) = world_.create(params.position_bits).map_err(world)? else {
        return Ok(None);
    };
    gactor_assert(world_, &object, 0xb3cc44, 0x880)?;
    finish_organization_members(world_, object, |_| Ok(params.byte_51))
}

///4FA2C4 ordinary member completion after the actual4F9C10 result and
///IsGActor assertion. This is the SAME final kernel used by4FA150. Callers
///lend their actual captured Organization/current layout/member owners; no
///separate member list or source-only population count is created here.
///`organization` captures actor+7C once. Leader/layout/definition checks use
///that original component; the leader spawn also uses it. Nonleader spawns
///and the final member count reread actor+7C. The parameter-byte callback is
///read only immediately before each reached nonleader spawn.
pub trait CompletionWorld {
    type Object;
    type Error: std::fmt::Display;
    fn organization(&mut self, object: &Self::Object) -> Result<bool, Self::Error>;
    fn leader_slot(&mut self, object: &Self::Object) -> Result<i32, Self::Error>;
    fn organization_slot_count(&mut self, object: &Self::Object) -> Result<i32, Self::Error>;
    fn slot_defined(&mut self, object: &Self::Object, slot: i32) -> Result<bool, Self::Error>;
    fn spawn_member(&mut self, object: &Self::Object, slot: i32, flag: u8) -> Result<(), Self::Error>;
    fn member_count(&mut self, object: &Self::Object) -> Result<u32, Self::Error>;
    fn log_no_members(&mut self, object: &Self::Object) -> Result<(), Self::Error>;
    fn destroy(&mut self, object: Self::Object) -> Result<(), Self::Error>;
}
impl<W: EntryWorld> CompletionWorld for W {
    type Object=W::Object;
    type Error=W::Error;
    fn organization(&mut self,o:&Self::Object)->Result<bool,Self::Error>{EntryWorld::organization(self,o)}
    fn leader_slot(&mut self,o:&Self::Object)->Result<i32,Self::Error>{EntryWorld::leader_slot(self,o)}
    fn organization_slot_count(&mut self,o:&Self::Object)->Result<i32,Self::Error>{EntryWorld::organization_slot_count(self,o)}
    fn slot_defined(&mut self,o:&Self::Object,s:i32)->Result<bool,Self::Error>{EntryWorld::slot_defined(self,o,s)}
    fn spawn_member(&mut self,o:&Self::Object,s:i32,f:u8)->Result<(),Self::Error>{EntryWorld::spawn_member(self,o,s,f)}
    fn member_count(&mut self,o:&Self::Object)->Result<u32,Self::Error>{EntryWorld::member_count(self,o)}
    fn log_no_members(&mut self,o:&Self::Object)->Result<(),Self::Error>{EntryWorld::log_no_members(self,o)}
    fn destroy(&mut self,o:Self::Object)->Result<(),Self::Error>{EntryWorld::destroy(self,o)}
}
pub fn finish_organization_members<W: CompletionWorld>(
    world_: &mut W, object: W::Object, mut byte_51: impl FnMut(&mut W) -> Result<u8, W::Error>,
) -> Result<Option<W::Object>, FreshError> {
    if !world_.organization(&object).map_err(world)? {
        return Ok(Some(object));
    }
    let leader = world_.leader_slot(&object).map_err(world)?;
    if leader >= 0 && world_.slot_defined(&object, leader).map_err(world)? {
        world_.spawn_member(&object, leader, 0).map_err(world)?;
    }
    let mut slot = 0;
    while slot < world_.organization_slot_count(&object).map_err(world)? {
        if slot != leader && world_.slot_defined(&object, slot).map_err(world)? {
            let flag = byte_51(world_).map_err(world)?;
            world_.spawn_member(&object, slot, flag).map_err(world)?;
        }
        slot += 1;
    }
    if world_.member_count(&object).map_err(world)? == 0 {
        world_.log_no_members(&object).map_err(world)?;
        world_.destroy(object).map_err(world)?;
        return Ok(None);
    }
    Ok(Some(object))
}
