//! Saved GActor::Init4D7230(NULL), before Kingdom4E5DF0 and outer publication.
//! Every world/component operation is required. The coordinator preserves
//! native order and retained effects on false/error; it never recreates state.
//! Actor+138 is deliberately neither read nor written on this NULL path.
use crate::{
    ActorId,
    actor_creation::{Error, Result},
    components::{ALL_SLOTS, ComponentSlot},
};
use aa_numeric::{Binary, Precision};

pub trait World {
    type Definition: Copy;
    type Track: Copy;
    type Zone: Copy;
    type Updatable: Copy;
    fn object_init_null(&mut self, actor: ActorId, pc: Precision) -> Result<bool>;
    fn required_object_init(&mut self, actor: ActorId) -> Result<bool>;
    fn has_component(&self, actor: ActorId, slot: ComponentSlot) -> Result<bool>;
    fn component_init_existing(
        &mut self,
        actor: ActorId,
        slot: ComponentSlot,
        pc: Precision,
    ) -> Result<bool>;
    fn component_after_init(&mut self, actor: ActorId, slot: ComponentSlot) -> Result<()>;
    ///Actual actor+158; null is ActorId::NULL, not a missing field default.
    fn required_object(&self, actor: ActorId) -> Result<ActorId>;
    fn gtid_live(&self, actor: ActorId) -> Result<bool>;
    fn required_object_attach(&mut self, required: ActorId, actor: ActorId) -> Result<()>;
    fn refresh_4d4cf0(&mut self, actor: ActorId) -> Result<()>;
    fn refresh_4d1350(&mut self, actor: ActorId) -> Result<()>;
    fn refresh_4d67f0(&mut self, actor: ActorId) -> Result<()>;
    fn thing_flags(&self, actor: ActorId) -> Result<u32>;
    ///Actual global source Property reference through GameData+464/+24.
    fn global_property(&self) -> Result<Option<u32>>;
    fn property_applies(&mut self, actor: ActorId, property: u32) -> Result<bool>;
    fn properties_add(&mut self, actor: ActorId, property: u32) -> Result<()>;
    fn refresh_4d4130(&mut self, actor: ActorId) -> Result<()>;
    fn organization_refresh(&mut self, actor: ActorId) -> Result<()>;
    fn settlement_refresh(&mut self, actor: ActorId) -> Result<()>;
    fn performer_refresh(&mut self, actor: ActorId) -> Result<()>;
    fn denizen_refresh(&mut self, actor: ActorId) -> Result<()>;
    fn needs_settlement_zone(&mut self, actor: ActorId) -> Result<bool>;
    fn associated_object(&self, actor: ActorId) -> Result<ActorId>;
    fn create_zone2(&mut self, actor: ActorId) -> Result<bool>;
    fn discovery_radius(&mut self, pc: Precision) -> Result<u32>;
    fn position_xy(&self, actor: ActorId) -> Result<[u32; 2]>;
    ///Current+14C>2 and actual[+148][2], including a null entry.
    fn zone2(&self, actor: ActorId) -> Result<Option<Self::Zone>>;
    fn move_zone(&mut self, zone: Self::Zone, xy: [u32; 2], radius: u32) -> Result<()>;
    ///Present inactive SAI is still present; the leaf owns its active gate.
    fn sai_present(&self) -> Result<bool>;
    fn sai_notify(&mut self, actor: ActorId) -> Result<()>;
    ///The native scan captures actor+4 once after SAI, retaining this identity.
    fn track_definition(&self, actor: ActorId) -> Result<Self::Definition>;
    fn track_count(&self, definition: Self::Definition) -> Result<i32>;
    fn track_at(&self, definition: Self::Definition, index: usize) -> Result<Self::Track>;
    fn track_kind(&self, track: Self::Track) -> Result<u32>;
    fn track_length_bits(&self, track: Self::Track) -> Result<u32>;
    fn track_scale_bits(&self) -> Result<u32>;
    fn track_divisor(&self) -> Result<i32>;
    ///6F7810(20): None is the actual allocation-null branch, not an error.
    fn allocate_track(&mut self) -> Result<Option<Self::Updatable>>;
    fn construct_track(
        &mut self,
        allocation: Self::Updatable,
        actor: ActorId,
        period: i32,
    ) -> Result<Self::Updatable>;
    fn set_track_updatable(&mut self, actor: ActorId, value: Option<Self::Updatable>)
    -> Result<()>;
    fn state_low(&self, actor: ActorId) -> Result<u32>;
    ///42A590 on the actor's CURRENT definition+E4, arguments(actor,0).
    fn existing_fx(&mut self, actor: ActorId) -> Result<()>;
}

