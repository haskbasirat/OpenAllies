//! Native4D7332 ->4D1350 over the actual held Actor. This uses the full
//! saved contributor/publication binding, including Structure589A60 before
//! Construction565140; no construction-only stand-in or fake Organization.
use crate::{catalog::Catalog, live_object_init::Record};
use aa_actors::actor_creation::{Error, Result};
use aa_mapworld::WorldInventory;
use aa_numeric::Precision;
#[cfg(test)]
#[path = "load_actor_action_refresh_tests.rs"]
mod tests;

pub fn refresh(
    catalog: &Catalog<'_>,
    inventory: &WorldInventory,
    record: &mut Record<'_>,
    live_pc: Precision,
) -> Result<()> {
    #[cfg(test)]
    let before = if CAPTURE_ENABLED.get() && record.owner.raw() == 2696 {
        Some(snapshot(catalog, record, live_pc))
    } else {
        None
    };
    #[cfg(test)]
    let allocations = before.as_ref().map(|_| allocation_addresses(record));
    let result = crate::saved_actor_actions::rebuild(catalog, inventory, record, live_pc)
        .map_err(Error::World);
    #[cfg(test)]
    if let Some(before) = before {
        let value = match before {
            Ok(before) => {
                serde_json::json!({"before":before,"after":snapshot(catalog,record,live_pc).unwrap(),"same_allocations":allocations.unwrap()==allocation_addresses(record),"result":result.as_ref().map(|_|()).map_err(|e|e.to_string())})
            }
            Err(e) => serde_json::json!({"capture_error":e.to_string()}),
        };
        CAPTURE.with(|c| c.borrow_mut().push(value));
    }
    result
}

#[cfg(test)]
fn allocation_addresses(record: &Record<'_>) -> Vec<usize> {
    use aa_actors::components::ComponentSlot;
    use aa_mapworld::restore::RestoredComponent;
    let actor = record.shell.actor.as_ref().unwrap();
    let mut p = vec![
        std::ptr::from_ref(&record.shell.thing.flags) as usize,
        std::ptr::from_ref(record.shell.object.as_ref().unwrap()) as usize,
        actor.zones_148.zones().slots.as_ptr() as usize,
    ];
    for slot in [
        ComponentSlot::Structure,
        ComponentSlot::Construction,
        ComponentSlot::Properties,
    ] {
        p.push(match actor.components.get(slot) {
            Some(RestoredComponent::Structure(s)) => std::ptr::from_ref(s.as_ref()) as usize,
            Some(RestoredComponent::Construction(s)) => std::ptr::from_ref(s.as_ref()) as usize,
            Some(RestoredComponent::Properties(s)) => std::ptr::from_ref(s.as_ref()) as usize,
            _ => 0,
        });
    }
    p
}

#[cfg(test)]
thread_local! {
    static CAPTURE_ENABLED:std::cell::Cell<bool>=const {std::cell::Cell::new(false)};
    static CAPTURE:std::cell::RefCell<Vec<serde_json::Value>>=const {std::cell::RefCell::new(Vec::new())};
}
#[cfg(test)]
fn snapshot(
    catalog: &Catalog<'_>,
    record: &Record<'_>,
    live_pc: Precision,
) -> Result<serde_json::Value> {
    use aa_actors::components::ComponentSlot;
    use aa_mapworld::restore::RestoredComponent;
    let source_pc = crate::load_actor_init_world::validate(catalog, record)?;
    let actor = record.shell.actor.as_ref().unwrap();
    let object = record.shell.object.as_ref().unwrap();
    let mut structure = serde_json::Value::Null;
    if let Some(RestoredComponent::Structure(s)) = actor.components.get(ComponentSlot::Structure) {
        structure = serde_json::json!({"owner":s.header.owner.raw(),"parent":s.object_handle_10,"queue_count":s.queue_14.len()});
    }
    let properties = match actor.components.get(ComponentSlot::Properties) {
        Some(RestoredComponent::Properties(p)) => Some(p.modifier.flags),
        _ => None,
    };
    let construction = catalog
        .actors
        .native_construction_flags(&record.definition.proof.id, source_pc)
        .map_err(|e| Error::World(e.to_string()))?;
    let allow_sell = catalog
        .settings
        .as_ref()
        .unwrap()
        .allow_sell_under_siege()
        .map_err(|e| Error::World(e.to_string()))?
        .0;
    Ok(
        serde_json::json!({"owner":record.owner.raw(),"definition":record.definition.proof,"live_precision":live_pc,"source_precision":source_pc,
        "flags":record.shell.thing.flags,"state":u64::from(actor.components.state.low)|(u64::from(actor.components.state.high)<<32),
        "actions":u64::from(actor.words[&0xf0])|(u64::from(actor.words[&0xf4])<<32),"owning_kingdom":actor.words[&0xe8],"thing_word_c":record.shell.thing.word_c,"object4c":object.byte_4c,
        "slots":aa_actors::components::ALL_SLOTS.iter().filter(|s|actor.components.present(**s)).map(|s|s.index()).collect::<Vec<_>>(),
        "structure":structure,"properties_flags":properties,"construction":construction,"allow_sell_under_siege":allow_sell}),
    )
}
