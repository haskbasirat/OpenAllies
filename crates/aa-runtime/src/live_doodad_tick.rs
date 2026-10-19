//! DynamicDoodad component deferred Updates on the live retained World.
//!
//! Slot 0 Disturbable (0x1C bytes, ctor 590160; Updatable at +C, vtable
//! A40DDC) deferred +18 = 5902C0(data):
//! 1. 5901F0: a 558720/5580F0 circle scan (aa_spatial::circle_actors) at the
//!    doodad's +20/+24 with DisturbableData+14 affect_radius; the first found
//!    Thing whose 4CEDE0 (moving) holds makes it disturbed; 557F90/557FD0
//!    clear the intrusive marks.
//! 2. disturbed: World+E0 >= +18 runs vf+C (590380 -> 42A590 on the
//!    DisturbableData+8 Action list at the doodad) and, in either case, +18 =
//!    now + min + u32(85D840(4F02B0 RNG)) * 2^-32 * (max - min);
//! 3. 50ACC0(DisturbableData+10 update_time, 0) on this Updatable (queue 3).
//! The disturbed branch (FX action list, synchronized RNG draw) is an
//! explicit error until those owners are bound. Slot 1 Breakable (vtable
//! A40D94, +18 = 58FA30) is likewise an explicit error.
use crate::{
    actor_world::{ActiveComponent, ActorStore, ObjectKey, RuntimeComponent, Source},
    catalog::Catalog,
    definitions::Bank,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot};
use aa_numeric::Precision;
use aa_spatial::{index::SpatialIndex, moving_state};
use aa_world::queues::{Delivery, Error as QueueError, Owner, Queues};

type Result<T> = std::result::Result<T, String>;
fn error(e: impl std::fmt::Display) -> String {
    format!("live Disturbable 5902C0: {e}")
}

pub(crate) struct Context<'a, 'source> {
    pub catalog: &'a Catalog<'source>,
    pub bank: &'a Bank,
    pub actors: &'a ActorStore,
    pub world: &'a mut crate::load_world::World,
    pub source_precision: Precision,
    pub precision: Precision,
}

/// DisturbableData +10/+14/+18/+1C (binary32 bits).
struct Data {
    update_time: u32,
    affect_radius: u32,
}
fn data(cx: &Context<'_, '_>, id: &str) -> Result<Data> {
    let definition = cx.catalog.actors.native_dynamic_doodad(id, cx.source_precision).map_err(error)?;
    let component = definition.components[0]
        .as_ref()
        .ok_or_else(|| error("DynamicDoodadData+174 Disturbable component absent"))?;
    let field = |name: &str| -> Result<u32> {
        component
            .fields
            .get(name)
            .ok_or_else(|| error(format!("DisturbableData {name} absent")))?
            .fixed_bits()
            .map_err(error)
    };
    Ok(Data { update_time: field("update_time")?, affect_radius: field("affect_radius")? })
}

/// 4CEDE0 over the SAME live owners (aa_spatial::moving_state). Identities are
/// current GTIDs; positions are the published spatial member positions.
struct Moving<'a> {
    actors: &'a ActorStore,
    index: &'a SpatialIndex,
}
fn moving_error(e: impl std::fmt::Display) -> moving_state::Error {
    moving_state::Error::World(e.to_string())
}
impl Moving<'_> {
    fn object(&self, id: u32) -> moving_state::Result<&crate::actor_world::RuntimeObject> {
        let key = self.actors.key(ActorId::from_raw(id)).ok_or_else(|| moving_error("4CEDE0 actor GTID is stale"))?;
        let object = self.actors.object(key).map_err(moving_error)?;
        if object.definition.native_class != 3 {
            return Err(moving_error("4CEDE0 requires ordinary class3 Actor dispatch"));
        }
        Ok(object)
    }
}
impl moving_state::World for Moving<'_> {
    fn actor(&self, id: u32) -> moving_state::Result<moving_state::Actor> {
        let object = self.object(id)?;
        let components = object.components().map_err(moving_error)?;
        let organization = components.present(Slot::Organization);
        let mover = components.present(Slot::Mover);
        let active = match (organization, object.active_component) {
            (true, _) => None,
            (false, ActiveComponent::UnresolvedNativeWord(_)) => {
                return Err(moving_error("4CEDE0 active component pointer is unresolved"));
            }
            (false, ActiveComponent::Slot(Slot::Mover)) => Some(id),
            _ => None,
        };
        let key = self.index.resolve_reference(id).ok_or_else(|| moving_error("4CEDE0 spatial member absent"))?;
        Ok(moving_state::Actor {
            component7c: organization.then_some(id),
            component6c: mover.then_some(id),
            active_component_e4: active,
            active_state70: components.present(Slot::Mind).then_some(id),
            position: self.index.member(key).map_err(moving_error)?.actor.position,
        })
    }
    fn component(&self, id: u32) -> moving_state::Result<moving_state::Component> {
        let owner = ActorId::from_raw(id);
        self.object(id)?;
        let Some(RuntimeComponent::Organization(value)) =
            self.actors.component(owner, Slot::Organization).map_err(moving_error)?
        else {
            return Err(moving_error("4CEDE0 Organization payload missing"));
        };
        if value.header.owner != owner {
            return Err(moving_error("4CEDE0 Organization owner mismatch"));
        }
        let selected = *value.words.get(&0x40).ok_or_else(|| moving_error("Organization+40 unwritten"))?;
        if selected != 0 {
            return Err(moving_error("4CEDE0 Organization+40 selected-member pointer is not bound"));
        }
        Ok(moving_state::Component {
            owner: id,
            linked_actor40: None,
            comparison_position: value.position_xy.map(|v| v.0),
        })
    }
    fn state_kind74(&self, id: u32) -> moving_state::Result<u32> {
        let owner = ActorId::from_raw(id);
        self.object(id)?;
        let Some(RuntimeComponent::Mind(value)) = self.actors.component(owner, Slot::Mind).map_err(moving_error)?
        else {
            return Err(moving_error("4CEDE0 Mind payload missing"));
        };
        let arena = &self.actors.minds;
        if arena.mind(value.mind_id).map_err(moving_error)?.actor != id {
            return Err(moving_error("4CEDE0 Mind owner mismatch"));
        }
        //4CEDE0 reads literal Mind+74.
        if let Some(state) = value.state_bindings.get(&0x68) {
            let state = arena.state(*state).map_err(moving_error)?;
            if state.parent != value.mind_id {
                return Err(moving_error("4CEDE0 embedded state parent mismatch"));
            }
            Ok(state.status)
        } else {
            value.words.get(&0x74).copied().ok_or_else(|| moving_error("Mind+74 field has no initialized binding"))
        }
    }
}

