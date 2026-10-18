//!7A0D40 on the actual retained map, major slots, GTID table and SAI owners.
//! A returned error preserves the manager and map prefix; ready=true alone
//! never constitutes a completed startup or an application activation seal.
use crate::{actor_world::{ActorStore,RuntimeObject},catalog::Catalog,definitions::Bank,
    load_kingdom_init_world::KingdomRegistryState,load_sai_players::Arena,
    load_sai_world::{Collections,Fields,Map},spatial_world::SpatialWorld};
use aa_actors::{ActorId,loading::LoadKind};
use aa_navigation::startup::{self,Object,World};
use std::collections::BTreeMap;
fn err(e:impl std::fmt::Display)->startup::Error {startup::Error(e.to_string())}
type Result<T>=startup::Result<T>;

pub fn initialize<'a,'source>(manager:&'a mut aa_mind::sai::manager::Sai,players:&'a mut Arena,
    catalog:&'a Catalog<'source>,bank:&'a Bank,inventory:&'a aa_mapworld::WorldInventory,
    actors:&'a mut ActorStore,spatial:&'a SpatialWorld,kingdoms:&'a KingdomRegistryState,
    zones:&'a aa_zones::tile::ZoneMap,map:&'a mut Map,revision:&'a crate::world_revision::ConstructedWorldRevision)->std::result::Result<(),String> {
    if manager.ready {return Err("SAI world startup needs its actual unstarted manager; ready alone is not completion".into())}
    map.bind_startup_actors(catalog,bank,inventory,zones,actors)?;
    map.begin_startup()?;
    crate::load_sai_world::initialize_manager(manager,|collections|Live {collections,players,catalog,bank,inventory,
        actors,spatial,kingdoms,zones,map,revision,flags:BTreeMap::new(),context_words:BTreeMap::new()}).map_err(|e|e.to_string())
}
struct Live<'a,'source> {
    collections:Collections<'a>,players:&'a mut Arena,catalog:&'a Catalog<'source>,bank:&'a Bank,
    inventory:&'a aa_mapworld::WorldInventory,actors:&'a mut ActorStore,spatial:&'a SpatialWorld,
    kingdoms:&'a KingdomRegistryState,zones:&'a aa_zones::tile::ZoneMap,map:&'a mut Map,
    revision:&'a crate::world_revision::ConstructedWorldRevision,
    // Read-only results of the immediately preceding constant native getters.
    // No authoritative actor/source field is copied back from these caches.
    flags:BTreeMap<u64,u32>,context_words:BTreeMap<u64,[u32;2]>,
}
impl Live<'_,'_> {
    fn id(object:Object)->Result<ActorId> {Ok(ActorId::from_raw(u32::try_from(object.0).map_err(err)?))}
    fn object(&self,object:Object)->Result<&RuntimeObject> {
        let id=Self::id(object)?;self.catalog.require_source_owner().map_err(err)?;
        let source=self.bank.combat_source().map_err(err)?;source.completed_means().map_err(err)?;
        if source.source_identities()!=self.catalog.actors.sources {return Err(err("SAI world source Bank/catalog differs"))}
        let key=self.actors.key(id).ok_or_else(||err("SAI current GTID generation is absent"))?;
        let value=self.actors.object(key).map_err(err)?;let proof=&value.definition.proof;
        self.catalog.require_untranslated_actor_identity(&proof.id).map_err(err)?;
        let canonical=self.catalog.actors.find(&proof.id).ok_or_else(||err("SAI canonical source absent"))?;
        if canonical.source!=proof.source||canonical.template_chain!=proof.template_chain
            || self.catalog.actors.native_category(&proof.id).map_err(err)?!=proof.category
            || proof.category.runtime_class!=value.definition.native_class
            || self.catalog.actors.native_final_flags(&proof.id,source.source_precision()).map_err(err)?!=value.definition.flags
            || !crate::load_spatial::same_proof(&value.thing.definition,proof) {
            return Err(err("SAI current source/constructor certificate differs"));
        }
        let valid=match value.definition.native_class {
            3|4=>value.kind==LoadKind::Actor&&value.actor.is_some()&&value.object.is_some(),
            2|6|7|9=>value.kind==LoadKind::Object&&value.actor.is_none()&&value.object.is_some(),
            1|5=>value.kind==LoadKind::Thing&&value.actor.is_none()&&value.object.is_none(),
            _=>false,
        };
        if !valid {return Err(err("SAI actual runtime constructor storage is unsupported or differs"))}
        Ok(value)
    }
    fn memory(&self,reference:Object)->Result<&aa_actors::detection_memory::Record> {
        let (actor,id)=self.map.memory_key(self.actors,reference).map_err(err)?;
        let object=self.object(Object(u64::from(actor.raw())))?;
        object.object.as_ref().ok_or_else(||err("SAI seen Object allocation absent"))?.memory.record(id).map_err(err)
    }
}
impl<'a> World<Fields<'a>> for Live<'a,'_> {
    fn clear_list78(&mut self,_:&mut Fields<'a>)->Result<()> {self.collections.commands.clear();Ok(())}
    fn clear_list80(&mut self,_:&mut Fields<'a>)->Result<()> {self.collections.general_powers.clear();Ok(())}
    fn global_revision(&self)->u32 {self.revision.value()}
    fn prepare_topology(&mut self,_:&mut Fields<'a>)->Result<()> {self.map.prepare(self.catalog,self.bank,self.zones).map_err(err)}
    fn kingdom_count(&self)->usize {self.kingdoms.majors().len()}
    fn kingdom_at(&self,i:usize)->Option<Object> {self.kingdoms.majors().get(i).copied().flatten().map(|id|Object(u64::from(id.raw())))}
    fn register_kingdom(&mut self,k:Object,_:&mut Fields<'a>)->Result<()> {
        self.map.register_kingdom_regions(self.catalog,self.bank,self.inventory,self.actors,self.kingdoms,self.zones,Self::id(k)?).map_err(err)?;
        for handle in self.collections.players.iter().flatten() {
            if self.players.player(*handle).map_err(err)?.controller_present() {
                return Err(err("79E170 actual existing AI controller requires7A48F0 Kingdom goal notification"));
            }
        }
        Ok(())
    }
    fn ai_slot_count(&self)->usize {self.collections.players.len()}
    fn ai_slot_at(&self,i:usize)->Option<Object> {self.collections.players.get(i).copied().flatten().map(|h|Object(u64::from(h)))}
    fn start_ai(&mut self,p:Object,_:&mut Fields<'a>)->Result<()> {
        let handle=u32::try_from(p.0).map_err(err)?;self.players.player(handle).map_err(err)?.validate_association().map_err(err)?;
        self.players.start_player(self.catalog,self.bank,self.actors,self.kingdoms,self.map,handle).map_err(err)
    }
    fn registry_count(&self)->i32 {i32::try_from(self.actors.identities().len()).expect("native GTID table is bounded to65535 entries")}
    fn next_registered_after(&self,cursor:u32)->Result<Option<Object>> {
        Ok(self.actors.identities().iter().find(|e|(e.id.raw()&0xffff)>(cursor&0xffff)).map(|e|Object(u64::from(e.id.raw()))))
    }
    fn actor_id(&self,a:Object)->u32 {u32::try_from(a.0).expect("actual GTID iterator produced u32")}
    fn actor_b4(&mut self,a:Object,_:&mut Fields<'a>)->Result<bool> {
        let value=self.object(a)?;let result=matches!(value.definition.native_class,3|4);
        if result {
            let id=Self::id(a)?;let key=self.actors.key(id).ok_or_else(||err("SAI actor identity absent"))?;
            let flags=self.actors.placement(key,self.spatial).map_err(err)?.flags;self.flags.insert(a.0,flags);
        }
        Ok(result)
    }
    fn actor_flags8(&self,a:Object)->u32 {*self.flags.get(&a.0).expect("native constant+B4 getter immediately preceded its flag read")}
    fn register_special_actor(&mut self,a:Object,f:&mut Fields<'a>)->Result<()> {
        crate::load_sai_actor::notify_with_world(self.catalog,self.bank,self.actors,self.spatial,Self::id(a)?,f,self.collections.players,self.players,
            self.map,self.collections.vector90,self.kingdoms).map_err(err)
    }
    fn actor_b0(&mut self,a:Object,_:&mut Fields<'a>)->Result<bool> {Ok(!matches!(self.object(a)?.definition.native_class,1|5))}
    fn seen_record(&mut self,a:Object,k:Object,_:&mut Fields<'a>)->Result<Option<Object>> {
        let kingdom=self.object(k)?;if kingdom.definition.native_class!=4 {return Err(err("SAI major slot is not actual Kingdom"))}
        let id=Self::id(k)?;let key=self.actors.key(id).ok_or_else(||err("SAI major Kingdom absent"))?;
        let crate::actor_world::Source::Special(special)=&self.actors.entity(key).map_err(err)?.source else {return Err(err("SAI major Kingdom derived owner absent"))};
        let derived=special.derived.as_ref().ok_or_else(||err("SAI major Kingdom derived allocation absent"))?;
        if derived.words.get(&0x190)!=Some(&0) {return Ok(None)}
        let index=derived.words.get(&0x18c).copied().ok_or_else(||err("SAI major index absent"))?;
        let value=self.object(a)?;let memory=&value.object.as_ref().ok_or_else(||err("SAI memory Object allocation absent"))?.memory;
        if memory.count()<=0 {return Ok(None)}
        let Some(record)=memory.slot(index).map_err(err)? else {return Ok(None)};
        self.map.remember_memory(self.actors,Self::id(a)?,record).map(Some).map_err(err)
    }
    fn seen_context(&self,r:Object)->Object {self.map.memory_context(r).expect("actual seen reference retained its source context")}
    fn context_eligible(&mut self,c:Object,_:&mut Fields<'a>)->Result<bool> {
        let id=self.map.context_definition(c).map_err(err)?;self.catalog.require_untranslated_actor_identity(id).map_err(err)?;
        let category=self.catalog.actors.native_category(id).map_err(err)?;
        if matches!(category.native_kind,1|20|21) {return Ok(false)}
        if category.native_kind!=2 {return Err(err("SAI seen-context source virtual38 dispatch is unsupported"))}
        let source=self.bank.combat_source().map_err(err)?;
        let mobile=self.catalog.actors.native_mobile_deployment_data(source.source_precision()).map_err(err)?;
        let definition=mobile.definition(id).map_err(err)?;
        self.context_words.insert(c.0,[u32::from(definition.pack_to.is_some()),u32::from(definition.unpack_to.is_some())]);Ok(true)
    }
    fn context_words(&self,c:Object)->[u32;2] {*self.context_words.get(&c.0).expect("actual ActorData virtual38 validated source pointer predicates")}
    fn seen_position(&self,r:Object)->[u32;2] {self.memory(r).expect("actual record retained by map/ActorStore owner").position_bits}
    fn region_at(&mut self,p:[u32;2],_:&mut Fields<'a>)->Result<Option<u32>> {self.map.topology().region_at(p).map_err(err)}
    fn region_kingdom(&mut self,region:u32,k:Object,_:&mut Fields<'a>)->Result<Option<Object>> {
        let id=Self::id(k)?;let state=self.map.regions().regions.get(region as usize).ok_or_else(||err("SAI region absent"))?;
        Ok(state.kingdoms.iter().flatten().any(|r|r.kingdom==k).then_some(Object((u64::from(region)<<32)|u64::from(id.raw()))))
    }
    fn register_seen(&mut self,binding:Object,r:Object,_:&mut Fields<'a>)->Result<()> {
        let _=self.memory(r)?;self.map.register_seen_ref((binding.0>>32)as u32,ActorId::from_raw(binding.0 as u32),r).map_err(err)
    }
}
