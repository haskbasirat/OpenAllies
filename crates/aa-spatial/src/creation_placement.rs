//! Original GWorld `4F3640` creation/load placement coordinator.
//!
//! This is distinct from movement's `4F3C40`. Every reached world service is
//! mandatory. The caller owns the current GACD descriptor and live object/source
//! storage; callbacks may change them, so later native loads are not snapshots.
use crate::{
    Result,
    collision::{Grid, Shape},
    geometry::Placement,
};
use aa_numeric::{Binary, Precision};

/// Native 554DD0/554330/5541F0 mode-zero cells for an immutable GMap and local
/// placed shape. The center cell is retained even if the exact shape test is
/// false. This is geometry only, never a snapshot of runtime tile/zone contents.
pub fn candidate_tiles(
    grid: &Grid,
    extents: [u32; 2],
    cell_shape: &Shape,
    placement: &Placement,
    p: Precision,
) -> Result<Vec<[i32; 2]>> {
    let rect = crate::geometry::tile_rectangle(grid, placement, p)?;
    let mut center = rect.min;
    let inside = placement
        .position
        .into_iter()
        .zip(extents)
        .all(|(xy, extent)| {
            // The native point checks are comparisons, with no arithmetic/rounding.
            let xy = f32::from_bits(xy);
            let extent = f32::from_bits(extent);
            xy >= 0.0 && xy < extent
        });
    if inside {
        for (a, value) in center.iter_mut().enumerate() {
            *value = Binary::from_f32_bits(placement.position[a])?.truncate_i32()?
                >> (grid.coordinate_shift & 31);
        }
    }
    let half = Binary::from_f32_bits(grid.cell_size_bits)?
        .mul(&Binary::from_f32_bits(0x3f000000)?, p)?
        .store_f32()?;
    let mut cells = Vec::new();
    for y in rect.min[1]..=rect.max[1] {
        for x in rect.min[0]..=rect.max[0] {
            let cell = [x, y];
            if cell == center {
                cells.push(cell);
                continue;
            }
            let mut position = [0; 2];
            for (a, value) in position.iter_mut().enumerate() {
                *value = Binary::from_i32(cell[a].wrapping_shl(grid.coordinate_shift & 31))
                    .add(&Binary::from_f32_bits(half)?, p)?
                    .store_f32()?;
            }
            let tile = Placement {
                shape: cell_shape.clone(),
                facing: [0, 0xbf800000],
                position,
            };
            if crate::shape_overlap::overlaps(placement, &tile, p)? {
                cells.push(cell);
            }
        }
    }
    Ok(cells)
}

/// Native554DD0 mode1 uses the explicitly owned B3A8C4 point shape at each
/// tile center. The global shape's constructor/provenance is a caller-owned
/// requirement; this function supplies no default point or empty occupancy.
pub fn occupancy_tiles(
    grid: &Grid,
    extents: [u32; 2],
    global_point_shape: &Shape,
    placement: &Placement,
    p: Precision,
) -> Result<Vec<[i32; 2]>> {
    if global_point_shape.kind != 0 {
        return Err(crate::Error::World(
            "mode-one occupancy requires proved global point shape".into(),
        ));
    }
    candidate_tiles(grid, extents, global_point_shape, placement, p)
}

/// The six arguments following the GACD descriptor (this is the world).
#[derive(Clone, Copy, Debug)]
pub struct Arguments<A> {
    pub source: Option<A>,
    pub other: Option<A>,
    pub argument_5: u32,
    pub argument_6: u32,
    pub argument_7: u32,
    pub argument_8: Option<A>,
}

/// Defined fields of the native 553090 local collision descriptor. Handles
/// remain typed until the concrete index adapter resolves them.
#[derive(Clone, Debug)]
pub struct Context<D, A> {
    pub placement: Placement,
    pub definition: D,
    pub source_actor: Option<A>,
    pub ignore_actor: Option<A>,
    pub ignore_definition: Option<D>,
    pub collision_owner: u32,
    pub hostility_context: u32,
    /// Native +34 custom-filter dispatch; zero means no custom filter.
    pub custom_filter: u32,
    pub ignore_all: bool,
    pub ignore_moving: bool,
    pub required_actor_flags: u32,
    pub mask: u32,
}