/// 5901F0 on the doodad at `position` with `radius`.
fn disturbed(cx: &mut Context<'_, '_>, position: [u32; 2], radius: u32) -> Result<bool> {
    let orientation = cx.catalog.current_renderer_orientation(cx.bank)?;
    let world = &mut *cx.world;
    let crate::load_world::SpatialOwner::Adopted(adopted) = &mut world.spatial else {
        return Err(error("actual adopted spatial owner absent"));
    };
    if adopted.error.is_some() || !adopted.pending.is_empty() || adopted.world.index().precision() != cx.precision {
        return Err(error("spatial adoption/live precision differs"));
    }
    let (_, geometry) = crate::load_object_init_world::map_geometry_index(
        &world.terrain,
        &world.header,
        adopted.world.index(),
        &world.zones,
        cx.precision,
    )
    .map_err(error)?;
    let (index, bindings) = adopted.world.split_index();
    let mut policy = crate::map_actor_creation::PlacementPolicy {
        catalog: cx.catalog,
        definitions: cx.bank,
        source_fields: &world.source_fields,
        bindings,
    };
    let mut cursor = aa_spatial::circle_actors::Cursor::begin(
        index,
        &geometry,
        position,
        radius,
        [orientation[0], orientation[1]],
    )
    .map_err(error)?;
    let result = (|| -> Result<bool> {
        while let Some(reference) = cursor.next(index, &mut policy).map_err(error)? {
            let view = Moving { actors: cx.actors, index: &*index };
            if moving_state::excluded(reference, &view).map_err(error)? {
                return Ok(true);
            }
        }
        Ok(false)
    })();
    let cleanup = cursor.finish(index).map_err(error);
    match (result, cleanup) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) | (Ok(_), Err(e)) => Err(e),
        (Err(e), Err(c)) => Err(format!("{e}; circle query cleanup failed: {c}")),
    }
}

/// One deferred +18 delivery to a DynamicDoodad component Updatable.
pub(crate) fn run(
    object: ObjectKey,
    slot: u8,
    event: Delivery,
    cx: &mut Context<'_, '_>,
    queues: &mut Queues,
) -> aa_world::queues::Result<()> {
    let fail = |e: String| QueueError::Callback(e);
    if slot != 0 {
        return Err(fail("live Breakable: required native 58FA30 deferred update is not bound".into()));
    }
    if event.elapsed_bits.is_some() || event.queue != 3 {
        return Err(fail(error("Disturbable Updatable is only reached by queue3 deferred +18")));
    }
    let entity = cx.actors.entity(object).map_err(|e| fail(error(e)))?;
    let present = match &entity.source {
        Source::Loaded(s) => s.doodad.as_ref().and_then(|d| d.components[0].as_ref()).is_some(),
        Source::NewObject { doodad, .. } => doodad.as_ref().and_then(|d| d.components[0].as_ref()).is_some(),
        _ => false,
    };
    if !present {
        return Err(fail(error("scheduled Disturbable component allocation absent")));
    }
    let runtime = cx.actors.object(object).map_err(|e| fail(error(e)))?;
    if runtime.definition.native_class != 9 {
        return Err(fail(error("Disturbable owner is not a DynamicDoodad")));
    }
    let id = runtime.definition.proof.id.clone();
    let data = data(cx, &id).map_err(fail)?;
    let spatial = cx.world.live_spatial().map_err(|e| fail(error(e)))?;
    let position = cx.actors.placement(object, spatial).map_err(|e| fail(error(e)))?.position_bits;
    if disturbed(cx, [position[0], position[1]], data.affect_radius).map_err(fail)? {
        return Err(fail(error(
            "required native 590380/42A590 disturb Action list and 4F02B0/85D840 recharge draw are not bound",
        )));
    }
    reschedule(queues, event.owner, data.update_time).map_err(|e| fail(error(e)))
}
/// 59035F: 50ACC0(DisturbableData+10, data 0) on queue 3.
fn reschedule(queues: &mut Queues, owner: Owner, delay: u32) -> aa_world::queues::Result<()> {
    queues.add_after(3, owner, 0, delay).map(|_| ())
}
