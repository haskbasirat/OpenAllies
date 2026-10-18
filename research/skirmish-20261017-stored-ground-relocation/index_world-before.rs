//! Concrete native height and collision bindings for the owned live index.
//! Definition policy, organization/allegiance and component effects remain
//! required services; geometry and membership are never permissive callbacks.
use crate::{
    Actor, Error, HeightLayer, HeightMethod, Result, ShapeKind, ShapePlacement, TerrainGrid, World,
    collision::{self, Descriptor},
    height_layers::HeightLayers,
    index::{ActorKey, SpatialIndex, TransformCallbacks},
};
use aa_numeric::{Binary, Precision};

pub struct GroundSurface<'a> {
    pub terrain: &'a TerrainGrid,
    pub heights: HeightLayers<'a>,
}
impl GroundSurface<'_> {
    pub fn sample(&self, position: [u32; 2], flags: u32, precision: Precision) -> Result<Binary> {
        let mut actor = Actor {
            flags: 0,
            position: [position[0], position[1], 0],
            facing: [0, 0],
        };
        crate::query_actor_height(
            &mut actor,
            flags,
            self.terrain,
            precision,
            &mut HeightQuery {
                heights: &self.heights,
            },
        )
    }
}
struct HeightQuery<'a, 'b> {
    heights: &'a HeightLayers<'b>,
}
impl World for HeightQuery<'_, '_> {
    fn sample_height(
        &mut self,
        _: &mut Actor,
        layer: HeightLayer,
        method: HeightMethod,
        position: [u32; 2],
        precision: Precision,
    ) -> Result<Binary> {
        self.heights.sample(layer, method, position, precision)
    }
    fn shape(&mut self, _: &mut Actor, _: ShapeKind) -> Result<u32> {
        Err(Error::World(
            "height-only query cannot request shapes".into(),
        ))
    }
    fn update_tiles(
        &mut self,
        _: &mut Actor,
        _: ShapePlacement,
        _: ShapePlacement,
        _: ShapePlacement,
        _: ShapePlacement,
    ) -> Result<()> {
        Err(Error::World(
            "height-only query cannot update membership".into(),
        ))
    }
    fn position_changed(&mut self, _: &mut Actor) -> Result<()> {
        Err(Error::World(
            "height-only query cannot commit positions".into(),
        ))
    }
    fn world_phase(&self) -> u32 {
        unreachable!("height query has no presentation branch")
    }
    fn presentation_available(&self) -> bool {
        unreachable!("height query has no presentation branch")
    }
    fn presentation_changed(&mut self, _: &mut Actor) -> Result<()> {
        Err(Error::World("height-only query cannot present".into()))
    }
}

impl SpatialIndex {
    /// Original4EEB40 using the original terrain/height query and authoritative
    /// pose. Height is sampled at the old XY, exactly as in the original body.
    pub fn move_ground<C: TransformCallbacks>(
        &mut self,
        key: ActorKey,
        mut candidate: [u32; 2],
        surface: &GroundSurface<'_>,
        callbacks: &mut C,
    ) -> Result<()> {
        let terrain = surface.terrain;
        terrain.validate()?;
        if self.grid().dimensions != [terrain.width, terrain.height]
            || self.grid().coordinate_shift != terrain.coordinate_shift
            || self.grid().cell_size_bits != terrain.cell_size
        {
            return Err(Error::InvalidGrid);
        }
        let member = self.member(key)?;
        let flags = member.actor.flags;
        if flags & (1 << 21) == 0 && flags & (1 << 12) != 0 {
            return Err(Error::UninitializedNativeHeight);
        }
        let height = surface
            .sample(
                member.actor.position,
                if flags & (1 << 21) != 0 { 0x123 } else { 0x122 },
                self.precision(),
            )?
            .store_f32()?;
        if flags & (1 << 16) != 0 {
            // The ordinary shape getter returns its immutable definition. The
            // 46EE00 body only needs cell size and the candidate coordinates.
            crate::snap_half_cell(&mut candidate, terrain.cell_size, self.precision())?;
        }
        self.commit_position(key, [candidate[0], candidate[1], height], callbacks)
    }
}