fn track_numerator(scale: u32, largest: u32, pc: Precision) -> Result<i32> {
    // The same proved numeric sequence as actor_creation::track_period,
    // split BEFORE the divisor read to retain native failure/read ordering.
    let product = Binary::from_f32_bits(scale)?
        .mul(&Binary::from_f32_bits(largest)?, pc)?
        .add(&Binary::from_f32_bits(0x3f000000)?, pc)?;
    let stored = aa_numeric::crt_pow::store_binary64(&product)
        .map_err(|_| Error::Domain("fstp qword of saved TrackInfo product"))?;
    let floored = f64::from_bits(stored).floor();
    if !(floored >= i32::MIN as f64 && floored < 2_147_483_648.0) {
        return Err(Error::Domain(
            "_ftol2 of saved TrackInfo period outside i32",
        ));
    }
    Ok(floored as i32)
}

fn tracks(world: &mut impl World, actor: ActorId, pc: Precision) -> Result<()> {
    let definition = world.track_definition(actor)?;
    if world.track_count(definition)? <= 0 {
        return Ok(());
    }
    let mut any = false;
    let mut largest = 0u32;
    let mut index = 0usize;
    let mut count = world.track_count(definition)?;
    while count > 0 && index < count as usize {
        let track = world.track_at(definition, index)?;
        if world.track_kind(track)? == 1 {
            any = true;
            let track = world.track_at(definition, index)?;
            let length = world.track_length_bits(track)?;
            // FCOMP/test AH,5/JP: strictly ordered largest<length only.
            if f32::from_bits(largest) < f32::from_bits(length) {
                let track = world.track_at(definition, index)?;
                largest = world.track_length_bits(track)?;
            }
        }
        index += 1;
        count = world.track_count(definition)?;
    }
    let numerator = track_numerator(world.track_scale_bits()?, largest, pc)?;
    let divisor = world.track_divisor()?;
    if divisor == 0 {
        return Err(Error::DivideByZero("4D74F4 saved TrackInfo idiv"));
    }
    let period = numerator
        .checked_div(divisor)
        .ok_or(Error::Domain("4D74F4 saved TrackInfo idiv overflow (#DE)"))?
        .wrapping_add(1);
    if any {
        let value = match world.allocate_track()? {
            Some(allocation) => Some(world.construct_track(allocation, actor, period)?),
            None => None,
        };
        world.set_track_updatable(actor, value)?;
    }
    Ok(())
}

///4D7230(NULL). Caller owns LoadThing's bit7 gate and later PostCreate,
///registration or destruction. This function alone cannot publish a battle.
pub fn initialize(world: &mut impl World, actor: ActorId, pc: Precision) -> Result<bool> {
    if !world.object_init_null(actor, pc)? {
        return Ok(false);
    }
    if !world.required_object_init(actor)? {
        return Ok(false);
    }
    for slot in ALL_SLOTS {
        if slot != ComponentSlot::Mind
            && world.has_component(actor, slot)?
            && !world.component_init_existing(actor, slot, pc)?
        {
            return Ok(false);
        }
    }
    if world.has_component(actor, ComponentSlot::Mind)?
        && !world.component_init_existing(actor, ComponentSlot::Mind, pc)?
    {
        return Ok(false);
    }
    for slot in ALL_SLOTS {
        if world.has_component(actor, slot)? {
            world.component_after_init(actor, slot)?;
        }
    }
    let required = world.required_object(actor)?;
    if !required.is_null() {
        if !world.gtid_live(required)? {
            return Err(Error::NullDereference(
                "4D7323 required-object virtual128 on stale/null slot",
            ));
        }
        world.required_object_attach(required, actor)?;
    }
    world.refresh_4d4cf0(actor)?;
    world.refresh_4d1350(actor)?;
    world.refresh_4d67f0(actor)?;
    if world.thing_flags(actor)? & (1 << 29) == 0
        && let Some(property) = world.global_property()?
        && world.has_component(actor, ComponentSlot::Properties)?
        && world.property_applies(actor, property)?
    {
        world.properties_add(actor, property)?;
    }
    world.refresh_4d4130(actor)?;
    if world.has_component(actor, ComponentSlot::Organization)? {
        world.organization_refresh(actor)?;
    }
    if world.has_component(actor, ComponentSlot::Settlement)? {
        world.settlement_refresh(actor)?;
    }
    if world.has_component(actor, ComponentSlot::Performer)? {
        world.performer_refresh(actor)?;
    }
    if world.has_component(actor, ComponentSlot::Denizen)? {
        world.denizen_refresh(actor)?;
    }
    if world.needs_settlement_zone(actor)?
        && !world.gtid_live(world.associated_object(actor)?)?
        && world.create_zone2(actor)?
    {
        let radius = world.discovery_radius(pc)?;
        let xy = world.position_xy(actor)?;
        let zone = world.zone2(actor)?.ok_or(Error::NullDereference(
            "4D7435 zone2 unavailable after successful create",
        ))?;
        world.move_zone(zone, xy, radius)?;
    }
    if world.sai_present()? && world.thing_flags(actor)? & 1 == 0 {
        world.sai_notify(actor)?;
    }
    tracks(world, actor, pc)?;
    if world.state_low(actor)? & (1 << 21) == 0 {
        world.existing_fx(actor)?;
    }
    Ok(true)
}
