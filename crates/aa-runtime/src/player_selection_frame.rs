//!Native5F4450/5F4490/5F44C0/5F44F0 accepted selection leaves over the
//!actual ordering player's VWorldState and current moved Bank. Pointer hit
//!testing and UI request production remain separate; no inspection geometry
//!or direct actor/mind command is accepted here.
use crate::{
    actor_world::ActorStore,
    catalog::Catalog,
    definitions::Bank,
    live_world_frame::GameplayOrderLoans,
    load_world_globals::SessionPhase,
    player_order_world::{self as orders, DefinitionBankLoan},
    player_selection::View,
};
use aa_actors::ActorId;
use aa_orders::payload::{Field, ThingCheck};
#[path = "player_selection_orders.rs"]
mod core;
pub use core::Operation;
pub struct Decoded {
    operation: Operation,
    actor: Option<ActorId>,
    owner: crate::actor_world::ActorStoreWitness,
}
impl Decoded {
    ///5F4490/5F44C0 NULL paths do not call74B3B0 or touch the player view.
    pub fn needs_view(&self) -> bool {
        matches!(self.operation, Operation::Clear | Operation::Set) || self.actor.is_some()
    }
}
pub fn supports(descriptor: usize) -> bool {
    matches!(descriptor, 16 | 17 | 18 | 333)
}
///The native76A530 decoder reads the current low16 slot and+B0 before the
///Process body. The same existing source/storage binder validates that read;
///an actual missing slot yieldsNULL, never a missing source owner.
pub fn decode(
    catalog: &Catalog<'_>,
    actors: &ActorStore,
    bank: &Bank,
    descriptor: usize,
    fields: &[Field],
) -> Result<Decoded, String> {
    let operation = match descriptor {
        16 => Operation::Add,
        17 => Operation::Remove,
        18 => Operation::Clear,
        333 => Operation::Set,
        _ => return Err("accepted selection decree16/17/18/333 required".into()),
    };
    let actor = if operation == Operation::Clear {
        if !fields.is_empty() {
            return Err("5F44F0 exact empty decoded selection fields required".into());
        }
        None
    } else {
        let [
            Field::Thing {
                check: ThingCheck::B0,
                slot,
            },
        ] = fields
        else {
            return Err("76A530 exact decoded B0 selection slot required".into());
        };
        let Some(slot) =
            orders::resolve_thing_reference(catalog, actors, bank, ThingCheck::B0, *slot)
                .map_err(|e| e.to_string())?
        else {
            return Ok(Decoded {
                operation,
                actor: None,
                owner: actors.identity_witness(),
            });
        };
        Some(
            actors
                .identities()
                .iter()
                .find(|entry| entry.id.slot() == slot)
                .ok_or("76A530 resolved current generation disappeared")?
                .id,
        )
    };
    Ok(Decoded {
        operation,
        actor,
        owner: actors.identity_witness(),
    })
}
///Closed synchronous bridge: no outward registry callback can retire/reuse an
///ActorId between decode and execute. The actual source/owner is revalidated
///by each original selection kernel at its reached field access.
pub fn execute(
    catalog: &Catalog<'_>,
    parts: GameplayOrderLoans<'_>,
    decoded: Decoded,
    slot: &u32,
    view: &mut impl View,
    session: &dyn SessionPhase,
) -> Result<(), String> {
    if !decoded.needs_view() {
        return Ok(());
    }
    if !decoded.owner.is_for(parts.actors) {
        return Err("selection decoder belongs to another ActorStore allocation".into());
    }
    let bank = DefinitionBankLoan::new(parts.definitions);
    let (_, mut actors) = parts.actors.split_minds();
    fn apply<V: View>(
        catalog: &Catalog<'_>,
        bank: &DefinitionBankLoan<'_>,
        actors: &mut crate::actor_world::ActorAccess<'_>,
        actor: Option<ActorId>,
        slot: &u32,
        view: &mut V,
        session: &dyn SessionPhase,
        operation: Operation,
    ) -> Result<(), String> {
        struct Typed<'a, 'bank, 'store, 'source, V> {
            catalog: &'a Catalog<'source>,
            bank: &'a DefinitionBankLoan<'bank>,
            actors: &'a mut crate::actor_world::ActorAccess<'store>,
            actor: Option<ActorId>,
            slot: &'a u32,
            view: &'a mut V,
            session: &'a dyn SessionPhase,
        }
        impl<V: View> Typed<'_, '_, '_, '_, V> {
            fn current(&self) -> Result<(), String> {
                if *self.slot == 0 || *self.slot != self.view.player_id() {
                    Err("74B3B0 actual Admin284 player and VWorldState owner differ".into())
                } else {
                    Ok(())
                }
            }
        }
        impl<V: View> core::World for Typed<'_, '_, '_, '_, V> {
            type Actor = ActorId;
            fn target(&mut self) -> Result<Option<ActorId>, String> {
                Ok(self.actor)
            }
            fn clear(&mut self) -> Result<(), String> {
                self.current()?;
                orders::clear_selection_in_bank(self.catalog, self.bank, self.actors, self.view)
                    .map_err(|e| e.to_string())
            }
            fn add(&mut self, actor: ActorId) -> Result<(), String> {
                self.current()?;
                orders::select_in_bank(
                    self.catalog,
                    self.bank,
                    self.actors,
                    self.view,
                    self.session,
                    Some(actor),
                )
                .map(drop)
                .map_err(|e| e.to_string())
            }
            fn remove(&mut self, actor: ActorId) -> Result<(), String> {
                self.current()?;
                orders::deselect_in_bank(self.catalog, self.bank, self.actors, self.view, actor)
                    .map(drop)
                    .map_err(|e| e.to_string())
            }
        }
        core::run(
            &mut Typed {
                catalog,
                bank,
                actors,
                actor,
                slot,
                view,
                session,
            },
            operation,
        )
    }
    apply(
        catalog,
        &bank,
        &mut actors,
        decoded.actor,
        slot,
        view,
        session,
        decoded.operation,
    )
}
