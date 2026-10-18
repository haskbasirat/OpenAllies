//! Original GWorld4F1660 required-space validation child of4F3640.
//! Reads stay at their native call/field boundary. Geometry, source definitions,
//! generation lookup, tile metadata and live node ownership are required.
use crate::{Result, collision::Shape, geometry::Placement};
use aa_numeric::Precision;

#[derive(Clone, Copy, Debug)]
pub struct Arguments<D, A> {
    pub position: [u32; 2],
    pub definition: Option<D>,
    pub owner: u32,
    pub other: Option<A>,
    pub argument_7: u32,
    pub argument_8: u32,
}

/// Native read/call services. No absent producer becomes a clear result.
/// Definition identities are source-bank handles; Actor/Node identities retain
/// actual ownership. Errors preserve preceding callbacks; no rollback occurs.
pub trait World {
    type Definition: Copy + Eq;
    type Actor: Copy + Eq;
    type Node: Copy;
    type Zone: Copy;
    type Tiles;
    fn manager_radius_74(&mut self) -> Result<u32>;
    fn outside_553370(&mut self, placement: &Placement, p: Precision) -> Result<bool>;
    /// Definition+38 tests full EAX; actor virtuals below test AL.
    fn definition_is_actor(&mut self, definition: Self::Definition) -> Result<bool>;
    fn definition_word(&mut self, definition: Self::Definition, offset: u32) -> Result<u32>;
    fn definition_byte(&mut self, definition: Self::Definition, offset: u32) -> Result<u8>;
    /// Settlement component+4 (definition+464) or required definition+4E0.
    fn definition_reference(
        &mut self,
        definition: Self::Definition,
        offset: u32,
    ) -> Result<Option<Self::Definition>>;
    fn actor_virtual(&mut self, actor: Self::Actor, slot: u32) -> Result<bool>;
    fn actor_link(&mut self, actor: Self::Actor, slot: u32) -> Result<Option<Self::Actor>>;
    fn actor_reference_50(&mut self, actor: Self::Actor) -> Result<u32>;
    fn resolve_reference(&mut self, reference: u32) -> Result<Option<Self::Actor>>;
    fn actor_definition(&mut self, actor: Self::Actor) -> Result<Self::Definition>;
    fn actor_flags(&mut self, actor: Self::Actor) -> Result<u32>;
    fn begin_tiles(&mut self, placement: &Placement, p: Precision) -> Result<Self::Tiles>;
    fn tiles_valid(&mut self, tiles: &Self::Tiles) -> Result<bool>;
    fn advance_tile(&mut self, tiles: &mut Self::Tiles) -> Result<()>;
    fn game_mode_e4(&mut self) -> Result<u32>;
    fn current_thread_is_simulation(&mut self) -> Result<bool>;
    fn tile_zone_54ef80(
        &mut self,
        tiles: &Self::Tiles,
        exclude_actor: Option<Self::Actor>,
        exclude_definition: Option<Self::Definition>,
    ) -> Result<Option<Self::Zone>>;
    fn zone_actor(&mut self, zone: Self::Zone) -> Result<Self::Actor>;
    fn zone_overlap_4f0ae0(
        &mut self,
        actor: Self::Actor,
        placement: &Placement,
        p: Precision,
    ) -> Result<bool>;
    /// TerrainData+4C through tile[0] VariationData+1C, never tile flags20.
    fn tile_terrain_flags_4c(&mut self, tiles: &Self::Tiles) -> Result<u8>;
    fn first_node(&mut self, tiles: &Self::Tiles) -> Result<Option<Self::Node>>;
    fn node_actor(&mut self, node: Self::Node) -> Result<Self::Actor>;
    fn next_node(&mut self, node: Self::Node) -> Result<Option<Self::Node>>;
    /// Actual4CF210 placed blocking shape, followed by46EF30.
    fn actor_overlap(
        &mut self,
        actor: Self::Actor,
        placement: &Placement,
        p: Precision,
    ) -> Result<bool>;
    fn actor_mobile_4d1120(&mut self, actor: Self::Actor) -> Result<bool>;
    fn actor_enemy_140(&mut self, actor: Self::Actor, owner: u32) -> Result<bool>;
    fn global_mode_b28158(&mut self) -> Result<u32>;
}

