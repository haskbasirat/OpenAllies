//! Native 4D64F0 state coordinator over the actual saved actor shell.
//!
//! An unchanged request returns before native source/effect lookup. Construction
//! flag 31 suppresses enter/leave, but never exclusive action-mask rebuild.
//! Reached live effects remain explicit errors; earlier state stores survive.
use aa_actors::state;
use aa_actors::{ActorId, components::StateBits};
use aa_mapworld::restore::ActorStateContext;

struct StateContext<'a> {
    owner: ActorId,
    owner_state: &'a mut StateBits,
    owner_thing_flags: &'a mut u32,
}
struct Effects<'a> {
    context: StateContext<'a>,
}
impl Effects<'_> {
    fn commit(&mut self, fields: &state::Fields) {
        self.context.owner_state.low = fields.bits as u32;
        self.context.owner_state.high = (fields.bits >> 32) as u32;
        *self.context.owner_thing_flags = fields.thing_flags;
    }
    fn unbound(&mut self, fields: &state::Fields, address: &str) -> state::Result<()> {
        self.commit(fields);
        Err(state::Error::Callback(format!(
            "{address} saved Actor state effects for {:#010x} are unbound",
            self.context.owner.raw()
        )))
    }
}
impl state::Effects for Effects<'_> {
    fn leave(
        &mut self,
        fields: &mut state::Fields,
        _: &'static state::Descriptor,
    ) -> state::Result<()> {
        self.unbound(fields, "4D16B0")
    }
    fn enter(
        &mut self,
        fields: &mut state::Fields,
        _: &'static state::Descriptor,
        _: Option<&'static state::Descriptor>,
    ) -> state::Result<()> {
        self.unbound(fields, "4D4A90")
    }
    fn rebuild_abilities(&mut self, fields: &mut state::Fields) -> state::Result<()> {
        self.unbound(fields, "4D1350")
    }
}

/// Executes the proved native coordinator. This does not claim that authored
/// properties, Character views, FX, Economy or action publication are bound.
/// The source-free early-return branch needs none of those services.
pub fn set(context: ActorStateContext<'_>, index: u8, enabled: bool) -> Result<(), String> {
    set_fields(
        context.owner,
        &mut context.owner_actor.components.state,
        context.owner_thing_flags,
        index,
        enabled,
    )
}

/// The same native coordinator over disjoint actual fields while another
/// component is borrowed from the Actor slot table. No replacement ActorShell
/// or second persistent state owner is constructed.
pub fn set_fields(
    owner: ActorId,
    owner_state: &mut StateBits,
    owner_thing_flags: &mut u32,
    index: u8,
    enabled: bool,
) -> Result<(), String> {
    let state = *owner_state;
    let mut fields = state::Fields {
        bits: u64::from(state.low) | (u64::from(state.high) << 32),
        thing_flags: *owner_thing_flags,
    };
    let mut effects = Effects {
        context: StateContext {
            owner,
            owner_state,
            owner_thing_flags,
        },
    };
    let result = state::set(&mut fields, index, enabled, &mut effects);
    effects.commit(&fields);
    result.map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "saved_actor_state_tests.rs"]
mod tests;
