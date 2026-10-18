//! NonNULL4D7230/4EBE50 over actual fields, independent of allocation layout.
//! The legacy LoadShell adapter and the split live arena use this SAME kernel.
//! Every reached world service is mandatory; no saved Init(NULL) is called.
use crate::{
    actor_creation::{ActorParams, Error, Result, Shape, TrackInfo, track_period},
    components::{ALL_SLOTS, ComponentSlot},
};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug)]
pub enum Diagnostic {
    Thin,
    Placement { code: u32, xy: [u32; 2] },
    DoesntFit { xy: [u32; 2] },
    RequiredDead,
    RequiredIgnored,
    RequiredPlaced(u32),
}
/// The actual GObject4EBE50 fields. A fresh Object never needs an Actor
/// component table merely to execute this common native body.
pub trait ObjectWorld<D, L> {
    type Actor: Copy;
    fn word(&self, a: Self::Actor, offset: u32) -> Result<u32>;
    fn set_word(&mut self, a: Self::Actor, offset: u32, value: u32) -> Result<()>;
    fn clear_byte_4c(&mut self, a: Self::Actor) -> Result<()>;
    fn definition_height_bits(&self, a: Self::Actor) -> Result<u32>;
    fn shape_a(&self, a: Self::Actor) -> Result<Shape>;
    fn tile_size_bits(&self) -> Result<u32>;
    fn tile_shift(&self) -> Result<u32>;
    fn diagnostic(&mut self, a: Self::Actor, d: Diagnostic) -> Result<()>;
    fn snap(&mut self, a: Self::Actor, shape: &Shape, xy: [u32; 2], tile: u32) -> Result<[u32; 2]>;
    fn placement_test(&mut self, a: Self::Actor, params: &ActorParams<D, L>) -> Result<u32>;
    fn map_mode(&self) -> Result<u32>;
    fn spatial_membership(&mut self, a: Self::Actor) -> Result<()>;
    fn terrain_height(&mut self, xy: [u32; 2], flags: u32) -> Result<Binary>;
    fn blocking_notify(&mut self, a: Self::Actor) -> Result<()>;
    fn tile_footprint(&mut self, a: Self::Actor) -> Result<()>;
    fn world_list_insert(&mut self, a: Self::Actor) -> Result<()>;
    fn memory_refresh(&mut self, a: Self::Actor) -> Result<()>;
    fn visibility_refresh(&mut self, a: Self::Actor) -> Result<()>;
}
struct ObjectAdapter<'a, W>(&'a mut W);
impl<D,L,W:World<D,L>> ObjectWorld<D,L> for ObjectAdapter<'_,W> {
    type Actor=W::Actor;
    fn word(&self, a: Self::Actor, offset: u32) -> Result<u32> { World::word(self.0, a, offset) }
    fn set_word(&mut self, a: Self::Actor, offset: u32, value: u32) -> Result<()> { World::set_word(self.0, a, offset, value) }
    fn clear_byte_4c(&mut self, a: Self::Actor) -> Result<()> { World::clear_byte_4c(self.0, a) }
    fn definition_height_bits(&self, a: Self::Actor) -> Result<u32> { World::definition_height_bits(self.0, a) }
    fn shape_a(&self, a: Self::Actor) -> Result<Shape> { World::shape_a(self.0, a) }
    fn tile_size_bits(&self) -> Result<u32> { World::tile_size_bits(self.0) }
    fn tile_shift(&self) -> Result<u32> { World::tile_shift(self.0) }
    fn diagnostic(&mut self, a: Self::Actor, d: Diagnostic) -> Result<()> { World::diagnostic(self.0, a, d) }
    fn snap(&mut self, a: Self::Actor, shape: &Shape, xy: [u32; 2], tile: u32) -> Result<[u32; 2]> { World::snap(self.0, a, shape, xy, tile) }
    fn placement_test(&mut self, a: Self::Actor, params: &ActorParams<D, L>) -> Result<u32> { World::placement_test(self.0, a, params) }
    fn map_mode(&self) -> Result<u32> { World::map_mode(self.0) }
    fn spatial_membership(&mut self, a: Self::Actor) -> Result<()> { World::spatial_membership(self.0, a) }
    fn terrain_height(&mut self, xy: [u32; 2], flags: u32) -> Result<Binary> { World::terrain_height(self.0, xy, flags) }
    fn blocking_notify(&mut self, a: Self::Actor) -> Result<()> { World::blocking_notify(self.0, a) }
    fn tile_footprint(&mut self, a: Self::Actor) -> Result<()> { World::tile_footprint(self.0, a) }
    fn world_list_insert(&mut self, a: Self::Actor) -> Result<()> { World::world_list_insert(self.0, a) }
    fn memory_refresh(&mut self, a: Self::Actor) -> Result<()> { World::memory_refresh(self.0, a) }
    fn visibility_refresh(&mut self, a: Self::Actor) -> Result<()> { World::visibility_refresh(self.0, a) }
}
pub trait World<D, L> {
    type Actor: Copy;
    fn word(&self, a: Self::Actor, offset: u32) -> Result<u32>;
    fn set_word(&mut self, a: Self::Actor, offset: u32, value: u32) -> Result<()>;
    fn clear_byte_4c(&mut self, a: Self::Actor) -> Result<()>;
    fn definition_height_bits(&self, a: Self::Actor) -> Result<u32>;
    fn shape_a(&self, a: Self::Actor) -> Result<Shape>;
    fn tile_size_bits(&self) -> Result<u32>;
    fn tile_shift(&self) -> Result<u32>;
    fn diagnostic(&mut self, a: Self::Actor, d: Diagnostic) -> Result<()>;
    fn snap(&mut self, a: Self::Actor, shape: &Shape, xy: [u32; 2], tile: u32) -> Result<[u32; 2]>;
    fn placement_test(&mut self, a: Self::Actor, params: &ActorParams<D, L>) -> Result<u32>;
    fn map_mode(&self) -> Result<u32>;
    fn spatial_membership(&mut self, a: Self::Actor) -> Result<()>;
    fn terrain_height(&mut self, xy: [u32; 2], flags: u32) -> Result<Binary>;
    fn blocking_notify(&mut self, a: Self::Actor) -> Result<()>;
    fn tile_footprint(&mut self, a: Self::Actor) -> Result<()>;
    fn world_list_insert(&mut self, a: Self::Actor) -> Result<()>;
    fn memory_refresh(&mut self, a: Self::Actor) -> Result<()>;
    fn visibility_refresh(&mut self, a: Self::Actor) -> Result<()>;
    fn required_object_enabled(&self, a: Self::Actor) -> Result<bool>;
    fn find_required_object(&mut self, a: Self::Actor, xy: [u32; 2]) -> Result<Option<u32>>;
    fn gtid_live(&self, id: u32) -> Result<bool>;
    fn required_placement_blocked(&mut self, required: u32, a: Self::Actor) -> Result<bool>;
    fn required_object_attach(&mut self, required: u32, a: Self::Actor) -> Result<()>;
    fn has_component(&self, a: Self::Actor, slot: ComponentSlot) -> Result<bool>;
    fn component_init_existing(&mut self, a: Self::Actor, slot: ComponentSlot) -> Result<bool>;
    fn component_after_init(&mut self, a: Self::Actor, slot: ComponentSlot) -> Result<()>;
    fn refresh_4d4cf0(&mut self, a: Self::Actor) -> Result<()>;
    fn refresh_4d1350(&mut self, a: Self::Actor) -> Result<()>;
    fn refresh_4d67f0(&mut self, a: Self::Actor) -> Result<()>;
    fn global_property(&self) -> Result<Option<u32>>;
    fn property_applies(&mut self, a: Self::Actor, p: u32) -> Result<bool>;
    fn properties_add(&mut self, a: Self::Actor, p: u32) -> Result<()>;
    fn refresh_4d4130(&mut self, a: Self::Actor) -> Result<()>;
    fn organization_refresh(&mut self, a: Self::Actor) -> Result<()>;
    fn settlement_refresh(&mut self, a: Self::Actor) -> Result<()>;
    fn performer_refresh(&mut self, a: Self::Actor) -> Result<()>;
    fn denizen_refresh(&mut self, a: Self::Actor) -> Result<()>;
    fn needs_settlement_zone(&self, a: Self::Actor) -> Result<bool>;
    fn create_zone2(&mut self, a: Self::Actor) -> Result<bool>;
    fn discovery_radius(&self, pc: Precision) -> Result<u32>;
    fn zone_count(&self, a: Self::Actor) -> Result<i32>;
    fn move_zone2(&mut self, a: Self::Actor, xy: [u32; 2], radius: u32) -> Result<()>;
    fn sai_present(&self) -> Result<bool>;
    fn sai_notify(&mut self, a: Self::Actor) -> Result<()>;
    fn tracks(&self, a: Self::Actor) -> Result<Vec<TrackInfo>>;
    fn track_settings(&self) -> Result<(u32, i32)>;
    fn track_updatable(&mut self, a: Self::Actor, period: i32) -> Result<()>;
    fn state_low(&self, a: Self::Actor) -> Result<u32>;
    fn existing_fx(&mut self, a: Self::Actor) -> Result<()>;
}
fn f32b(v: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(v)?)
}
fn ftol(v: &Binary) -> Result<i32> {
    v.truncate_i32()
        .map_err(|_| Error::Domain("Object Init _ftol2 outside i32"))
}
fn position<D, L, W: World<D, L>>(w: &W, a: W::Actor) -> Result<[u32; 3]> {
    Ok([w.word(a, 0x20)?, w.word(a, 0x24)?, w.word(a, 0x28)?])
}

