//! Original Object/Actor position-change orchestration and small state leaves.
//! Component handles refer to the caller's authoritative component registry.
//! Required world mutations are fallible services, never omitted callbacks.
use crate::{
    Error, Result,
    collision::Shape,
    geometry::{Placement, marks_blocker},
    index::{ActorKey, SpatialIndex},
};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};

pub type ComponentKey = u32;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Components {
    pub organization: Option<ComponentKey>,
    pub element: Option<ComponentKey>,
    pub mover: Option<ComponentKey>,
    pub settlement: Option<ComponentKey>,
    pub history: Option<ComponentKey>,
    pub follower_reference: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionHistory {
    pub samples: Vec<[u32; 3]>,
    pub read_index: i32,
    pub write_index: i32,
    pub reset_latch: bool,
    pub last_frame: i32,
}
impl PositionHistory {
    /// Original50A710. The cursor resets only on the false→true transition.
    pub fn set_reset(&mut self, requested: bool) {
        if self.reset_latch != requested {
            self.reset_latch = requested;
            if requested {
                self.write_index = self.read_index;
            }
        }
    }
    /// Original50A970, including signed/wrapping frame admission and modulo
    /// ring cursors. Caller supplies the actual SVar interval and world clock.
    pub fn sample(
        &mut self,
        frame: i32,
        interval: i32,
        time_bits: u32,
        xy: [u32; 2],
    ) -> Result<bool> {
        if self.last_frame.wrapping_add(interval) > frame {
            return Ok(false);
        }
        self.last_frame = frame;
        let count = i32::try_from(self.samples.len()).map_err(|_| {
            Error::World("history capacity exceeds native signed representation".into())
        })?;
        if count <= 0
            || self.read_index < 0
            || self.read_index >= count
            || self.write_index < 0
            || self.write_index >= count
        {
            return Err(Error::World("invalid native position-history ring".into()));
        }
        for bits in xy {
            Binary::from_f32_bits(bits)?;
        }
        self.samples[self.write_index as usize] = [xy[0], xy[1], time_bits];
        self.write_index = self.write_index.wrapping_add(1) % count;
        if self.write_index == self.read_index {
            self.read_index = self.read_index.wrapping_add(1) % count;
        }
        Ok(true)
    }
}
///46B000 and inlined4D0F61 compare ordered values: signed zeros are equal,
/// either NaN is different. No approximate epsilon/bitwise equality substitute.
pub fn different_xy(a: [u32; 2], b: [u32; 2]) -> bool {
    f32::from_bits(a[0]) != f32::from_bits(b[0]) || f32::from_bits(a[1]) != f32::from_bits(b[1])
}

/// Native5528F0 footprint expansion. Circle/point preserve unused bounds;
/// rectangles retain the original setter's kind/radius arithmetic and stores.
pub fn expand_footprint(shape: &Shape, padding_bits: u32, p: Precision) -> Result<Shape> {
    let mut out = shape.clone();
    let b = |bits| Binary::from_f32_bits(bits).map_err(Error::from);
    let padding = b(padding_bits)?;
    let zero = Binary::from_i32(0);
    let half = b(0x3f000000)?;
    if shape.kind <= 1 {
        let radius = padding.mul(&half, p)?.add(&b(shape.radius_bits)?, p)?;
        let stored = radius.store_f32()?;
        if radius.compare(&zero).is_gt() {
            out.kind = 1;
            out.radius_bits = stored;
        } else {
            out.kind = 0;
            out.radius_bits = 0;
        }
        return Ok(out);
    }
    if shape.kind > 3 {
        return Err(Error::World("unsupported footprint shape".into()));
    }
    let dimension = |bits| -> Result<Binary> {
        let x = b(bits)?;
        Ok(x.add(&x, p)?.add(&padding, p)?)
    };
    let clamp = |x: Binary| -> Result<Binary> {
        if x.is_negative_nonzero() {
            return Ok(Binary::from_i32(0));
        }
        let x = b(x.store_f32()?)?;
        let max = b(0x7f7fffff)?;
        Ok(if x.compare(&max).is_gt() { max } else { x })
    };
    let (width, height) = if shape.kind == 2 {
        let width = clamp(dimension(shape.bounds[2])?)?;
        (width.clone(), width)
    } else {
        // RECT stores both raw expanded dimensions before its comparisons.
        (
            clamp(b(dimension(shape.bounds[2])?.store_f32()?)?)?,
            clamp(b(dimension(shape.bounds[3])?.store_f32()?)?)?,
        )
    };
    out.kind = if width.compare(&height).is_eq() { 2 } else { 3 };
    let min_x = width.mul(&half.clone().negated(), p)?;
    let max_y = height.mul(&half, p)?;
    out.bounds = [
        min_x.store_f32()?,
        height.mul(&half.clone().negated(), p)?.store_f32()?,
        width.mul(&half, p)?.store_f32()?,
        max_y.store_f32()?,
    ];
    out.radius_bits = max_y
        .mul(&max_y, p)?
        .add(&min_x.mul(&min_x, p)?, p)?
        .sqrt(p)?
        .store_f32()?;
    Ok(out)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeightEdit {
    pub placement: Placement,
    /// Native settings+0=mode,+4=height,+8=margin,+C=min,+10=max.
    pub settings_words: [u32; 5],
    pub flags: [bool; 2],
    pub strength_bits: u32,
    pub trailing_flag: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerrainPaint {
    pub placement: Placement,
    pub variation: u32,
    pub deferred: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Diagnostic {
    Identity(u32),
    X(u32),
    Y(u32),
}

/// Reads without &mut are snapshots. Mutating services must preserve the active
/// actor/component lifetimes and return errors for unsupported effects. Earlier
/// native effects survive a later error. No service has a permissive default.
pub trait World {
    fn components(&self, actor: ActorKey) -> Result<Components>;
    fn session_mode(&self) -> Result<i32>;
    fn world_word_48(&self) -> Result<u32>;
    fn height_margin_bits(&self) -> Result<u32>;
    fn ground_height(
        &mut self,
        index: &mut SpatialIndex,
        position: [u32; 2],
        flags: u32,
    ) -> Result<Binary>;
    fn edit_height(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        edit: HeightEdit,
    ) -> Result<()>;
    fn clear_doodads(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        placement: Placement,
    ) -> Result<()>;
    /// Actual4EA260 lookup, including effective parent/biome definitions.
    fn ground_variation(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
    ) -> Result<Option<u32>>;
    fn footprint_padding_bits(&self, actor: ActorKey) -> Result<u32>;
    fn paint_terrain(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        paint: TerrainPaint,
    ) -> Result<()>;
    /// Actual virtual+124 (ordinary4D6350): fog/terrain/region transition work.
    fn cell_changed(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        old: [i32; 2],
    ) -> Result<()>;
    fn diagnostic(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        value: Diagnostic,
    ) -> Result<()>;
    fn component_owner(&self, component: ComponentKey) -> Result<ActorKey>;
    /// Write real Organization/Regiment+BC/+C0 words; no independent copy.
    fn set_organization_saved_xy(&mut self, component: ComponentKey, xy: [u32; 2]) -> Result<()>;
    /// Actual5705E0→57C800 organization movement service.
    fn element_position_changed(
        &mut self,
        index: &mut SpatialIndex,
        component: ComponentKey,
    ) -> Result<()>;
    /// Actual Actor virtual+158 and +15C, e.g.aa_actors::attachment helpers.
    fn is_attached(&mut self, index: &mut SpatialIndex, actor: ActorKey) -> Result<bool>;
    fn attachment_parent(&mut self, index: &mut SpatialIndex, actor: ActorKey) -> Result<ActorKey>;
    fn settlement_attached_actor(&self, component: ComponentKey) -> Result<Option<ActorKey>>;
    /// Calls actual4EEB40, including this position callback recursively.
    fn move_ground(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        position: [u32; 2],
    ) -> Result<()>;
    /// Definition+270/+274 attachment offset for the mover's owner.
    fn follower_offset(&self, owner: ActorKey) -> Result<[u32; 2]>;
    /// Actual577240 on the resolved follower's mover, not an issued game order.
    fn follower_target(
        &mut self,
        index: &mut SpatialIndex,
        follower: ActorKey,
        position: [u32; 3],
    ) -> Result<()>;
    fn history_mut(&mut self, component: ComponentKey) -> Result<&mut PositionHistory>;
    fn history_interval(&self) -> Result<i32>;
    fn frame(&self) -> Result<i32>;
    fn world_time_bits(&self) -> Result<u32>;
}

///The reached4EB500 services over the current object. Each field and child is
///read only at its native point. The runtime may release short index loans
///before destruction callbacks mutate that same world.
pub trait ObjectEffects {
    fn precision(&self) -> Precision;
    fn flags(&self) -> Result<u32>;
    fn blocking_radius(&self) -> Result<u32>;
    fn cell_size(&self) -> Result<u32>;
    fn position(&self) -> Result<[u32; 2]>;
    fn blocking_shape(&self) -> Result<Shape>;
    fn facing(&self) -> Result<[u32; 2]>;
    fn actor_category(&self) -> Result<bool>;
    fn wall_present(&self) -> Result<bool>;
    fn session_mode(&self) -> Result<i32>;
    fn world_word_48(&self) -> Result<u32>;
    fn height_margin_bits(&self) -> Result<u32>;
    fn ground_height(&mut self, position: [u32; 2], flags: u32) -> Result<Binary>;
    fn edit_height(&mut self, edit: HeightEdit) -> Result<()>;
    fn clear_doodads(&mut self, placement: Placement) -> Result<()>;
    fn ground_variation(&mut self) -> Result<Option<u32>>;
    fn footprint_padding_bits(&self) -> Result<u32>;
    fn paint_terrain(&mut self, paint: TerrainPaint) -> Result<()>;
}
///4EB52D..579 for a genuine ActorParams (A25B28 vf0=true). The+4C GTID
///is not read unless mode6 is reached. Invalid/retired references fall back to
///the ground sampler; a valid referenced Thing supplies its current+28 word.
pub fn actor_post_create_height(
    mode: u32,
    reference: impl FnOnce() -> Result<u32>,
    mut referenced_height: impl FnMut(u32) -> Result<Option<Binary>>,
    ground: impl FnOnce() -> Result<Binary>,
) -> Result<Binary> {
    if mode == 6 {
        let reference = reference()?;
        if reference != 0
            && let Some(height) = referenced_height(reference)?
        {
            return Ok(height);
        }
    }
    ground()
}
struct PositionObjectEffects<'a, W> {
    index: &'a mut SpatialIndex,
    actor: ActorKey,
    world: &'a mut W,
}
impl<W: World> ObjectEffects for PositionObjectEffects<'_, W> {
    fn precision(&self) -> Precision {
        self.index.precision()
    }
    fn flags(&self) -> Result<u32> {
        Ok(self.index.member(self.actor)?.actor.flags)
    }
    fn blocking_radius(&self) -> Result<u32> {
        Ok(self.index.member(self.actor)?.blocking.radius_bits)
    }
    fn cell_size(&self) -> Result<u32> {
        Ok(self.index.grid().cell_size_bits)
    }
    fn position(&self) -> Result<[u32; 2]> {
        Ok(self.index.member(self.actor)?.actor.position)
    }
    fn blocking_shape(&self) -> Result<Shape> {
        Ok(self.index.member(self.actor)?.blocking.clone())
    }
    fn facing(&self) -> Result<[u32; 2]> {
        Ok(self.index.member(self.actor)?.actor.facing)
    }
    fn actor_category(&self) -> Result<bool> {
        Ok(self.index.member(self.actor)?.actor_category)
    }
    fn wall_present(&self) -> Result<bool> {
        Ok(self.index.member(self.actor)?.wall_present)
    }
    fn session_mode(&self) -> Result<i32> {
        World::session_mode(self.world)
    }
    fn world_word_48(&self) -> Result<u32> {
        World::world_word_48(self.world)
    }
    fn height_margin_bits(&self) -> Result<u32> {
        World::height_margin_bits(self.world)
    }
    fn ground_height(&mut self, p: [u32; 2], f: u32) -> Result<Binary> {
        World::ground_height(self.world, self.index, p, f)
    }
    fn edit_height(&mut self, e: HeightEdit) -> Result<()> {
        World::edit_height(self.world, self.index, self.actor, e)
    }
    fn clear_doodads(&mut self, p: Placement) -> Result<()> {
        World::clear_doodads(self.world, self.index, self.actor, p)
    }
    fn ground_variation(&mut self) -> Result<Option<u32>> {
        World::ground_variation(self.world, self.index, self.actor)
    }
    fn footprint_padding_bits(&self) -> Result<u32> {
        World::footprint_padding_bits(self.world, self.actor)
    }
    fn paint_terrain(&mut self, p: TerrainPaint) -> Result<()> {
        World::paint_terrain(self.world, self.index, self.actor, p)
    }
}
///4EB500 null-context branch used by every4EB820 position notification.
pub fn object_effects<W: World>(
    index: &mut SpatialIndex,
    actor: ActorKey,
    world: &mut W,
) -> Result<()> {
    object_effects_on_owners(&mut PositionObjectEffects {
        index,
        actor,
        world,
    })
}
fn current_placement(world: &impl ObjectEffects) -> Result<Placement> {
    //Object+0C yields its current immutable blocking shape. Following field
    //reads have no mutating native callback between them.
    Ok(Placement {
        shape: world.blocking_shape()?,
        facing: world.facing()?,
        position: world.position()?,
    })
}
///One native orchestration for both null and genuine nonnull Params callers.
///The height service decides the reached Params branch; all later flag,
///shape, wall, variation and Session reads retain the original order.
pub fn object_effects_on_owners(world: &mut impl ObjectEffects) -> Result<()> {
    let flags = world.flags()?;
    if flags & 0x20000400 == 0x20000400
        && marks_blocker(
            flags,
            world.blocking_radius()?,
            world.cell_size()?,
            world.precision(),
        )?
        && world.flags()? & (1 << 13) != 0
    {
        let position = world.position()?;
        let height = world.ground_height(position, 0x122)?.store_f32()?;
        let margin = world.height_margin_bits()?;
        let placement = current_placement(world)?;
        world.edit_height(HeightEdit {
            placement,
            settings_words: [0, height, margin, 0xff7fffff, 0x7f7fffff],
            flags: [true, true],
            strength_bits: 0x3f800000,
            trailing_flag: false,
        })?;
    }
    if world.flags()? & 0x20000400 == 0x20000400 {
        let placement = current_placement(world)?;
        world.clear_doodads(placement)?;
    }
    if world.world_word_48()? == 0
        && let Some(variation) = world.ground_variation()?
    {
        if !world.actor_category()? || !world.wall_present()? {
            let position = world.position()?;
            let shape = world.blocking_shape()?;
            let padding = world.footprint_padding_bits()?;
            let shape = expand_footprint(&shape, padding, world.precision())?;
            let deferred = world.session_mode()? != 3;
            let facing = world.facing()?;
            world.paint_terrain(TerrainPaint {
                placement: Placement {
                    shape,
                    facing,
                    position,
                },
                variation,
                deferred,
            })?;
        }
    }
    Ok(())
}
///Short current-owner reads for4EB820. No mutable index loan spans a child.
pub trait ObjectPositionWorld {
    fn set_membership_request(&mut self, actor: ActorKey, value: bool) -> Result<()>;
    fn object_effects(&mut self, actor: ActorKey) -> Result<()>;
    fn cached_tile(&self, actor: ActorKey) -> Result<[i32; 2]>;
    fn position(&self, actor: ActorKey) -> Result<[u32; 2]>;
    fn coordinate_shift(&self) -> Result<u32>;
    fn store_cached_tile(&mut self, actor: ActorKey, tile: [i32; 2]) -> Result<()>;
    fn cell_changed(&mut self, actor: ActorKey, old: [i32; 2]) -> Result<()>;
}
///4EB820 after position words have already been committed by4EE5C0.
pub fn object_position_changed_on_owners(
    world: &mut impl ObjectPositionWorld,
    actor: ActorKey,
) -> Result<()> {
    world.set_membership_request(actor, false)?;
    world.object_effects(actor)?;
    let old = world.cached_tile(actor)?;
    let xy = world.position(actor)?;
    let shift = world.coordinate_shift()? & 31;
    let next = [
        Binary::from_f32_bits(xy[0])?.truncate_i32()? >> shift,
        Binary::from_f32_bits(xy[1])?.truncate_i32()? >> shift,
    ];
    world.store_cached_tile(actor, next)?;
    if next != old {
        world.cell_changed(actor, old)?;
    }
    Ok(())
}
struct PositionChanges<'a, W> {
    index: &'a mut SpatialIndex,
    world: &'a mut W,
}
impl<W: World> ObjectPositionWorld for PositionChanges<'_, W> {
    fn set_membership_request(&mut self, actor: ActorKey, value: bool) -> Result<()> {
        self.index.set_ordinary_membership_request(actor, value)
    }
    fn object_effects(&mut self, actor: ActorKey) -> Result<()> {
        object_effects(self.index, actor, self.world)
    }
    fn cached_tile(&self, actor: ActorKey) -> Result<[i32; 2]> {
        Ok(self.index.member(actor)?.cached_tile)
    }
    fn position(&self, actor: ActorKey) -> Result<[u32; 2]> {
        Ok(self.index.member(actor)?.actor.position)
    }
    fn coordinate_shift(&self) -> Result<u32> {
        Ok(self.index.grid().coordinate_shift)
    }
    fn store_cached_tile(&mut self, actor: ActorKey, tile: [i32; 2]) -> Result<()> {
        self.index.member_mut(actor)?.cached_tile = tile;
        Ok(())
    }
    fn cell_changed(&mut self, actor: ActorKey, old: [i32; 2]) -> Result<()> {
        World::cell_changed(self.world, self.index, actor, old)
    }
}
///Legacy index-taking entry delegates to the same borrowed-owner algorithm.
pub fn object_position_changed<W: World>(
    index: &mut SpatialIndex,
    actor: ActorKey,
    world: &mut W,
) -> Result<()> {
    object_position_changed_on_owners(&mut PositionChanges { index, world }, actor)
}
///5771B0 source mover owner→follower target point, before577240 transforms it.
pub fn follower_point(index: &SpatialIndex, owner: ActorKey, offset: [u32; 2]) -> Result<[u32; 3]> {
    let m = index.member(owner)?;
    let p = index.precision();
    let b = |bits| Binary::from_f32_bits(bits).map_err(Error::from);
    let x = b(offset[0])?;
    let y = b(offset[1])?;
    let fx = b(m.actor.facing[0])?;
    let fy = b(m.actor.facing[1])?;
    Ok([
        x.mul(&fy, p)?
            .add(&y.mul(&fx, p)?, p)?
            .add(&b(m.actor.position[0])?, p)?
            .store_f32()?,
        y.mul(&fy, p)?
            .sub(&x.mul(&fx, p)?, p)?
            .add(&b(m.actor.position[1])?, p)?
            .store_f32()?,
        m.height_bits,
    ])
}
///Actual4D0EE0 fields and callbacks on current owners. Each read happens at
///its native point, including rereads after potentially mutating children.
pub trait ActorPositionWorld: ObjectPositionWorld {
    fn reference(&self, actor: ActorKey) -> Result<u32>;
    fn flags(&self, actor: ActorKey) -> Result<u32>;
    fn components(&self, actor: ActorKey) -> Result<Components>;
    fn session_mode(&self) -> Result<i32>;
    fn diagnostic(&mut self, actor: ActorKey, value: Diagnostic) -> Result<()>;
    fn component_owner(&self, component: ComponentKey) -> Result<ActorKey>;
    fn set_organization_saved_xy(&mut self, component: ComponentKey, xy: [u32; 2]) -> Result<()>;
    fn element_position_changed(&mut self, component: ComponentKey) -> Result<()>;
    fn is_attached(&mut self, actor: ActorKey) -> Result<bool>;
    fn attachment_parent(&mut self, actor: ActorKey) -> Result<ActorKey>;
    fn settlement_attached_actor(&self, component: ComponentKey) -> Result<Option<ActorKey>>;
    fn move_ground(&mut self, actor: ActorKey, position: [u32; 2]) -> Result<()>;
    fn resolve_reference(&self, reference: u32) -> Result<Option<ActorKey>>;
    fn follower_offset(&self, owner: ActorKey) -> Result<[u32; 2]>;
    fn follower_point(&self, owner: ActorKey, offset: [u32; 2]) -> Result<[u32; 3]>;
    fn follower_target(&mut self, follower: ActorKey, position: [u32; 3]) -> Result<()>;
    fn history_mut(&mut self, component: ComponentKey) -> Result<&mut PositionHistory>;
    fn history_interval(&self) -> Result<i32>;
    fn frame(&self) -> Result<i32>;
    fn world_time_bits(&self) -> Result<u32>;
}
impl<W: World> ActorPositionWorld for PositionChanges<'_, W> {
    fn reference(&self, actor: ActorKey) -> Result<u32> {
        Ok(self.index.member(actor)?.reference)
    }
    fn flags(&self, actor: ActorKey) -> Result<u32> {
        Ok(self.index.member(actor)?.actor.flags)
    }
    fn components(&self, actor: ActorKey) -> Result<Components> {
        World::components(self.world, actor)
    }
    fn session_mode(&self) -> Result<i32> {
        World::session_mode(self.world)
    }
    fn diagnostic(&mut self, actor: ActorKey, value: Diagnostic) -> Result<()> {
        World::diagnostic(self.world, self.index, actor, value)
    }
    fn component_owner(&self, component: ComponentKey) -> Result<ActorKey> {
        World::component_owner(self.world, component)
    }
    fn set_organization_saved_xy(&mut self, component: ComponentKey, xy: [u32; 2]) -> Result<()> {
        World::set_organization_saved_xy(self.world, component, xy)
    }
    fn element_position_changed(&mut self, component: ComponentKey) -> Result<()> {
        World::element_position_changed(self.world, self.index, component)
    }
    fn is_attached(&mut self, actor: ActorKey) -> Result<bool> {
        World::is_attached(self.world, self.index, actor)
    }
    fn attachment_parent(&mut self, actor: ActorKey) -> Result<ActorKey> {
        World::attachment_parent(self.world, self.index, actor)
    }
    fn settlement_attached_actor(&self, component: ComponentKey) -> Result<Option<ActorKey>> {
        World::settlement_attached_actor(self.world, component)
    }
    fn move_ground(&mut self, actor: ActorKey, position: [u32; 2]) -> Result<()> {
        World::move_ground(self.world, self.index, actor, position)
    }
    fn resolve_reference(&self, reference: u32) -> Result<Option<ActorKey>> {
        Ok(self.index.resolve_reference(reference))
    }
    fn follower_offset(&self, owner: ActorKey) -> Result<[u32; 2]> {
        World::follower_offset(self.world, owner)
    }
    fn follower_point(&self, owner: ActorKey, offset: [u32; 2]) -> Result<[u32; 3]> {
        follower_point(self.index, owner, offset)
    }
    fn follower_target(&mut self, follower: ActorKey, position: [u32; 3]) -> Result<()> {
        World::follower_target(self.world, self.index, follower, position)
    }
    fn history_mut(&mut self, component: ComponentKey) -> Result<&mut PositionHistory> {
        World::history_mut(self.world, component)
    }
    fn history_interval(&self) -> Result<i32> {
        World::history_interval(self.world)
    }
    fn frame(&self) -> Result<i32> {
        World::frame(self.world)
    }
    fn world_time_bits(&self) -> Result<u32> {
        World::world_time_bits(self.world)
    }
}
///Legacy index-taking entry delegates to the same4D0EE0 algorithm.
pub fn actor_position_changed<W: World>(
    index: &mut SpatialIndex,
    actor: ActorKey,
    world: &mut W,
) -> Result<()> {
    actor_position_changed_on_owners(&mut PositionChanges { index, world }, actor)
}
/// Full4D0EE0 orchestration with short field loans around each real child.
pub fn actor_position_changed_on_owners(
    world: &mut impl ActorPositionWorld,
    actor: ActorKey,
) -> Result<()> {
    object_position_changed_on_owners(world, actor)?;
    world.diagnostic(actor, Diagnostic::Identity(world.reference(actor)?))?;
    world.diagnostic(actor, Diagnostic::X(world.position(actor)?[0]))?;
    world.diagnostic(actor, Diagnostic::Y(world.position(actor)?[1]))?;
    if let Some(component) = world.components(actor)?.organization
        && world.session_mode()? == 3
    {
        let owner = world.component_owner(component)?;
        let xy = world.position(owner)?;
        world.set_organization_saved_xy(component, xy)?;
    }
    if let Some(component) = world.components(actor)?.element {
        world.element_position_changed(component)?;
    }
    if world.is_attached(actor)? {
        let parent = world.attachment_parent(actor)?;
        if different_xy(world.position(parent)?, world.position(actor)?) {
            //4D0F83 captures XY before its second virtual+15C dispatch.
            let xy = world.position(actor)?;
            let parent = world.attachment_parent(actor)?;
            world.move_ground(parent, xy)?;
        }
    } else if let Some(settlement) = world.components(actor)?.settlement
        && let Some(other) = world.settlement_attached_actor(settlement)?
        && different_xy(world.position(other)?, world.position(actor)?)
    {
        let xy = world.position(actor)?;
        let settlement = world.components(actor)?.settlement.ok_or_else(|| {
            Error::World("settlement disappeared during native attachment comparison".into())
        })?;
        let other = world
            .settlement_attached_actor(settlement)?
            .ok_or_else(|| {
                Error::World("settlement actor disappeared during native comparison".into())
            })?;
        world.move_ground(other, xy)?;
    }
    if let Some(follower) = world.resolve_reference(world.components(actor)?.follower_reference)?
        && let Some(mover) = world.components(actor)?.mover
    {
        let owner = world.component_owner(mover)?;
        let position = world.follower_point(owner, world.follower_offset(owner)?)?;
        world.follower_target(follower, position)?;
    }
    if world.flags(actor)? & (1 << 30) != 0
        && let Some(history) = world.components(actor)?.history
    {
        world.history_mut(history)?.set_reset(true);
    }
    if let Some(history) = world.components(actor)?.history {
        let interval = world.history_interval()?;
        let frame = world.frame()?;
        if world
            .history_mut(history)?
            .last_frame
            .wrapping_add(interval)
            <= frame
        {
            let xy = world.position(actor)?;
            let time = world.world_time_bits()?;
            world
                .history_mut(history)?
                .sample(frame, interval, time, xy)?;
        }
    }
    Ok(())
}

///4EEB40/4EE5C0 composition on one retained owner. The field prefix includes
///the existing numeric height/snap and tile-update kernels. Its index loan
///ends before+C8 and the lazily reached50C250 view notification.
pub trait GroundRelocationWorld: ActorPositionWorld {
    fn move_ground_fields(&mut self, actor: ActorKey, point: [u32; 2]) -> Result<()>;
    fn world_phase(&self) -> Result<u32>;
    fn presentation_available(&self) -> Result<bool>;
    fn presentation_position_changed(&mut self, actor: ActorKey) -> Result<()>;
}
pub fn relocate_on_owners(
    world: &mut impl GroundRelocationWorld,
    actor: ActorKey,
    point: [u32; 2],
) -> Result<()> {
    world.move_ground_fields(actor, point)?;
    actor_position_changed_on_owners(world, actor)?;
    if world.world_phase()? != 6 && world.presentation_available()? {
        world.presentation_position_changed(actor)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "position_ground_tests.rs"]
mod ground_tests;