/// Source/descriptor getters load only their named current word. No missing
/// producer may be treated as zero, an empty zone map, or an unblocked tile.
pub trait World {
    type Actor: Copy + Eq;
    type Definition: Copy;
    type Zone: Copy;
    type Tiles;
    fn descriptor_definition(&mut self) -> Result<Self::Definition>;
    fn descriptor_word(&mut self, offset: u32) -> Result<u32>;
    fn descriptor_byte(&mut self, offset: u32) -> Result<u8>;
    /// Definition virtual +30/+38 use the full EAX test; +2C/+3C use AL.
    fn definition_virtual(&mut self, definition: Self::Definition, slot: u32) -> Result<bool>;
    fn definition_word(&mut self, definition: Self::Definition, offset: u32) -> Result<u32>;
    fn definition_byte(&mut self, definition: Self::Definition, offset: u32) -> Result<u8>;
    /// Literal ObjectData+38 when blocking=true, otherwise virtual+28.
    fn definition_shape(
        &mut self,
        definition: Self::Definition,
        blocking: bool,
        p: Precision,
    ) -> Result<Shape>;
    fn definition_reference(
        &mut self,
        definition: Self::Definition,
        offset: u32,
    ) -> Result<Option<Self::Definition>>;
    fn actor_definition(&mut self, actor: Self::Actor) -> Result<Self::Definition>;
    fn actor_word(&mut self, actor: Self::Actor, offset: u32) -> Result<u32>;
    /// Local this-world+28 may be null. Global [B36534]+28 is dereferenced
    /// without a null guard in the late required-object branch.
    fn map_settings_mode(&mut self, global: bool) -> Result<Option<u32>>;
    fn map_cell_size(&mut self) -> Result<u32>;
    fn outside_553370(&mut self, placement: &Placement, p: Precision) -> Result<bool>;
    #[allow(clippy::too_many_arguments)] // Preserve the explicit native ABI.
    fn required_zone_4f1660(
        &mut self,
        position: [u32; 2],
        definition: Self::Definition,
        owner: u32,
        other: Option<Self::Actor>,
        argument_7: u32,
        argument_8: u32,
        p: Precision,
    ) -> Result<u32>;
    /// Actual 432730 -> 554DD0(shape,0,0); current/advance preserve native order.
    fn begin_tiles(&mut self, placement: &Placement, p: Precision) -> Result<Self::Tiles>;
    fn tile_zone_54ef80(&mut self, tiles: &Self::Tiles) -> Result<Option<Self::Zone>>;
    fn advance_tile(&mut self, tiles: &mut Self::Tiles) -> Result<bool>;
    fn tiles_valid(&mut self, tiles: &Self::Tiles) -> Result<bool>;
    fn zone_actor(&mut self, zone: Self::Zone) -> Result<Self::Actor>;
    fn zone_overlap_4f0ae0(
        &mut self,
        actor: Self::Actor,
        placement: &Placement,
        p: Precision,
    ) -> Result<bool>;
    fn actor_category_id(&mut self, actor: Self::Actor) -> Result<u32>;
    fn structure_at_4f2220(&mut self, position: [u32; 2]) -> Result<Option<Self::Actor>>;
    /// Actual 553BA0 plus the mandatory definition virtual+38 call, then the
    /// native hit-kind mapping {0:0,1:1,3:3,otherwise:2}.
    fn collision_4f0a70(
        &mut self,
        context: &mut Context<Self::Definition, Self::Actor>,
        original_definition: Self::Definition,
        p: Precision,
    ) -> Result<u32>;
    fn footprint_4f0b70(&mut self) -> Result<u32>;
}