/// Non-geometric native descriptor services. Callbacks may change descriptor or
/// live actor state through the index, but must not reclaim the currently
/// traversed actor/list node. A broken lifetime is an explicit stale-node error.
pub trait CollisionPolicy {
    fn definition_flags(&self, definition: u32) -> collision::Result<u32>;
    fn definition_flag_1a8(&self, definition: u32) -> collision::Result<bool>;
    fn custom_filter(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        descriptor: &mut Descriptor,
    ) -> collision::Result<bool>;
    fn source_excludes(
        &mut self,
        index: &mut SpatialIndex,
        source: ActorKey,
        actor: ActorKey,
        descriptor: &mut Descriptor,
    ) -> collision::Result<bool>;
    fn allegiance(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        descriptor: &mut Descriptor,
    ) -> collision::Result<Option<u32>>;
    fn hostility_dispatch(&self, context: u32) -> collision::Result<u32>;
    fn hostile(
        &mut self,
        index: &mut SpatialIndex,
        dispatch: u32,
        context: Option<u32>,
        other: Option<u32>,
        descriptor: &mut Descriptor,
    ) -> collision::Result<bool>;
    fn moving_exclusion(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        descriptor: &mut Descriptor,
    ) -> collision::Result<bool>;
    fn outer_actor(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        descriptor: &mut Descriptor,
    ) -> collision::Result<Option<u32>>;
}
pub struct CollisionView<'a, P> {
    pub index: &'a mut SpatialIndex,
    pub policy: &'a mut P,
}
fn error(e: Error) -> collision::Error {
    collision::Error::World(e.to_string())
}
impl<P: CollisionPolicy> CollisionView<'_, P> {
    pub fn blocked(
        &mut self,
        cell: [i32; 2],
        descriptor: &mut Descriptor,
    ) -> collision::Result<bool> {
        let grid = self.index.grid().clone();
        if (0..2).any(|a| cell[a] < 0 || cell[a] >= grid.dimensions[a]) {
            return Err(collision::Error::Grid);
        }
        let id = (cell[1] * grid.dimensions[0] + cell[0]) as u32;
        collision::blocked(id, &grid, descriptor, self.index.precision(), self)
    }
}
impl<P: CollisionPolicy> collision::World for CollisionView<'_, P> {
    fn tile(&self, id: u32) -> collision::Result<collision::Tile> {
        let width = self.index.grid().dimensions[0] as u32;
        let coordinates = [(id % width) as i32, (id / width) as i32];
        let t = self.index.tile(coordinates).map_err(error)?;
        Ok(collision::Tile {
            coordinates,
            flags20: t.flags20,
            actor_reference: t.primary_reference,
            first_node: t.first_node,
        })
    }
    fn node(&self, id: u32) -> collision::Result<collision::Node> {
        let n = self.index.node(id).map_err(error)?;
        Ok(collision::Node {
            actor: n.actor,
            next: n.next,
        })
    }
    fn actor(&self, id: u32) -> collision::Result<collision::Actor> {
        Ok(self.index.member(id).map_err(error)?.actor.clone())
    }
    fn definition_flags(&self, id: u32) -> collision::Result<u32> {
        self.policy.definition_flags(id)
    }
    fn definition_flag_1a8(&self, id: u32) -> collision::Result<bool> {
        self.policy.definition_flag_1a8(id)
    }
    fn blocking_radius(
        &mut self,
        actor: u32,
        _: Option<&mut Descriptor>,
    ) -> collision::Result<Binary> {
        Ok(Binary::from_f32_bits(
            self.index
                .member(actor)
                .map_err(error)?
                .blocking
                .radius_bits,
        )?)
    }
    fn custom_filter(&mut self, actor: u32, d: &mut Descriptor) -> collision::Result<bool> {
        self.policy.custom_filter(self.index, actor, d)
    }
    fn source_excludes(
        &mut self,
        source: u32,
        actor: u32,
        d: &mut Descriptor,
    ) -> collision::Result<bool> {
        self.policy.source_excludes(self.index, source, actor, d)
    }
    fn category_b4(&mut self, actor: u32, _: &mut Descriptor) -> collision::Result<bool> {
        Ok(self.index.member(actor).map_err(error)?.actor_category)
    }
    fn allegiance(&mut self, actor: u32, d: &mut Descriptor) -> collision::Result<Option<u32>> {
        self.policy.allegiance(self.index, actor, d)
    }
    fn hostility_dispatch(&self, context: u32) -> collision::Result<u32> {
        self.policy.hostility_dispatch(context)
    }
    fn hostile(
        &mut self,
        dispatch: u32,
        context: Option<u32>,
        other: Option<u32>,
        d: &mut Descriptor,
    ) -> collision::Result<bool> {
        self.policy.hostile(self.index, dispatch, context, other, d)
    }
    fn moving_exclusion(&mut self, actor: u32, d: &mut Descriptor) -> collision::Result<bool> {
        self.policy.moving_exclusion(self.index, actor, d)
    }
    fn resolve_reference(&self, reference: u32) -> collision::Result<Option<u32>> {
        Ok(self.index.resolve_reference(reference))
    }
    fn outer_actor(&mut self, actor: u32, d: &mut Descriptor) -> collision::Result<Option<u32>> {
        self.policy.outer_actor(self.index, actor, d)
    }
    fn blocking_shape(
        &mut self,
        actor: u32,
        _: Option<&mut Descriptor>,
    ) -> collision::Result<collision::Shape> {
        Ok(self.index.member(actor).map_err(error)?.blocking.clone())
    }
}
