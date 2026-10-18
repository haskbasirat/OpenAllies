//! Reached4D0A00/4293C0 on the same stored Actor and World presentation owner.
//! The common command constructor retains native getter order. Only a matched
//! compiled state graph reaches this adapter; mute returns before OS/clock reads.
use crate::{
    actor_world::{ActorAccess, ObjectKey},
    catalog::Catalog,
    definitions::Bank,
    fx_graph::{self, Command, CreationInputs, MatchedState, StateCreationInputs},
    fx_manager,
    spatial_world::SpatialWorld,
};
use aa_actors::ActorId;
type Result<T> = std::result::Result<T, String>;

pub(crate) fn publish(
    catalog: &Catalog<'_>,
    actors: &ActorAccess<'_>,
    bank: &Bank,
    spatial: &SpatialWorld,
    clock: &aa_simulation::NativeClock,
    actor: ActorId,
    matched: MatchedState,
    presentation: &mut fx_manager::State<Command>,
) -> Result<()> {
    matched.graph.require_in_bank(catalog, bank)?;
    let key = actors
        .key(actor)
        .ok_or("stored state FX current Actor generation absent")?;
    let object = actors.object(key).map_err(|e| e.to_string())?;
    if object.definition.native_class != 3
        || object.actor.is_none()
        || !matched
            .graph
            .owner()
            .eq_ignore_ascii_case(&object.definition.proof.id)
    {
        return Err("stored state FX graph/ordinary Actor source differs".into());
    }
    let command = fx_graph::construct_actor_state(
        matched,
        &mut Inputs {
            catalog,
            actors,
            spatial,
            clock,
            key,
            actor,
            presentation,
        },
    )?;
    if let Some(command) = command {
        presentation.submit(Some(command), 0, &mut Presentation)?;
    }
    Ok(())
}
struct Inputs<'a, 'store, 'source> {
    catalog: &'a Catalog<'source>,
    actors: &'a ActorAccess<'store>,
    spatial: &'a SpatialWorld,
    clock: &'a aa_simulation::NativeClock,
    key: ObjectKey,
    actor: ActorId,
    presentation: &'a fx_manager::State<Command>,
}
impl CreationInputs for Inputs<'_, '_, '_> {
    fn muted(&mut self) -> Result<bool> {
        Ok(self.presentation.muted())
    }
    fn creator_kingdom(&mut self) -> Result<Option<ActorId>> {
        //Ordinary Actor+A8=false/B4=true. Its virtual108 returns current E8;
        //428F40 then reads that actual Thing's full generation-bearing GTID.
        let raw = *self
            .actors
            .object(self.key)
            .map_err(|e| e.to_string())?
            .actor
            .as_ref()
            .ok_or("stored state FX Actor storage absent")?
            .words
            .get(&0xe8)
            .ok_or("stored state FX Actor+E8 is unwritten")?;
        if raw == 0 {
            return Ok(None);
        }
        let owner = ActorId::from_raw(raw);
        let key = self
            .actors
            .key(owner)
            .ok_or("stored state FX E8 owner generation absent")?;
        let actual = self
            .actors
            .id(key)
            .ok_or("stored state FX owner Thing GTID absent")?;
        if actual != owner {
            return Err("stored state FX owner Thing identity differs".into());
        }
        Ok(Some(actual))
    }
    fn current_position(&mut self) -> Result<[u32; 3]> {
        Ok(self
            .actors
            .placement(self.key, self.spatial)
            .map_err(|e| e.to_string())?
            .position_bits)
    }
    fn current_thing(&mut self) -> Result<Option<ActorId>> {
        if self.actors.key(self.actor) != Some(self.key) {
            return Err("stored state FX current Actor generation changed".into());
        }
        Ok(Some(self.actor))
    }
    fn current_os_thread_id(&mut self) -> Result<u32> {
        let mut execution = self
            .catalog
            .load_execution
            .try_borrow_mut()
            .map_err(|e| e.to_string())?;
        execution
            .as_deref_mut()
            .ok_or("stored state FX requires actual execution OS identity owner")?
            .current_os_thread_id()
    }
    fn application_os_thread_id(&mut self) -> Result<u32> {
        let execution = self
            .catalog
            .load_execution
            .try_borrow()
            .map_err(|e| e.to_string())?;
        Ok(execution
            .as_deref()
            .ok_or("stored state FX requires actual application OS identity owner")?
            .application_os_thread_id())
    }
    fn view_time(&mut self) -> Result<u32> {
        Err("stored state FX on application thread requires actual View+8C clock".into())
    }
    fn world_time(&mut self) -> Result<u32> {
        Ok(self.clock.begin.0)
    }
}
impl StateCreationInputs for Inputs<'_, '_, '_> {
    fn global_position(&mut self) -> Result<[u32; 3]> {
        Ok(crate::fx_static_position::unset_position())
    }
}
struct Presentation;
impl fx_manager::Effects<Command> for Presentation {
    fn deliver(&mut self, _: u64, _: Option<Command>) -> Result<()> {
        Err("stored state FX requires actual attached View43B870 delivery".into())
    }
    fn destroy(&mut self, _: Command) -> Result<()> {
        Err("stored state FX requires actual428350 destruction effects".into())
    }
}