/// Preserve native branch/read/callback order. All persistent mutations are
/// performed by their actual world owner and survive a later service error.
pub fn check<W: World>(arguments: Arguments<W::Actor>, p: Precision, world: &mut W) -> Result<u32> {
    let definition = world.descriptor_definition()?;
    let object_data = world.definition_virtual(definition, 0x30)?;
    let actor_data = world.definition_virtual(definition, 0x38)?;
    // 4F3672 reads Y before X, then captures both before external dispatch.
    let y = world.descriptor_word(0xc)?;
    let x = world.descriptor_word(8)?;
    let mut last = arguments.argument_8;
    if last.is_none()
        && let Some(other) = arguments.other
    {
        let d = world.actor_definition(other)?;
        if world.definition_virtual(d, 0x2c)? {
            last = Some(other);
        }
    }
    let explicit = world.descriptor_word(0x3c)? != 0;
    let strict = if explicit || world.map_settings_mode(false)?.is_some_and(|v| v != 0) {
        true
    } else if let Some(other) = arguments.other {
        world.actor_word(other, 8)? as i32 >= 0
    } else {
        false
    };
    let blocking = object_data && strict && world.descriptor_word(0x38)? != 3;
    let shape = world.definition_shape(definition, blocking, p)?;
    let facing_x = world.descriptor_word(0x14)?;
    let facing_y = world.descriptor_word(0x18)?;
    let mut context = Context {
        placement: Placement {
            shape,
            facing: [facing_x, facing_y],
            position: [x, y],
        },
        definition,
        source_actor: None,
        ignore_actor: None,
        ignore_definition: None,
        collision_owner: 0,
        hostility_context: 0,
        custom_filter: 0,
        ignore_all: false,
        ignore_moving: false,
        required_actor_flags: 0,
        mask: 0,
    };
    // 552250 always calls definition+30 again, even with its earlier view cached.
    if world.definition_virtual(definition, 0x30)?
        && world.definition_word(definition, 0x1c)? & (1 << 16) != 0
    {
        let cell_size = world.map_cell_size()?;
        crate::snap_half_cell(&mut context.placement.position, cell_size, p)?;
    }
    let position = context.placement.position;
    if strict {
        context.required_actor_flags |= 0x20000000;
    }
    if world.outside_553370(&context.placement, p)? {
        return Ok(3);
    }
    if !explicit && arguments.argument_6 as u8 == 0 && world.definition_virtual(definition, 0x2c)? {
        let owner = world.descriptor_word(0x28)?;
        if world.required_zone_4f1660(
            position,
            definition,
            owner,
            arguments.other.or(last),
            arguments.argument_5,
            1,
            p,
        )? != 0
        {
            return Ok(5);
        }
    }
    if object_data {
        let flags = world.definition_word(definition, 0x1c)?;
        if flags & (1 << 10) != 0 && flags & (1 << 29) != 0 {
            let fx = world.descriptor_word(0x14)?;
            let fy = world.descriptor_word(0x18)?;
            let shape = world.definition_shape(definition, true, p)?;
            let placed = Placement {
                shape,
                facing: [fx, fy],
                position,
            };
            let mut tiles = world.begin_tiles(&placed, p)?;
            let mut zone = None;
            let mut structure = None;
            let mut same_last = false;
            while world.tiles_valid(&tiles)? {
                if let Some(z) = world.tile_zone_54ef80(&tiles)? {
                    let actor = world.zone_actor(z)?;
                    if world.zone_overlap_4f0ae0(actor, &placed, p)? {
                        zone = Some(z);
                        let actor = world.zone_actor(z)?;
                        let d = world.actor_definition(actor)?;
                        let classification = world.definition_virtual(d, 0x3c)?;
                        if arguments.argument_6 as u8 == 0 {
                            structure = if classification || world.actor_category_id(actor)? == 0x1f
                            {
                                Some(world.zone_actor(z)?)
                            } else {
                                world.structure_at_4f2220(position)?
                            };
                        }
                        same_last = Some(world.zone_actor(z)?) == last;
                        break;
                    }
                }
                if !world.advance_tile(&mut tiles)? {
                    break;
                }
            }
            if !same_last {
                let six = !strict && structure.is_some() && {
                    let flags = world.definition_word(definition, 0x1c)?;
                    flags & (1 << 29) != 0 && flags & (1 << 10) != 0
                };
                if six {
                    if !actor_data
                        || (world.definition_word(definition, 0x430)? == 0 && {
                            let flags = world.definition_word(definition, 0x174)?;
                            (flags & (1 << 13) == 0
                                || world.definition_word(definition, 0x444)? == 0)
                                && flags & (1 << 12) == 0
                                && world.definition_byte(definition, 0x1a8)? == 0
                        })
                    {
                        return Ok(6);
                    }
                } else if zone.is_some() && structure.is_none() {
                    let a = (!actor_data || world.definition_byte(definition, 0x290)? == 0)
                        && world.definition_word(definition, 0x1c)? & (1 << 11) == 0;
                    let b = actor_data && world.definition_word(definition, 0x430)? != 0;
                    let c = actor_data
                        && (world.definition_byte(definition, 0x1a8)? != 0
                            || world.definition_word(definition, 0x174)? & (1 << 12) != 0);
                    if !a && !b && !c {
                        return Ok(7);
                    }
                }
            }
        }
    }
    if world.definition_word(definition, 0x1c)? & (1 << 18) == 0
        && world.descriptor_byte(0x52)? == 0
        && (arguments.other.is_none()
            || world.actor_word(arguments.other.unwrap(), 8)? & (1 << 12) == 0)
    {
        context.definition = world.descriptor_definition()?;
        context.ignore_actor = arguments.source;
        context.custom_filter = arguments.argument_7;
        context.source_actor = arguments.other;
        if actor_data {
            context.ignore_definition = world.definition_reference(definition, 0x4e0)?;
        }
        context.required_actor_flags |= 0x400;
        if world.definition_word(definition, 0x1c)? & (1 << 10) == 0 {
            context.required_actor_flags |= 0x20000000;
        }
        if actor_data && world.definition_word(definition, 0x1c)? & (1 << 29) != 0 {
            context.hostility_context = world.descriptor_word(0x28)?;
        }
        context.mask |= if arguments.argument_5 as u8 != 0 {
            0x400
        } else if world.definition_word(context.definition, 0x1c)? & (1 << 21) != 0 {
            0x600
        } else {
            0x500
        };
        if arguments.argument_6 as u8 != 0 {
            context.ignore_all = true;
        }
        let verdict = world.collision_4f0a70(&mut context, definition, p)?;
        if verdict != 0 {
            return Ok(verdict);
        }
        let current = world.descriptor_definition()?;
        if world.definition_virtual(current, 0x38)? {
            let current = world.descriptor_definition()?;
            if last.is_none() {
                let mode = world.map_settings_mode(true)?.ok_or_else(|| {
                    crate::Error::World("4F3BD2 dereferences null global world settings".into())
                })?;
                if mode == 0
                    && world.definition_word(current, 0x444)? != 0
                    && world.definition_word(current, 0x430)? == 0
                    && (arguments.argument_6 as u8 != 0 || world.footprint_4f0b70()? != 4)
                {
                    return Ok(4);
                }
            }
        }
    }
    Ok(0)
}
