//! Concrete4F1E10 binding after StartMission camera initialization.
//! Source definitions are immutable projections; the world byte, actors,
//! statistics, snapshot lists and Kingdom order are the same moved owners.
use crate::{catalog::Catalog,restoration::Staged};
use aa_numeric::Precision;
type Result<T>=std::result::Result<T,String>;
pub(crate) fn run(staged:&mut Staged,catalog:&Catalog<'_>,pc:Precision)->Result<()> {
 catalog.require_source_owner()?;
 let source=staged.definitions.combat_source()?;source.completed_means()?;
 if source.source_identities()!=catalog.actors.sources.as_slice(){return Err("mission WorldStart moved source universe differs".into())}
 let source_pc=source.source_precision();
 let resources=crate::kingdom_economy::Definitions::bind(staged.resource_source.as_deref().ok_or("mission WorldStart actual Resource source owner absent")?,catalog.actors,source_pc).map_err(|e|e.to_string())?;
 let formulas=crate::world_start_world::bind_formulas(catalog.actors,resources.catalog(),source_pc).map_err(|e|e.to_string())?;
 let world=staged.load_world.as_mut().ok_or("mission WorldStart actual World absent")?;
 crate::world_start_world::World{actors:&mut staged.actors,economy:&mut staged.kingdom_economy,
  started:&mut world.tail.started_274,major_kingdoms:world.kingdoms.majors(),source:catalog.actors,
  resources:resources.catalog(),formulas:&formulas,source_precision:source_pc,precision:pc}
  .start().map_err(|e|e.to_string())
}
