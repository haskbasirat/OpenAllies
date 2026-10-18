//! Concrete4FDA70 traversal over the same staged actor/RNG/source owners.
//! Reuses the native coordinator and captured replacement leaves. Empty
//! replacement vectors complete their real no-iteration drain; positive
//! destruction/creation/publication remains a required4FC4A0 boundary.
use crate::{actor_world::{ObjectKey,Source,SpecialSource,Spatial,PlacementState},
    catalog::Catalog,live_clocked::SyncLog,restoration::Staged,
    skirmish_start::{Definitions,Substitution},map_replacement_queue::{self as capture,Queue}};
use aa_actors::{ActorId,registry::IdentityRegistry};
use aa_mapworld::map_prepare::{self as native,RandomKind};
use aa_numeric::Precision;
fn error(e:impl std::fmt::Display)->native::Error {native::Error::Host(e.to_string())}
type Result<T>=native::Result<T>;

fn next<T>(registry:&IdentityRegistry<T>,cursor:&mut u32)->Option<ActorId> {
    let first=if *cursor==0 {1}else{(*cursor&0xffff)+1};
    let id=registry.iter().find(|entry|u32::from(entry.id.slot())>=first).map(|entry|entry.id);
    *cursor=id.map_or(0,ActorId::raw);id
}
struct StagedOnly;
impl Spatial for StagedOnly {
    fn placement(&self,_:ActorId)->crate::actor_world::Result<PlacementState> {
        Err(crate::actor_world::Error::Missing("mission replacement needs actual spatial placement after adoption"))
    }
    fn publish(&mut self,_:ActorId,_:PlacementState)->crate::actor_world::Result<()> {
        Err(crate::actor_world::Error::Missing("mission replacement cannot publish through a read-only staged loan"))
    }
}
struct World<'a,'source,'log> {
    catalog:&'a Catalog<'source>,staged:&'a mut Staged,source_pc:Precision,precision:Precision,
    sync:&'a mut SyncLog<'log>,sim_sync:&'a mut SyncLog<'log>,definitions:Option<Definitions>,
}
impl World<'_, '_, '_> {
    fn bind_definitions(&mut self)->Result<()> {
        if self.definitions.is_none() {
            self.definitions=Some(Definitions::bind(self.catalog.actors,&mut self.staged.definitions,self.source_pc).map_err(error)?);
        }
        Ok(())
    }
    fn token(&self,definition:u32)->Result<&str> {
        let entry=self.staged.definitions.entry(definition).map_err(error)?;
        if entry.key.domain!="actordefinition" {return Err(error("map preparation definition token belongs to another native domain"))}
        self.catalog.require_untranslated_actor_identity(&entry.key.name).map_err(error)?;
        Ok(&entry.key.name)
    }
    fn kingdom(&self,id:ActorId)->Result<&aa_mapworld::special_state::SpecialShell> {
        let key=self.staged.actors.key(id).ok_or_else(||error("map preparation Kingdom GTID is stale"))?;
        let entity=self.staged.actors.entity(key).map_err(error)?;
        let Source::Special(value)=&entity.source else {return Err(error("map preparation owner is not a held special allocation"))};
        let SpecialSource::Kingdom(saved)=&value.source else {return Err(error("map preparation owner is not a Kingdom"))};
        if saved.serialized_id!=id.raw() {return Err(error("map preparation Kingdom identity differs"))}
        value.derived.as_ref().ok_or_else(||error("map preparation Kingdom derived storage absent"))
    }
    fn log(sync:&mut SyncLog<'_>,label:&str,word:Option<u32>)->Result<()> {
        let gate=if sync.owner.enabled&&sync.owner.suspend<1 {(sync.gate)().map_err(error)?}else{false};
        if let Some(word)=word {sync.owner.log_int(gate,label,word)}else{sync.owner.log_marker(gate,label)}
        Ok(())
    }
}
impl native::World for World<'_, '_, '_> {
    type Object=ObjectKey;type Definition=u32;type Kingdom=ActorId;type Nation=usize;type Batch=Queue;
    fn begin_replacements(&mut self)->Result<Queue> {Ok(Queue::default())}
    fn live_count(&mut self)->Result<i32> {i32::try_from(self.staged.actors.identities().len()).map_err(error)}
    fn next_object(&mut self,cursor:&mut u32)->Result<Option<ObjectKey>> {
        Ok(next(self.staged.actors.identities(),cursor).and_then(|id|self.staged.actors.key(id)))
    }
    fn definition(&mut self,object:ObjectKey)->Result<u32> {
        let d=&self.staged.actors.object(object).map_err(error)?.definition;
        self.catalog.require_untranslated_actor_identity(&d.proof.id).map_err(error)?;
        let source=self.catalog.actors.find(&d.proof.id).ok_or_else(||error("map preparation source definition absent"))?;
        let category=self.catalog.actors.native_category(&d.proof.id).map_err(error)?;
        if source.source!=d.proof.source || source.template_chain!=d.proof.template_chain || category!=d.proof.category
            || category.runtime_class!=d.native_class {
            return Err(error("map preparation held immutable definition differs from canonical source"));
        }
        self.staged.definitions.token("ActorDefinition",&d.proof.id).ok_or_else(||error("map preparation actual definition has no held Bank identity"))
    }
    fn category(&mut self,definition:u32)->Result<u32> {
        self.catalog.actors.native_category(self.token(definition)?).map(|d|d.registry_index).map_err(error)
    }
    fn owner(&mut self,object:ObjectKey)->Result<Option<ActorId>> {
        let runtime=self.staged.actors.object(object).map_err(error)?;
        if runtime.definition.native_class!=3 || runtime.actor.is_none() {
            return Err(error("placeholder owner requires actual ordinary GActor virtual108 dispatch"));
        }
        let id=self.staged.actors.id(object).ok_or_else(||error("map preparation held object has no GTID"))?;
        self.staged.actors.ordinary_owner(id).map_err(error)
    }
    fn kingdom_word_190(&mut self,kingdom:ActorId)->Result<u32> {
        self.kingdom(kingdom)?.words.get(&0x190).copied().ok_or_else(||error("map preparation Kingdom+190 absent"))
    }
    fn nation(&mut self,kingdom:ActorId)->Result<Option<usize>> {
        let shell=self.kingdom(kingdom)?;
        if let Some(value)=shell.references.get(&0x234) {
            let Some(value)=value else{return Ok(None)};
            let registry=self.catalog.actors.native_data_registry("Nation").ok_or_else(||error("Nation registry absent"))?;
            let found:Vec<_>=registry.entries.iter().enumerate().filter(|(_,e)|e.ids.eq_ignore_ascii_case(&value.id)).collect();
            let [(ordinal,entry)]=found.as_slice() else{return Err(error("canonical Nation reference has no unique registered identity"))};
            self.catalog.actors.native_data_node(entry).map_err(error)?;
            if value.index.is_some_and(|v|v as usize!=*ordinal) {return Err(error("canonical Nation ordinal differs"))}
            return Ok(Some(*ordinal));
        }
        if shell.words.get(&0x234)==Some(&0) {return Ok(None)}
        Err(error("map preparation actual Nation+234 owner is unbound"))
    }
    fn substitution(&mut self,nation:Option<usize>,definition:u32)->Result<Option<u32>> {
        let nation=nation.ok_or_else(||error("473370 dereferences actual null Nation"))?;
        self.token(definition)?;self.bind_definitions()?;
        let registry=self.catalog.actors.native_data_registry("Nation").ok_or_else(||error("Nation registry absent"))?;
        let nation=&registry.entries.get(nation).ok_or_else(||error("map preparation canonical Nation ordinal absent"))?.ids;
        let rows=self.definitions.as_ref().unwrap().nations.get(&nation.to_ascii_lowercase()).ok_or_else(||error("Nation substitution list unbound"))?;
        Ok(match crate::skirmish_start::substitution(rows,definition) {Substitution::Missing=>None,Substitution::Present(v)=>v})
    }
    fn replacement_is_actor(&mut self,definition:u32)->Result<bool> {
        self.token(definition)?;self.bind_definitions()?;
        let d=self.definitions.as_ref().unwrap().things.get(&definition).ok_or_else(||error("replacement native definition absent"))?;
        //Actual typed ObjectData+38 is true only for ActorData families.
        match d.native_kind {1|18|20|21=>Ok(false),2|19=>Ok(true),_=>Err(error("replacement typed virtual38 dispatch is unsupported"))}
    }
    fn missing_substitution(&mut self,_:ObjectKey)->Result<()> {
        Err(error("4FDA70 missing substitution requires actual404F60 application diagnostic sink"))
    }
    fn random_scope(&mut self,kind:RandomKind)->Result<()> {
        Self::log(self.sync,match kind {RandomKind::Object=>"KKE_THING_CATEGORY_RANDOM_OBJECT",RandomKind::Actor=>"KKE_THING_CATEGORY_RANDOM_ACTOR"},None)
    }
    fn rng_word(&mut self)->Result<u32> {Ok(self.staged.rng.state)}
    fn log_rng(&mut self,channel:u8,seed:u32)->Result<()> {
        match channel {0=>Self::log(self.sync,"(int)m_rng.GetSeed()",Some(seed)),1=>Self::log(self.sim_sync,"(int)m_rng.GetSeed()",Some(seed)),_=>Err(error("native RNG log channel differs"))}
    }
    fn choose_random(&mut self,definition:u32,kind:RandomKind)->Result<Option<u32>> {
        self.token(definition)?;self.bind_definitions()?;
        let defs=self.definitions.as_ref().unwrap();
        let d=defs.things.get(&definition).ok_or_else(||error("random definition absent"))?;
        if d.native_kind!=match kind {RandomKind::Object=>18,RandomKind::Actor=>19} {return Err(error("random table native class differs"))}
        defs.random.get(&definition).ok_or_else(||error("random source table absent"))?.choose(&mut self.staged.rng,self.precision).map_err(error)
    }
    fn queue_replacement(&mut self,batch:&mut Queue,object:ObjectKey,definition:Option<u32>,kind:RandomKind)->Result<()> {
        self.bind_definitions()?;
        let context=capture::Context {actors:&self.staged.actors,spatial:&StagedOnly,definitions:self.definitions.as_ref().unwrap(),bank:&self.staged.definitions,source:self.catalog.actors,precision:self.source_pc};
        let mut warnings=Vec::new();
        context.capture(batch,object,definition,match kind {RandomKind::Object=>capture::Kind::Object,RandomKind::Actor=>capture::Kind::Actor},&mut warnings).map_err(error)?;
        if !warnings.is_empty() {return Err(error("replacement capture foreign property requires actual diagnostic sink"))}
        Ok(())
    }
    fn process_replacements(&mut self,batch:&mut Queue)->Result<()> {
        if batch.objects().is_empty()&&batch.actors().is_empty() {Ok(())}else{Err(error("4FC4A0 nonempty actual replacement queue requires held destruction, fresh creation, property transfer and publication"))}
    }
    fn finish_replacements(&mut self,_batch:Queue)->Result<()> {Ok(())}
}
pub(crate) fn run<'log>(staged:&mut Staged,catalog:&Catalog<'_>,sync:&mut SyncLog<'log>,sim_sync:&mut SyncLog<'log>,precision:Precision)->Result<()> {
    catalog.require_source_owner().map_err(error)?;
    let source=staged.definitions.combat_source().map_err(error)?;source.completed_means().map_err(error)?;
    if source.source_identities()!=catalog.actors.sources.as_slice() {return Err(error("map preparation moved source owner differs"))}
    let source_pc=source.source_precision();
    native::resolve_placeholders_and_random_things(&mut World{catalog,staged,source_pc,precision,sync,sim_sync,definitions:None})
}
#[cfg(test)]
#[path="map_prepare_world_tests.rs"]
mod tests;
