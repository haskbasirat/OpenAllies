//! Native ability FX handoff4C8500 ->4294E0 and owned instance428570.
//! Renderer/effect-graph execution remains the required submit service.
use crate::{Error, Result, ability::Ability, performer::Context};
use aa_body::F32;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trigger {
    pub definition: u32,
    pub word4: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instance {
    pub trigger: Trigger,
    pub word8: u32,
    pub allegiance_id: u32,
    pub source_id: u32,
    pub source_position: [F32; 3],
    pub target_id: u32,
    pub target_position: [F32; 3],
    pub local_definition: Option<u32>,
    pub use_count: u32,
    pub started_at: F32,
    pub parameters: [F32; 3],
    pub offset: [F32; 3],
    pub kind: u32,
    pub flag58: bool,
    pub callback_word60: u32,
    pub next: Option<u32>,
}
pub trait World {
    /// Definition+70+8*event: original four authored start/use/end/hit references.
    fn trigger(&self, ability_definition: u32, event: u32) -> Result<Trigger>;
    fn suppressed(&self) -> Result<bool>;
    fn is_kingdom(&mut self, actor: u32) -> Result<bool>;
    fn is_actor(&mut self, actor: u32) -> Result<bool>;
    fn allegiance(&mut self, actor: u32) -> Result<Option<u32>>;
    fn actor_id(&self, actor: u32) -> Result<u32>;
    fn actor_position(&self, actor: u32) -> Result<[F32; 3]>;
    /// Native GetCurrentThreadId vs rendering thread identity. This must not be
    /// guessed from the simulation call's timestamp or replacement wall clock.
    fn current_is_render_thread(&mut self) -> Result<bool>;
    fn render_time(&self) -> Result<F32>;
    fn world_time(&self) -> Result<F32>;
    /// Original50C800(instance,0): consumes the owned instance. The actual manager
    /// handles queuing, immediate registration or suppression/disposal here.
    /// A diagnostic event list is not a production replacement for this service.
    fn submit(&mut self, instance: Instance) -> Result<()>;
}
/// Captures fallback vectors before virtual dispatch, then refreshes live actor
/// positions and generation IDs. Definitions/actor handles remain valid during
/// the call; allocation layout and reference-pointer addresses are not emulated.
pub fn emit(
    ability: &Ability,
    event: u32,
    context: &Context,
    world: &mut impl World,
) -> Result<()> {
    if event > 3 {
        return Err(Error::Invalid("unknown native ability FX event"));
    }
    let source_fallback = context.source_position;
    let target_fallback = context.target_position;
    let local_definition = ability.local_definition;
    let use_count = ability.use_count;
    let trigger = world.trigger(ability.definition, event)?;
    if trigger.definition == 0 || world.suppressed()? {
        return Ok(());
    }
    let allegiance_id = if let Some(source) = context.source {
        if world.is_kingdom(source)? {
            world.actor_id(source)?
        } else if world.is_actor(source)? {
            match world.allegiance(source)? {
                Some(owner) => world.actor_id(owner)?,
                None => 0,
            }
        } else {
            0
        }
    } else {
        0
    };
    let target_position = match context.target {
        Some(actor) => world.actor_position(actor)?,
        None => target_fallback,
    };
    let source_position = match context.source {
        Some(actor) => world.actor_position(actor)?,
        None => source_fallback,
    };
    let target_id = match context.target {
        Some(actor) => world.actor_id(actor)?,
        None => 0,
    };
    let source_id = match context.source {
        Some(actor) => world.actor_id(actor)?,
        None => 0,
    };
    let started_at = if world.current_is_render_thread()? {
        world.render_time()?
    } else {
        world.world_time()?
    };
    world.submit(Instance {
        trigger,
        word8: 0,
        allegiance_id,
        source_id,
        source_position,
        target_id,
        target_position,
        local_definition,
        use_count,
        started_at,
        parameters: [F32(0xbf800000); 3],
        offset: [F32::ZERO; 3],
        kind: 42,
        flag58: false,
        callback_word60: 0,
        next: None,
    })
}
