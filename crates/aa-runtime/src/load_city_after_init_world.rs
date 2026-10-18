//!563FE0 over the actual held City, Actor+148 zone8 and same World ZoneMap.
//!The shared native leaf reads radius/position only after CreateZone succeeds;
//!later required solver/presentation callbacks retain earlier zone allocation.
use crate::{catalog::Catalog,live_object_init::Record,load_detection_activation::{self,Effects,Identities}};
use aa_actors::{actor_creation::{Error,Result},components::ComponentSlot,component_virtuals::{CityAfterWorld,city_after_init}};
use aa_mapworld::restore::RestoredComponent;
use aa_numeric::Precision;
use aa_zones::tile::ZoneMap;
fn error(e:impl std::fmt::Display)->Error {Error::World(e.to_string())}
fn checked(catalog:&Catalog<'_>,record:&Record<'_>)->Result<Precision> {
    let pc=crate::load_actor_init_world::validate(catalog,record)?;
    if record.definition.component_mask.is_none_or(|v|v&(1<<ComponentSlot::City.index())==0) {
        return Err(Error::Unsupported("563FE0 actual source City factory absent"));
    }
    match record.shell.actor.as_ref().and_then(|v|v.components.get(ComponentSlot::City)) {
        Some(RestoredComponent::City(city)) if city.header.owner==record.owner=>Ok(pc),
        _=>Err(Error::Unsupported("563FE0 actual held City allocation/owner differs")),
    }
}
struct World<'a,'r,'source,E> {
    catalog:&'a Catalog<'source>,record:&'a mut Record<'r>,map:&'a mut ZoneMap,
    identities:&'a mut Identities,effects:&'a mut E,source_pc:Precision,pc:Precision,
}
impl<E:Effects> CityAfterWorld for World<'_,'_,'_,E> {
    fn create_zone8(&mut self)->Result<bool> {
        load_detection_activation::create_slot(self.record,self.identities,8).map_err(error)
    }
    fn radius_5b4(&mut self)->Result<u32> {
        //The same source-bound RuntimeDefinition owns the actual loader value.
        //A missing field is unbound; the AfterInit path invents no radius.
        let stored=self.record.definition.ground.city_zone_radius_bits
            .ok_or(Error::Unsupported("563FF9 actual ActorData5B4 storage absent"))?;
        let current=self.catalog.actors.native_city_zone_radius(&self.record.definition.proof.id,self.source_pc)
            .map_err(error)?.ok_or(Error::Unsupported("563FF9 source CityComponent absent"))?;
        if stored!=current {return Err(Error::Unsupported("563FF9 actual source5B4 field differs"));}
        Ok(stored)
    }
    fn position_xy(&mut self)->Result<[u32;2]> {
        Ok(self.record.shell.thing.position_bits[..2].try_into().unwrap())
    }
    fn move_zone8(&mut self,xy:[u32;2],radius:u32)->Result<()> {
        load_detection_activation::operate_slot(self.record,self.map,self.identities,8,Some((xy,radius)),self.pc,self.effects).map_err(error)
    }
}
pub fn after_init(catalog:&Catalog<'_>,record:&mut Record<'_>,map:&mut ZoneMap,identities:&mut Identities,pc:Precision,effects:&mut impl Effects)->Result<()> {
    let source_pc=checked(catalog,record)?;
    city_after_init(&mut World {catalog,record,map,identities,effects,source_pc,pc})
}

