//! Actual4D62F0 state reentry then4CFEA0 over the same published ActorStore.
//! No held LoadShell, copied state bits, reconstructed spatial map or ignored
//! positive zone child can stand in for these synchronous current owners.
use crate::{
    actor_world::{ActorAccess,ObjectKey,Spatial},catalog::Catalog,definitions::Bank,
    live_object_init::Source,load_world_globals::{PointShape,SessionPhase},
    stored_actor_state_effects,
};
use aa_actors::{ActorId,actor_creation::{Error,Result},actor_registration,components::{ComponentSlot,StateBits},loading::LoadKind};
use aa_numeric::Precision;
use aa_spatial::geometry::Placement;
use aa_zones::tile::ZoneMap;
#[cfg(test)]#[path="stored_actor_registration_tests.rs"]mod tests;

///All loans refer to the actual mission's disjoint owners. The parent4F9510
///wrapper can still borrow its own registration lists independently.
pub struct Context<'a,'store>{
    pub state:stored_actor_state_effects::Context<'a,'store>,
    pub zones:&'a mut ZoneMap,
    pub point:&'a PointShape,
    pub session:&'a dyn SessionPhase,
    pub mission:&'a crate::mission_fields::State,
}
fn error(e:impl std::fmt::Display)->Error{Error::World(e.to_string())}

fn checked(actors:&ActorAccess<'_>,key:ObjectKey,expected:Option<ActorId>)->Result<ActorId>{
    let owner=actors.id(key).ok_or(Error::Unsupported("stored4D62F0 actual Actor GTID absent"))?;
    if owner.is_null()||actors.key(owner)!=Some(key)||expected.is_some_and(|id|id!=owner){
        return Err(Error::Unsupported("stored4D62F0 current generation differs"));
    }
    let object=actors.object(key).map_err(error)?;
    if object.kind!=LoadKind::Actor||object.object.is_none()||object.actor.is_none()
        ||object.definition.native_class!=3||object.definition.proof.category.runtime_class!=3
        ||!crate::load_spatial::same_proof(&object.thing.definition,&object.definition.proof){
        return Err(Error::Unsupported("stored4D62F0 requires same ordinary Actor constructor/source"));
    }
    Ok(owner)
}
fn bind_source(catalog:&Catalog<'_>,actors:&ActorAccess<'_>,bank:&Bank,key:ObjectKey)->Result<Source>{
    let object=actors.object(key).map_err(error)?;
    let source=Source::bind_in_bank(catalog,bank,&object.definition)?;
    let mask=catalog.canonical_component_mask(&object.definition.proof.id,source.source_precision()).map_err(error)?;
    if object.definition.component_mask!=Some(mask){
        return Err(Error::Unsupported("stored4D62F0 canonical source component mask differs"));
    }
    if let Some(binding)=&actors.entity(key).map_err(error)?.bindings{
        let actual=catalog.actor_bindings(&object.definition.proof.id,source.source_precision()).map_err(error)?;
        if !std::sync::Arc::ptr_eq(binding,&actual){
            return Err(Error::Unsupported("stored4D62F0 retained source allocation differs"));
        }
    }
    Ok(source)
}

///Called only at4F9510's actual Actor branch. Native count is captured once;
///each state read and each entry callback uses the same current allocation.
pub fn register(catalog:&Catalog<'_>,context:&mut Context<'_,'_>,key:ObjectKey,pc:Precision)->Result<()>{
    let owner=checked(context.state.actors,key,None)?;
    bind_source(catalog,context.state.actors,context.state.bank,key)?;
    let ids=crate::load_actor_registration::state_ids_in_bank(catalog,context.state.bank)?;
    //Portable binding guard: actual fresh Object Init must already have moved
    //its placement into this same SpatialWorld before registration can run.
    if context.state.actors.staged_placement_mut(key).is_ok(){
        return Err(Error::Unsupported("stored4D62F0 requires actual published placement"));
    }
    if context.state.spatial.index().precision()!=pc{
        return Err(Error::Unsupported("stored4D62F0 actual index/live precision differs"));
    }
    let spatial_key=context.state.spatial.key(key).map_err(error)?;
    if context.state.spatial.object(spatial_key).map_err(error)?!=key
        ||context.state.spatial.index().resolve_reference(owner.raw())!=Some(spatial_key)
        ||context.state.spatial.index().member(spatial_key).map_err(error)?.reference!=owner.raw(){
        return Err(Error::Unsupported("stored4D62F0 actual published identity differs"));
    }
    actor_registration::register(&mut Bound{catalog,context,key,owner,ids,pc})
}
struct Bound<'a,'c,'source,'owners,'store>{
    catalog:&'c Catalog<'source>,context:&'a mut Context<'owners,'store>,
    key:ObjectKey,owner:ActorId,ids:Vec<String>,pc:Precision,
}
impl actor_registration::World for Bound<'_,'_,'_,'_,'_>{
    fn registered_state_count(&mut self)->Result<i32>{i32::try_from(self.ids.len()).map_err(error)}
    fn state_bits(&mut self)->Result<StateBits>{
        checked(self.context.state.actors,self.key,Some(self.owner))?;
        Ok(self.context.state.actors.object(self.key).map_err(error)?.components().map_err(error)?.state)
    }
    fn state_reenter(&mut self,index:u32)->Result<()>{
        let name=self.ids.get(index as usize).ok_or(Error::Unsupported("stored4D62F0 actual state index"))?;
        let descriptor=aa_actors::state::DESCRIPTORS.get(index as usize).ok_or(Error::Unsupported("stored4D62F0 native descriptor index"))?;
        if !name.eq_ignore_ascii_case(descriptor.name){return Err(Error::Unsupported("stored4D62F0 native descriptor order differs"))}
        let result=stored_actor_state_effects::enter(self.catalog,&mut self.context.state,self.owner,
            u8::try_from(index).map_err(error)?,None,self.pc).map_err(error);
        let owner=checked(self.context.state.actors,self.key,Some(self.owner));
        match(result,owner){
            (Ok(()),Ok(_))=>Ok(()),(Err(prior),Ok(_))=>Err(prior),
            (Ok(()),Err(owner))=>Err(owner),(Err(prior),Err(owner))=>Err(error(format!("{prior}; {owner}"))),
        }
    }
    fn grid_register(&mut self)->Result<()>{grid_register(self.catalog,self.context,self.key,self.owner,self.pc)}
}