///4F1660:0 clear,1 outside,2 discovery-zone overlap,3 nonland terrain,
///4 overlapping solid/mobile blocker,5 overlapping hostile bit29 blocker.
/// Low bytes of arguments7/8 are used. Native allocator/invalid-pointer paths
/// and geometric nonfinite domains remain errors in their concrete owners.
pub fn check<W: World>(
    arguments: Arguments<W::Definition, W::Actor>,
    p: Precision,
    world: &mut W,
) -> Result<u32> {
    let radius = world.manager_radius_74()?;
    let placement = Placement {
        shape: Shape {
            kind: 1,
            radius_bits: radius,
            bounds: [0; 4],
        },
        facing: [0, 0xbf800000],
        position: arguments.position,
    };
    if world.outside_553370(&placement, p)? {
        return Ok(1);
    }
    let mut excluded_definition = None;
    if let Some(mut definition) = arguments.definition
        && world.definition_is_actor(definition)?
    {
        if world.definition_word(definition, 0x174)? & (1 << 14) != 0 {
            definition = world
                .definition_reference(definition, 0x464)?
                .ok_or_else(|| crate::Error::World("4F1660 SettlementData+4 is null".into()))?;
        }
        excluded_definition = world.definition_reference(definition, 0x4e0)?;
    }
    let mut object = None;
    let mut excluded_actor = arguments.other;
    if let Some(other) = arguments.other {
        if world.actor_virtual(other, 0xb0)? {
            object = Some(other);
        }
        if world.actor_virtual(other, 0xb4)?
            && let Some(parent) = world.actor_link(other, 0x15c)?
        {
            excluded_actor = Some(parent);
        }
    }
    let cached_reference = if let Some(object) = object {
        let reference = world.actor_reference_50(object)?;
        world.resolve_reference(reference)?
    } else {
        None
    };
    let mut tiles = world.begin_tiles(&placement, p)?;
    while world.tiles_valid(&tiles)? {
        let has_reference = if let Some(object) = object {
            let reference = world.actor_reference_50(object)?;
            world.resolve_reference(reference)?.is_some()
        } else {
            false
        };
        if !has_reference {
            let mut excluded = excluded_definition;
            if world.game_mode_e4()? == 3 && world.current_thread_is_simulation()? {
                excluded = None;
            }
            if let Some(zone) = world.tile_zone_54ef80(&tiles, excluded_actor, excluded)? {
                let actor = world.zone_actor(zone)?;
                if world.zone_overlap_4f0ae0(actor, &placement, p)? {
                    return Ok(2);
                }
            }
        }
        if arguments.argument_7 as u8 == 0 && world.tile_terrain_flags_4c(&tiles)? & 1 == 0 {
            return Ok(3);
        }
        let mut node = world.first_node(&tiles)?;
        while let Some(current) = node {
            let actor = world.node_actor(current)?;
            // B4 is dispatched before any pointer/definition exclusion.
            let actor_view = world.actor_virtual(actor, 0xb4)?;
            let excluded = Some(actor) == arguments.other
                || Some(world.actor_definition(actor)?) == excluded_definition
                || Some(actor) == cached_reference;
            if !excluded && world.actor_flags(actor)? & (1 << 10) != 0 {
                let mut allowed = true;
                if actor_view {
                    let definition = world.actor_definition(actor)?;
                    allowed = world.definition_byte(definition, 0x1a8)? == 0;
                }
                if allowed && world.actor_overlap(actor, &placement, p)? {
                    let same_parent = if arguments.other.is_some() && actor_view {
                        world.actor_link(actor, 0x100)? == arguments.other
                    } else {
                        false
                    };
                    if !same_parent {
                        // Bit11 is captured before4D1120, bit29 after it and
                        // before140. These calls can change live flags/fields.
                        let solid = world.actor_flags(actor)? & (1 << 11) != 0;
                        let mobile = actor_view && world.actor_mobile_4d1120(actor)?;
                        let has_required = if actor_view {
                            let definition = world.actor_definition(actor)?;
                            world.definition_word(definition, 0x430)? != 0
                        } else {
                            false
                        };
                        if !has_required && (mobile || solid) {
                            return Ok(4);
                        }
                        let bit29 = world.actor_flags(actor)? & (1 << 29) != 0;
                        let enemy = if actor_view {
                            world.actor_enemy_140(actor, arguments.owner)?
                        } else {
                            world.global_mode_b28158()? == 3
                        };
                        if arguments.argument_8 as u8 == 0 && bit29 && enemy {
                            return Ok(5);
                        }
                    }
                }
            }
            node = world.next_node(current)?;
        }
        world.advance_tile(&mut tiles)?;
    }
    Ok(0)
}
