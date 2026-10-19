//!Live native reference resolution between Admin callbacks. No player/actor
//!roster or definition Bank is copied. Unsupported domains are errors rather
//!than invented NULLs; genuine absent slots/players retain native semantics.
use super::*;
use aa_orders::{Error as OrderError,Result as OrderResult,payload::{Resolver,SchemaContext,ThingCheck,NameKind,Registry}};
use aa_runtime::{catalog::Catalog,live_sim::Sim,live_world_frame::Frame};
const COMMANDS:Registry=Registry(0xb3aa58);
#[path="local_object_registry.rs"]
mod object_registry;
fn required<T>(s:&'static str)->OrderResult<T>{Err(OrderError::SchemaContext(s))}
fn live_player(players:&RefCell<PlayersBorrow<'_>>,id:u32)->OrderResult<bool>{
    let players=players.try_borrow().map_err(|e|OrderError::SchemaSource(format!("actual reference Session loan: {e}")))?;
    Ok(players.find_order_player(id,false).is_some())
}
pub(crate) struct Current<'a,'battle,'owner,'source>{
    catalog:&'a Catalog<'source>,frame:&'a Frame<'battle,'static>,
    sim:&'a RefCell<&'owner mut Sim>,players:&'a RefCell<PlayersBorrow<'owner>>,
}
impl<'a,'battle,'owner,'source> Current<'a,'battle,'owner,'source>{
    pub(crate) fn new(catalog:&'a Catalog<'source>,frame:&'a Frame<'battle,'static>,sim:&'a RefCell<&'owner mut Sim>,players:&'a RefCell<PlayersBorrow<'owner>>)->Self{Self{catalog,frame,sim,players}}
    fn with<R>(&self,call:impl FnOnce(&aa_runtime::actor_world::ActorStore,&aa_runtime::definitions::Bank)->std::result::Result<R,String>)->OrderResult<R>{
        let sim=self.sim.try_borrow().map_err(|e|OrderError::SchemaSource(format!("actual reference Sim loan: {e}")))?;
        self.frame.with_order_data(|actors,bank,revision|{
            sim.require_created_world(revision)?;self.catalog.require_source_owner()?;
            let source=bank.combat_source()?;source.completed_means()?;
            if source.source_identities()!=self.catalog.actors.sources.as_slice(){return Err("actual reference Bank/source universe differs".into())}
            call(actors,bank)
        }).map_err(OrderError::SchemaSource)
    }
    fn command_count(&self)->OrderResult<usize>{
        self.with(|_,_|{
            let registry=self.catalog.actors.native_data_registry("ActorCommand").ok_or("actual ActorCommand source manager absent")?;
            if registry.mode!=Some(2){return Err("actual ActorCommand manager mode differs from47F7C0".into())}
            //The code constructors register all39 before UI source loading.
            //Do not equate authored declaration count/order with that registry;
            //unknown authored extensions remain outside this proved domain.
            for entry in &registry.entries{
                aa_rules::native::actor_command::predefined_actor_command(&entry.ids).map_err(|e|e.to_string())?;
            }
            Ok(aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS.len())
        })
    }
}
impl Resolver for Current<'_,'_,'_,'_>{
    fn thing(&self,_:ThingCheck,_:u16)->Option<u16>{unreachable!("live normalization must use fallible reference readers")}
    fn player(&self,_:u32)->bool{unreachable!("live normalization must use fallible reference readers")}
    fn definition(&self,_:Registry,_:u32)->bool{unreachable!("live normalization must use fallible reference readers")}
    fn named(&self,_:NameKind,_:&[u16])->Option<Vec<u16>>{unreachable!("live normalization must use fallible reference readers")}
    fn try_thing(&self,check:ThingCheck,slot:u16)->OrderResult<Option<u16>>{
        self.with(|actors,bank|aa_runtime::player_order_world::resolve_thing_reference(self.catalog,actors,bank,check,slot).map_err(|e|e.to_string()))
    }
    fn try_player(&self,id:u32)->OrderResult<bool>{
        live_player(self.players,id)
    }
    fn try_definition(&self,registry:Registry,index:u32)->OrderResult<bool>{
        if registry==object_registry::OBJECTS{return self.object_definition_76a960(index)}
        if registry!=COMMANDS{return required("live reference needs actual reached definition registry")}
        Ok((index as usize)<self.command_count()?)
    }
    fn try_named(&self,_:NameKind,_:&[u16])->OrderResult<Option<Vec<u16>>>{required("live reference needs actual reached named resource owner")}
}
impl SchemaContext for Current<'_,'_,'_,'_>{
    fn registry_count(&self,registry:Registry)->OrderResult<usize>{
        if registry==object_registry::OBJECTS{return self.object_count_b39f58()}
        if registry!=COMMANDS{return required("live payload needs actual reached definition registry count")}
        self.command_count()
    }
    fn team_command_flags(&self,_:u32)->OrderResult<u32>{required("live payload needs actual TeamCommand flag source")}
    fn definition_kind(&self,_:Registry,_:u32)->OrderResult<u32>{required("live payload needs actual reached definition kind")}
    fn zero_width_list_budget(&self)->usize{usize::MAX}
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn actual_live_player_reader_matches_native_first_live_match_and_missing_id(){
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-player-order-bank/native-references.json")).unwrap();
        let mut owner=LocalBootstrap::from_native_constructors();
        for(id,film)in[(0,false),(7,true),(7,false),(u32::MAX,false),(7,false)]{
            let mut core=PlayerFields::from_constructor(id,crate::lobby::PeerLink::Node);core.film=film;
            owner.session.players.push(OwnedPlayer{allocation_lifetime:Arc::new(()),core,vector_44:vec![],view:None});
        }
        owner.host_node=Some(crate::local_host::Node::before_session_reset().unwrap());
        let parts=owner.runtime_borrow().unwrap();let players=RefCell::new(parts.players);
        let rows=fixture["rows"].as_array().unwrap().iter().filter(|r|r["kind"]=="player");
        let mut count=0;
        for row in rows{
            let id=row["input"]["id"].as_u64().unwrap()as u32;
            assert_eq!(live_player(&players,id).unwrap(),!row["output"]["ordinal"].is_null());
            let found=players.borrow().find_order_player(id,false);
            assert_eq!(found.as_ref().map(|p|p.slot),row["output"]["ordinal"].as_u64().map(|v|v as usize));
            count+=1;
        }
        assert_eq!(count,24);
        let guard=players.borrow_mut();
        assert!(matches!(live_player(&players,7),Err(OrderError::SchemaSource(_))));
        drop(guard);assert!(live_player(&players,7).unwrap());
    }
}