///The actual nonNULL4EFA40 parameter writebacks precede placement callbacks.
fn object_position<D,L,W:ObjectWorld<D,L>>(w:&W,a:W::Actor)->Result<[u32;3]> {
    Ok([w.word(a,0x20)?,w.word(a,0x24)?,w.word(a,0x28)?])
}
fn object_place<D, L, W: ObjectWorld<D, L>>(
    a: W::Actor,
    params: &mut ActorParams<D, L>,
    w: &mut W,
    tile: u32,
) -> Result<bool> {
    if w.word(a, 8)? & (1 << 16) != 0 {
        let p = object_position(w, a)?;
        let xy = w.snap(a, &w.shape_a(a)?, [p[0], p[1]], tile)?;
        w.set_word(a, 0x20, xy[0])?;
        w.set_word(a, 0x24, xy[1])?;
    }
    params.position_bits = object_position(w, a)?;
    params.direction_bits = [w.word(a, 0x2c)?, w.word(a, 0x30)?];
    let code = w.placement_test(a, params)?;
    if code != 0 {
        let p = object_position(w, a)?;
        let xy = [p[0], p[1]];
        w.diagnostic(a, Diagnostic::Placement { code, xy })?;
        if matches!(code, 1 | 2) && w.map_mode()? == 2 {
            w.diagnostic(a, Diagnostic::DoesntFit { xy })?;
        } else {
            return Ok(false);
        }
    }
    w.spatial_membership(a)?;
    let flags = w.word(a, 8)?;
    let height = if flags & (1 << 21) != 0 {
        Some(0x123)
    } else if flags & (1 << 12) == 0 {
        Some(0x122)
    } else {
        None
    };
    if let Some(flags) = height {
        let p = object_position(w, a)?;
        let z = w.terrain_height([p[0], p[1]], flags)?.store_f32()?;
        w.set_word(a, 0x28, z)?;
    }
    Ok(true)
}
pub fn object_initialize_fields<D, L, W: ObjectWorld<D, L>>(
    a: W::Actor,
    params: &mut ActorParams<D, L>,
    w: &mut W,
    pc: Precision,
) -> Result<bool> {
    let height = f32b(w.word(a, 0x3c)?)?.mul(&f32b(w.definition_height_bits(a)?)?, pc)?;
    //4EBE59/5C calculate before4EBE5F clears the byte. Native FSTP4EBE63
    //then stores the product; a reached service refusal keeps that prefix.
    w.clear_byte_4c(a)?;
    w.set_word(a, 0x38, height.store_f32()?)?;
    let tile = w.tile_size_bits()?;
    let tile_int = ftol(&f32b(tile)?)?;
    let quarter = Binary::from_i32(tile_int)
        .mul(&f32b(0x3e800000)?, pc)?
        .store_f32()?;
    if w.word(a, 8)? & 0x20000400 == 0x20000400 {
        let shape = w.shape_a(a)?;
        let r4 = f32b(shape.radius_bits)?.mul(&f32b(0x40800000)?, pc)?;
        if Binary::from_i32(tile_int).compare(&r4) != Ordering::Greater
            && matches!(shape.kind, 2 | 3)
        {
            let x = f32b(shape.extent_10_bits)?;
            let y = f32b(shape.extent_14_bits)?;
            let thin = if x.compare(&y) == Ordering::Less {
                x
            } else {
                y
            };
            if thin.compare(&f32b(quarter)?) == Ordering::Less {
                w.diagnostic(a, Diagnostic::Thin)?;
            }
        }
    }
    if !object_place(a, params, w, tile)? {
        return Ok(false);
    }
    if w.word(a, 8)? & (1 << 10) != 0 {
        w.blocking_notify(a)?;
    }
    w.tile_footprint(a)?;
    let shift = w.tile_shift()? & 31;
    let p = object_position(w, a)?;
    let x = ftol(&f32b(p[0])?)? >> shift;
    let y = ftol(&f32b(p[1])?)? >> shift;
    w.set_word(a, 0x44, x as u32)?;
    w.set_word(a, 0x48, y as u32)?;
    if w.word(a, 8)? & (1 << 8) != 0 {
        w.world_list_insert(a)?;
    }
    w.memory_refresh(a)?;
    w.visibility_refresh(a)?;
    Ok(true)
}
pub fn object_initialize<D,L,W:World<D,L>>(a:W::Actor,params:&mut ActorParams<D,L>,w:&mut W,pc:Precision)->Result<bool> {
    object_initialize_fields(a,params,&mut ObjectAdapter(w),pc)
}
pub fn required_object_initialize<D, L, W: World<D, L>>(a: W::Actor, w: &mut W) -> Result<bool> {
    if !w.required_object_enabled(a)? {
        return Ok(true);
    }
    let required = w.word(a, 0x158)?;
    if required == 0 {
        let p = position(w, a)?;
        return match w.find_required_object(a, [p[0], p[1]])? {
            Some(found) => {
                w.set_word(a, 0x158, found)?;
                Ok(true)
            }
            None => Ok(false),
        };
    }
    if !w.gtid_live(required)? {
        w.diagnostic(a, Diagnostic::RequiredDead)?;
        return Ok(false);
    }
    if w.map_mode()? != 0 {
        w.diagnostic(a, Diagnostic::RequiredIgnored)?;
        return Ok(true);
    }
    if w.required_placement_blocked(required, a)? {
        return Ok(false);
    }
    w.diagnostic(a, Diagnostic::RequiredPlaced(required))?;
    Ok(true)
}
pub fn initialize<D, L, W: World<D, L>>(
    a: W::Actor,
    params: &mut ActorParams<D, L>,
    w: &mut W,
    pc: Precision,
) -> Result<bool> {
    if !object_initialize(a, params, w, pc)? || !required_object_initialize(a, w)? {
        return Ok(false);
    }
    let value = if w.gtid_live(params.word_40)? {
        params.word_40
    } else if w.gtid_live(params.word_44)? {
        params.word_44
    } else {
        0
    };
    w.set_word(a, 0x138, value)?;
    for slot in ALL_SLOTS {
        if slot != ComponentSlot::Mind
            && w.has_component(a, slot)?
            && !w.component_init_existing(a, slot)?
        {
            return Ok(false);
        }
    }
    if w.has_component(a, ComponentSlot::Mind)?
        && !w.component_init_existing(a, ComponentSlot::Mind)?
    {
        return Ok(false);
    }
    for slot in ALL_SLOTS {
        if w.has_component(a, slot)? {
            w.component_after_init(a, slot)?;
        }
    }
    let required = w.word(a, 0x158)?;
    if required != 0 {
        if !w.gtid_live(required)? {
            return Err(Error::NullDereference(
                "4D7320: +158 object virtual128 on a stale GTID",
            ));
        }
        w.required_object_attach(required, a)?;
    }
    w.refresh_4d4cf0(a)?;
    w.refresh_4d1350(a)?;
    w.refresh_4d67f0(a)?;
    if w.word(a, 8)? & (1 << 29) == 0
        && let Some(p) = w.global_property()?
        && w.has_component(a, ComponentSlot::Properties)?
        && w.property_applies(a, p)?
    {
        w.properties_add(a, p)?;
    }
    w.refresh_4d4130(a)?;
    if w.has_component(a, ComponentSlot::Organization)? {
        w.organization_refresh(a)?;
    }
    if w.has_component(a, ComponentSlot::Settlement)? {
        w.settlement_refresh(a)?;
    }
    if w.has_component(a, ComponentSlot::Performer)? {
        w.performer_refresh(a)?;
    }
    if w.has_component(a, ComponentSlot::Denizen)? {
        w.denizen_refresh(a)?;
    }
    if w.needs_settlement_zone(a)? && !w.gtid_live(w.word(a, 0x50)?)? && w.create_zone2(a)? {
        let radius = w.discovery_radius(pc)?;
        let p = position(w, a)?;
        if w.zone_count(a)? <= 2 {
            return Err(Error::NullDereference("4D7435:515F40 on a null zone"));
        }
        w.move_zone2(a, [p[0], p[1]], radius)?;
    }
    if w.sai_present()? && w.word(a, 8)? & 1 == 0 {
        w.sai_notify(a)?;
    }
    let tracks = w.tracks(a)?;
    if !tracks.is_empty() {
        let (scale, divisor) = w.track_settings()?;
        if let Some(period) = track_period(&tracks, scale, divisor, pc)? {
            w.track_updatable(a, period)?;
        }
    }
    if w.state_low(a)? & (1 << 21) == 0 && params.requested_id == params.word_3c {
        w.existing_fx(a)?;
    }
    Ok(true)
}