fn grid_register(catalog:&Catalog<'_>,context:&mut Context<'_,'_>,key:ObjectKey,owner:ActorId,pc:Precision)->Result<()>{
    checked(context.state.actors,key,Some(owner))?;
    let source=bind_source(catalog,context.state.actors,context.state.bank,key)?;
    context.point.validate_catalog(catalog).map_err(error)?;
    let spatial=&*context.state.spatial;
    let index=spatial.index();let grid=index.grid();let geometry=context.zones.geometry;
    if grid.dimensions!=[geometry.width,geometry.height]||grid.coordinate_shift!=geometry.shift
        ||grid.cell_size_bits!=geometry.tile_size_bits||index.precision()!=pc{
        return Err(Error::Unsupported("stored4CFEA0 zone/index geometry or live precision differs"));
    }
    let spatial_key=spatial.key(key).map_err(error)?;
    let member=index.member(spatial_key).map_err(error)?;
    if spatial.object(spatial_key).map_err(error)?!=key||member.reference!=owner.raw()
        ||index.resolve_reference(owner.raw())!=Some(spatial_key){
        return Err(Error::Unsupported("stored4CFEA0 actual spatial generation differs"));
    }
    let definition=context.state.bank.require_usable(member.actor.definition).map_err(error)?;
    let object=context.state.actors.object(key).map_err(error)?;
    if definition.key.domain!="actordefinition"||!definition.key.name.eq_ignore_ascii_case(&object.definition.proof.id)
        ||member.blocking.kind!=source.blocking_shape().kind
        ||member.blocking.radius_bits!=source.blocking_shape().radius_bits||member.blocking.bounds!=source.blocking_shape().bounds{
        return Err(Error::Unsupported("stored4CFEA0 actual same-Bank spatial source shape differs"));
    }
    let fields=spatial.placement(owner).map_err(error)?;
    let placement=Placement{shape:source.blocking_shape().clone(),
        facing:fields.facing_bits[..2].try_into().unwrap(),position:fields.position_bits[..2].try_into().unwrap()};
    let cached=fields.cached_tile.ok_or(Error::Unsupported("stored4CFEA0 actual44/48 cache absent"))?;
    crate::load_zone_entry::visit_indices(grid,geometry.extent_bits,context.point.shape(),&placement,fields.flags,cached,pc,|tile|{
        //54EF10 reads these current owners lazily at EACH native iterator tile.
        if context.session.application_state().map_err(error)?==3&&context.mission.started48()==0{return Ok(())}
        let Some(set)=context.zones.tile(tile).map_err(error)?.set else{return Ok(())};
        for entry in context.zones.pool.entries(Some(set)).map_err(error)?{
            crate::load_zone_entry::entered_fields(entry,&mut Entered{catalog,actors:context.state.actors,
                bank:context.state.bank,spatial:context.state.spatial,key,owner})?;
        }
        Ok(())
    })
}
struct Entered<'a,'store,'source>{catalog:&'a Catalog<'source>,actors:&'a ActorAccess<'store>,bank:&'a Bank,
    spatial:&'a crate::spatial_world::SpatialWorld,key:ObjectKey,owner:ActorId}
impl crate::load_zone_entry::EnteredFields for Entered<'_,'_,'_>{
    fn control_entered(&mut self,zone_actor:ActorId)->Result<()>{
        checked(self.actors,self.key,Some(self.owner))?;
        crate::stored_registration_control::entered(self.catalog,self.actors,self.bank,self.spatial,zone_actor,self.owner)
    }
    fn city_entered(&mut self,_city:ActorId)->Result<()>{
        Err(Error::Unsupported("stored515620 actual City8 entered owner/RNG/component effects"))
    }
    fn detection_present(&mut self)->Result<bool>{
        checked(self.actors,self.key,Some(self.owner))?;
        Ok(self.actors.object(self.key).map_err(error)?.components().map_err(error)?.present(ComponentSlot::Detection))
    }
}
