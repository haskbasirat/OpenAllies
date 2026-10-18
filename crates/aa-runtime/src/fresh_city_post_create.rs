//!564BE0/5644F0 over the SAME staged City, source layout and nested creations.
use crate::{actor_world::{ObjectKey,RuntimeComponent,Spatial,PlacementState},catalog::Catalog,restoration::Staged,
    city_post_create_core::{self as core,Row,Request,World},actor_creation_startup::StartupInputs,
    map_replacement_queue::DefinitionRef};
use aa_actors::{ActorId,components::ComponentSlot as Slot};
use aa_numeric::Precision;
type Result<T>=std::result::Result<T,String>;
fn text(e:impl std::fmt::Display)->String{e.to_string()}
pub(crate) fn run(staged:&mut Staged,catalog:&Catalog<'_>,key:ObjectKey,argument:u32,with_values:bool,
    startup:Option<&StartupInputs>,pc:Precision)->Result<()> {
    crate::fresh_post_create_world::canonical(staged,catalog,key)?;
    if with_values {city(staged,key)?;}
    core::run(&mut Current{staged,catalog,key,startup,pc,layouts:None},argument,with_values,pc)
}
fn city(staged:&Staged,key:ObjectKey)->Result<&aa_actors::lair_city_state::CityState>{
    let id=staged.actors.id(key).ok_or("564BE0 current City actor GTID absent")?;
    match staged.actors.object(key).map_err(text)?.components().map_err(text)?.get(Slot::City) {
        Some(RuntimeComponent::City(c))if c.header.owner==id=>Ok(c),
        _=>Err("564BE0 SAME City slot/header owner differs".into()),
    }
}
struct ReadSpatial<'a>(&'a Staged);
impl Spatial for ReadSpatial<'_> {
    fn placement(&self,id:ActorId)->crate::actor_world::Result<PlacementState>{match self.0.load_world.as_ref().map(|w|&w.spatial){
        Some(crate::load_world::SpatialOwner::Adopted(a))if a.error.is_none()&&a.pending.is_empty()=>a.world.placement(id),
        _=>Err(crate::actor_world::Error::Missing("5644F0 actual adopted spatial owner")),
    }}
    fn publish(&mut self,_:ActorId,_:PlacementState)->crate::actor_world::Result<()>{Err(crate::actor_world::Error::Missing("5644F0 readonly pose loan"))}
}
struct Current<'a,'s>{staged:&'a mut Staged,catalog:&'a Catalog<'s>,key:ObjectKey,startup:Option<&'a StartupInputs>,pc:Precision,
    layouts:Option<Vec<Vec<aa_rules::city_layouts::Object>>>}
