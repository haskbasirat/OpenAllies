//! `558720` iterator, `5580F0(x, y, radius)` begin and `558750` next: the
//! mode-1 (circle) actor scan used by `5901F0` (Disturbable doodads).
//!
//! `5580F0` builds a `552DD0` descriptor, stores the circle radius (`46D8E0`,
//! shape kind 1) and position (`552250`; no source definition, so no snap),
//! requires actor flags `0x4000` (descriptor +38) and calls `558040(mode 1)`.
//! `558280` then visits the circle's tile rectangle like mode 0 (the zero
//! descriptor mask makes the `54F370` tile test and the outside-world result
//! inert), and per tile-list Thing: intrusive mark (`4EFB20`), the full
//! required-flags test, `552340`, and the mode-1 test
//! `(Thing vf+0 soft radius + query radius)^2 >= dx^2 + dy^2` with `dx` stored
//! binary32 and every other term at the thread precision (unordered rejects).
//! `558750` (its +A4 list is null here) resolves the result GTID; a stale or
//! null result ends the scan, a false virtual +B4 continues it.
use crate::{
    Error, Result,
    collision::{self, Descriptor, Shape},
    geometry::{Placement, tile_rectangle},
    index::{ActorKey, NodeKey, SpatialIndex},
    index_world::CollisionView,
    query::{self, QueryGeometry, QueryPolicy},
};
use aa_numeric::Binary;
use std::cmp::Ordering;

/// Descriptor +38 written by `5580F0` (`[esp+44] = 0x4000`).
pub const REQUIRED_ACTOR_FLAGS: u32 = 0x4000;

/// Native stack iterator state; the index must stay at the same owner address
/// until `finish`. The pointer is compared only, never dereferenced.
#[must_use = "finish the native circle iterator on every exit"]
pub struct Cursor {
    owner: *const SpatialIndex,
    rect: crate::geometry::TileRect,
    cell: [i32; 2],
    node: Option<NodeKey>,
    head: Option<ActorKey>,
    finished: bool,
    descriptor: Descriptor,
    center: [u32; 2],
    radius: u32,
}
impl Cursor {
    /// `558720` + `5580F0`.
    pub fn begin(
        index: &mut SpatialIndex,
        geometry: &QueryGeometry,
        position: [u32; 2],
        radius: u32,
        source_facing: [u32; 2],
    ) -> Result<Self> {
        let placement = Placement {
            shape: Shape { kind: 1, radius_bits: radius, bounds: [0; 4] },
            facing: source_facing,
            position,
        };
        //558040 -> 553370: the outside-world byte only matters with a mask.
        let _boundary = query::outside(&placement, geometry.extent_bits, index.precision())?;
        let rect = tile_rectangle(index.grid(), &placement, index.precision())?;
        let cell = rect.min;
        let node = index.tile(cell)?.first_node;
        Ok(Self {
            owner: index as *const _,
            rect,
            cell,
            node,
            head: None,
            finished: false,
            descriptor: Descriptor::default(),
            center: position,
            radius,
        })
    }
    fn validate(&self, index: &SpatialIndex) -> Result<()> {
        if !std::ptr::eq(self.owner, index) {
            return Err(Error::World("native circle iterator index owner moved or differs".into()));
        }
        Ok(())
    }
    /// 558450..55848E: ST0 = vf+0 radius + query radius; dx stored binary32.
    fn within(&self, index: &SpatialIndex, actor: ActorKey) -> Result<bool> {
        let p = index.precision();
        let b = |bits: u32| Binary::from_f32_bits(bits).map_err(Error::from);
        let member = index.member(actor)?;
        let r = b(member.soft.radius_bits)?.add(&b(self.radius)?, p)?;
        let position = member.actor.position;
        let dx = b(position[0])?.sub(&b(self.center[0])?, p)?.store_f32()?;
        let dy = b(position[1])?.sub(&b(self.center[1])?, p)?;
        let dy2 = dy.mul(&dy, p)?;
        let dx2 = b(dx)?.mul(&b(dx)?, p)?;
        let distance = dy2.add(&dx2, p)?;
        let reach = r.mul(&r, p)?;
        //fcompp ST0=reach vs ST1=distance; C0 (less or unordered) rejects.
        Ok(reach.compare(&distance) != Ordering::Less)
    }
    /// `558750`: the next accepted Thing's GTID, or `None` at the end.
    pub fn next<P: QueryPolicy>(&mut self, index: &mut SpatialIndex, policy: &mut P) -> Result<Option<u32>> {
        self.validate(index)?;
        if self.finished {
            return Ok(None);
        }
        loop {
            while let Some(node) = self.node {
                let value = index.node(node)?.clone();
                self.node = value.next;
                let actor = value.actor;
                if query::mark(index, &mut self.head, actor)? {
                    continue;
                }
                if index.member(actor)?.actor.flags & REQUIRED_ACTOR_FLAGS != REQUIRED_ACTOR_FLAGS {
                    continue;
                }
                if collision::excluded(actor, &mut self.descriptor, &mut CollisionView { index, policy })
                    .map_err(|e| Error::World(e.to_string()))?
                {
                    continue;
                }
                if !self.within(index, actor)? {
                    continue;
                }
                //5587E5: a stale or null GTID returns NULL (the scan ends).
                let reference = index.member(actor)?.reference;
                if index.resolve_reference(reference) != Some(actor) {
                    return Ok(None);
                }
                if !index.member(actor)?.actor_category {
                    continue;
                }
                return Ok(Some(reference));
            }
            self.cell[0] += 1;
            if self.cell[0] > self.rect.max[0] {
                self.cell[0] = self.rect.min[0];
                self.cell[1] += 1;
            }
            if self.cell[1] > self.rect.max[1] {
                self.finish(index)?;
                return Ok(None);
            }
            self.node = index.tile(self.cell)?.first_node;
        }
    }
    /// Native virtual +8 end cleanup and `557F90`/`557FD0` idempotent cleanup.
    pub fn finish(&mut self, index: &mut SpatialIndex) -> Result<()> {
        self.validate(index)?;
        query::clear(index, &mut self.head)?;
        self.finished = true;
        Ok(())
    }
}

#[cfg(test)]
#[path = "circle_actors_tests.rs"]
mod tests;
