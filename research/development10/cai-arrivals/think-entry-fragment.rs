
///Actual state Think after scheduler dispatch has lent the SAME live owners.
///Only Regiment+34 has its concrete derived Think here; no absent state can
///become a successful tick and no state/runtime owner is reconstructed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn think_on_owners(
 catalog:&Catalog<'_>,actors:&mut ActorAccess<'_>,bank:&mut Bank,
 spatial:&mut SpatialWorld,variables:&mut crate::load_view_variables::State,
 presentation:&mut crate::fx_manager::State<crate::fx_graph::Command>,
 zones:&aa_zones::tile::ZoneMap,runtime:&mut mind_world::Runtime,
 state:StateId,arena:&mut Arena,queues:&mut aa_world::queues::Queues,
 owners:&mut crate::scheduler_owners::Owners,random:&mut aa_simulation::NativeRng,
 clock:&aa_simulation::NativeClock,inventory:&aa_mapworld::WorldInventory,pc:Precision,
)->aa_mind::Result<bool>{
 let binding=mind_world::state_binding(actors,arena,state)?;
 if binding.kind!=Kind::Regiment||binding.offset!=0x34{return required("actual Mind Think subtype has no concrete runtime leaf")}
 let mut bound=Bound{catalog,bank,spatial,variables,inventory,random,clock,presentation,zones,pc};
 let mut context=Context{actors,arena,runtime,queues,owners};
 regiment_sleep::think(&mut bound,&mut context,state)
}