impl Current<'_,'_>{
    fn check(&self)->Result<&crate::actor_world::RuntimeObject>{crate::fresh_post_create_world::canonical(self.staged,self.catalog,self.key)}
    fn pose(&self)->Result<PlacementState>{self.check()?;self.staged.actors.placement(self.key,&ReadSpatial(self.staged)).map_err(text)}
}
impl World for Current<'_,'_>{
    type Definition=DefinitionRef;type Layout=usize;type Created=ObjectKey;
    fn clear_values(&mut self)->Result<()>{city(self.staged,self.key)?;
        let c=self.staged.actors.object_mut(self.key).map_err(text)?.components_mut().map_err(text)?.get_mut(Slot::City).unwrap();
        let RuntimeComponent::City(c)=c else{unreachable!()};c.values.clear();Ok(())}
    fn select_layout(&mut self,argument:u32)->Result<usize>{
        let runtime=self.check()?;let id=runtime.definition.proof.id.clone();
        let source=self.staged.definitions.combat_source()?;
        let pc=source.source_precision();
        let layouts=self.catalog.actors.native_city_layouts(&id,pc).map_err(text)?.ok_or("5644F0 actual Data5A8 City layout array absent")?;
        let index=if argument==u32::MAX {
            crate::configure_world::log_random_seed(self.catalog,&self.staged.rng)?;
            if layouts.len()==1 {0}else{usize::try_from(self.staged.rng.next_u32()).map_err(text)?%layouts.len()}
        }else{usize::try_from(argument).map_err(text)?};
        if index>=layouts.len(){return Err("5644F0 explicit Params34 outside actual City layout array".into())}
        self.layouts=Some(layouts);Ok(index)
    }
    fn row_count(&mut self,layout:usize)->Result<usize>{Ok(self.layouts.as_ref().and_then(|l|l.get(layout)).ok_or("5644F0 captured source Layout absent")?.len())}
    fn row(&mut self,layout:usize,index:usize)->Result<Row<DefinitionRef>>{
        let row=self.layouts.as_ref().and_then(|l|l.get(layout)).and_then(|l|l.get(index)).ok_or("5644F0 captured source row absent")?;
        self.catalog.require_untranslated_actor_identity(&row.definition_ids)?;
        let token=self.staged.definitions.token("ActorDefinition",&row.definition_ids).ok_or("5644F0 canonical source object definition absent from SAME Bank")?;
        let definition=self.staged.definitions.require_usable(token)?;
        Ok(Row{definition:DefinitionRef{token,canonical_ids:definition.key.name.clone()},offset:row.offset,orientation:row.orientation})
    }
    fn position(&mut self)->Result<[u32;2]>{let p=self.pose()?;Ok([p.position_bits[0],p.position_bits[1]])}
    fn facing(&mut self)->Result<[u32;2]>{let p=self.pose()?;Ok([p.facing_bits[0],p.facing_bits[1]])}
    fn owner_108(&mut self)->Result<u32>{let id=self.staged.actors.id(self.key).ok_or("5644F0 actual Actor GTID absent")?;
        self.check()?;Ok(self.staged.actors.ordinary_owner(id).map_err(text)?.map_or(0,ActorId::raw))}
    fn current_id(&mut self)->Result<u32>{self.check()?;Ok(self.staged.actors.id(self.key).ok_or("5644F0 current Actor GTID absent")?.raw())}
    fn is_actor_data(&mut self,d:&DefinitionRef)->Result<bool>{
        let entry=self.staged.definitions.require_usable(d.token)?;
        if entry.key.domain!="actordefinition"||!entry.key.name.eq_ignore_ascii_case(&d.canonical_ids){return Err("5644F0 source definition token identity differs".into())}
        match self.catalog.actors.native_category(&entry.key.name).map_err(text)?.native_kind {
            2=>Ok(true),0|1|20|21=>Ok(false),
            _=>Err("5644F0 source Data+38 predicate for this derived class is unbound".into()),
        }
    }
    fn create(&mut self,r:Request<DefinitionRef>,actor:bool)->Result<Option<ObjectKey>>{
        if !actor{return Err(format!("5644F0 actual Object4F9C10 factory required for {}",r.definition.canonical_ids))}
        let startup=self.startup.ok_or("5644F0 nested Actor creation requires SAME sealed StartupInputs")?;
        let mut params=crate::map_actor_creation::Params::new(r.definition);
        params.position_bits=r.position;params.direction_bits=r.direction;params.owner=r.owner;params.mode=6;params.word_4c=r.parent;
        crate::map_actor_creation::create_member_thing(self.staged,self.catalog,&mut params,startup,self.pc)
    }
    fn append_settlement(&mut self,created:ObjectKey)->Result<()>{
        let made=self.staged.actors.object(created).map_err(text)?;
        let Some(component)=made.actor.as_ref().and_then(|a|a.components.get(Slot::Structure))else{return Ok(())};
        let RuntimeComponent::Structure(s)=component else{return Err("5644F0 created Settlement slot contains different payload".into())};
        let made_id=self.staged.actors.id(created).ok_or("5642A0 created Settlement GTID absent")?;
        if s.header.owner!=made_id{return Err("5642A0 created Settlement header/owner differs".into())}
        city(self.staged,self.key)?;
        //5642A0 commits its head insertion BEFORE588AB0 writes the child.
        let parent=self.staged.actors.id(self.key).ok_or("5642A0 actual City owner GTID absent")?;
        let current=self.staged.actors.object_mut(self.key).map_err(text)?.components_mut().map_err(text)?.get_mut(Slot::City).unwrap();
        let RuntimeComponent::City(c)=current else{unreachable!()};c.values.insert(0,made_id.raw());
        let child=self.staged.actors.object_mut(created).map_err(text)?.components_mut().map_err(text)?.get_mut(Slot::Structure).ok_or("588AB0 actual Settlement disappeared")?;
        let RuntimeComponent::Structure(s)=child else{return Err("588AB0 actual Settlement type changed".into())};s.word_34=parent.raw();Ok(())
    }
}
