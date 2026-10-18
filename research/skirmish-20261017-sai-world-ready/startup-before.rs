//! Native SAI engine startup7A0D40. Required live services have no defaults.
//! A true native ready byte after a failed operation is not successful startup.
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
#[error("required SAI startup service failed: {0}")]
pub struct Error(pub String);
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Object(pub u64);
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub ready: bool,
    pub world_revision: u32,
    pub word6c: u32,
    pub word70: u32,
}
///Implementations own actual live arrays/registries and honor their native
///identity/ownership contracts. Queries expose fields, never guessed defaults.
///Callbacks may mutate those fields; subsequent getters reread at native sites.
pub trait World {
    fn clear_list78(&mut self, state: &mut State) -> Result<()>;
    fn clear_list80(&mut self, state: &mut State) -> Result<()>;
    fn global_revision(&self) -> u32;
    ///7ABC30 including adjacency, areas, bottlenecks and empty route cache.
    fn prepare_topology(&mut self, state: &mut State) -> Result<()>;
    fn kingdom_count(&self) -> usize;
    fn kingdom_at(&self, index: usize) -> Option<Object>;
    ///79E170 initializes actual per-region Kingdom state and existing AI slots.
    fn register_kingdom(&mut self, kingdom: Object, state: &mut State) -> Result<()>;
    fn ai_slot_count(&self) -> usize;
    fn ai_slot_at(&self, index: usize) -> Option<Object>;
    fn start_ai(&mut self, slot: Object, state: &mut State) -> Result<()>;
    fn registry_count(&self) -> i32;
    ///4EFBE0: first occupied slot strictly after low16(cursor), starting1 for0.
    fn next_registered_after(&self, cursor: u32) -> Result<Option<Object>>;
    fn actor_id(&self, actor: Object) -> u32;
    fn actor_b4(&mut self, actor: Object, state: &mut State) -> Result<bool>;
    fn actor_flags8(&self, actor: Object) -> u32;
    fn register_special_actor(&mut self, actor: Object, state: &mut State) -> Result<()>;
    fn actor_b0(&mut self, actor: Object, state: &mut State) -> Result<bool>;
    ///4EA430 returns the actual actor/Kingdom seen-record identity.
    fn seen_record(
        &mut self,
        actor: Object,
        kingdom: Object,
        state: &mut State,
    ) -> Result<Option<Object>>;
    fn seen_context(&self, record: Object) -> Object;
    ///Seen context virtual+38, tested for nonzero by native code.
    fn context_eligible(&mut self, context: Object, state: &mut State) -> Result<bool>;
    ///Original context+548 and+534 after its virtual callback.
    fn context_words(&self, context: Object) -> [u32; 2];
    fn seen_position(&self, record: Object) -> [u32; 2];
    fn region_at(&mut self, point: [u32; 2], state: &mut State) -> Result<Option<u32>>;
    fn region_kingdom(
        &mut self,
        region: u32,
        kingdom: Object,
        state: &mut State,
    ) -> Result<Option<Object>>;
    fn register_seen(
        &mut self,
        region_kingdom: Object,
        record: Object,
        state: &mut State,
    ) -> Result<()>;
}
fn first_kingdom<W: World>(world: &W, mut index: usize) -> usize {
    while index < world.kingdom_count() && world.kingdom_at(index).is_none() {
        index += 1;
    }
    index
}
///Complete outer non-diagnostic7A0D40 order. Full startup succeeds only if all
///required services return success. Prior effects/ready writes are not rolled
///back; do not retry a failed operation merely because ready is now true.
pub fn initialize<W: World>(state: &mut State, world: &mut W) -> Result<()> {
    if state.ready {
        return Ok(());
    }
    world.clear_list78(state)?;
    world.clear_list80(state)?;
    state.world_revision = world.global_revision();
    world.prepare_topology(state)?;
    let mut index = first_kingdom(world, 0);
    while index < world.kingdom_count() {
        let kingdom = world
            .kingdom_at(index)
            .ok_or_else(|| Error("Kingdom iterator invalidated".into()))?;
        world.register_kingdom(kingdom, state)?;
        index = first_kingdom(world, index + 1);
    }
    let mut index = 0;
    while index < world.ai_slot_count() {
        if let Some(slot) = world.ai_slot_at(index) {
            world.start_ai(slot, state)?;
        }
        index += 1;
    }
    state.ready = true;
    let mut cursor = 0;
    while world.registry_count() > 0 {
        let Some(actor) = world.next_registered_after(cursor)? else {
            break;
        };
        cursor = world.actor_id(actor);
        if world.actor_b4(actor, state)? && world.actor_flags8(actor) & 1 == 0 {
            world.register_special_actor(actor, state)?;
        }
        if world.actor_b0(actor, state)? {
            let mut index = first_kingdom(world, 0);
            while index < world.kingdom_count() {
                let kingdom = world
                    .kingdom_at(index)
                    .ok_or_else(|| Error("Kingdom iterator invalidated".into()))?;
                if let Some(record) = world.seen_record(actor, kingdom, state)?
                    && state.ready
                {
                    let context = world.seen_context(record);
                    if world.context_eligible(context, state)? {
                        let words = world.context_words(context);
                        if (words[0] != 0 || words[1] != 0)
                            && let Some(region) =
                                world.region_at(world.seen_position(record), state)?
                            && let Some(binding) = world.region_kingdom(region, kingdom, state)?
                        {
                            world.register_seen(binding, record, state)?;
                        }
                    }
                }
                index = first_kingdom(world, index + 1);
            }
        }
    }
    state.word6c = u32::MAX;
    state.word70 = u32::MAX;
    Ok(())
}
