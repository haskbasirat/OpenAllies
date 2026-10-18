//!Selected immutable ActorData fields consumed by VActor453F10. One canonical
//!Data entry retains them; no View gets a second Data/count owner. This is not
//!a full ActorData loader or a source of Tire payload/bone/resource readiness.
use super::{Data,State,Result};
use crate::{catalog::Catalog,definitions::Bank};
use aa_mapworld::restore::RuntimeDefinition;
use aa_numeric::Precision;
use aa_rules::{actors::ResolvedNode,native::{TypedValue,ScalarType},resources::Provenance,turrets::TurretDefinition};

#[derive(Debug)]
pub(super) struct Fields{
    actor:bool,precision:Precision,source:Provenance,
    turrets:Vec<TurretDefinition>,tires:Vec<Vec<Provenance>>,tracks:Vec<Vec<Provenance>>,circumference:u32,
}
///Readonly identity capability for the actual canonical Data allocation.
///There is no raw count/value constructor or detached source-owner clone.
pub struct ActorView{data:Data}
fn actor_node<'a>(catalog:&'a Catalog<'_>,d:&RuntimeDefinition)->Result<ResolvedNode<'a>>{
    let mut found=None;
    for tag in ["Thing","Unit","Projectile","VFX"]{
        for node in catalog.actors.resolved_definitions(tag).map_err(|e|e.to_string())?{
            if node.sources().first()==Some(&d.proof.source){
                if found.is_some(){return Err("Actor view canonical source position is ambiguous".into())}
                found=Some(node);
            }
        }
    }
    let node=found.ok_or("Actor view canonical source absent")?;
    if node.sources()!=d.proof.template_chain{return Err("Actor view source layers differ".into())}Ok(node)
}
fn project(catalog:&Catalog<'_>,d:&RuntimeDefinition,precision:Precision)->Result<Fields>{
    let actor=d.proof.category.native_kind==2;
    //4624B0 constructor stores1FC=0,244=0,23C=0. Only the reached
    //Performer/Mover source branches below replace those actual fields.
    let mut fields=Fields{actor,precision,source:d.proof.source.clone(),turrets:vec![],tires:vec![],tracks:vec![],circumference:0};
    if actor{
        fields.turrets=catalog.actors.native_turrets(&d.proof.id,precision).map_err(|e|e.to_string())?;
        let root=actor_node(catalog,d)?;
        if let Some(mover)=root.child("MoverComponent").map_err(|e|e.to_string())?{
            fields.circumference=match mover.field("tire_circumference",precision).map_err(|e|e.to_string())?{
                //465A6E calls5B5130 GetFixed, then465A73 spills binary32
                //toData23C. The source declaration is FIXED, not FLOAT.
                Some(TypedValue::Scalar(v))if v.field_type==ScalarType::Fixed=>v.bits,
                _=>return Err("ActorData23C requires actual typed Mover tire_circumference".into()),
            };
            aa_numeric::Binary::from_f32_bits(fields.circumference).map_err(|e|e.to_string())?;
            if f32::from_bits(fields.circumference)==0.0{return Err("ActorData loader rejects zero Mover tire_circumference".into())}
            for tire in mover.children("Tire").map_err(|e|e.to_string())?{fields.tires.push(tire.sources());}
            //464BD0's distinct TrackInfo25C/260 list. It must never be
            //confused with Tire240/244; this binds count/source identities,
            //not the positive42D7B0/42D830 VActor Track payload.
            for track in mover.children("TrackInfo").map_err(|e|e.to_string())?{fields.tracks.push(track.sources());}
        }
    }
    //The native consumers compare counts as signed32. Do not wrap an owned
    //Rust list length or silently truncate it to a fabricated field value.
    i32::try_from(fields.turrets.len()).map_err(|_|"ActorData1FC count exceeds native signed domain")?;
    i32::try_from(fields.tires.len()).map_err(|_|"ActorData244 count exceeds native signed domain")?;
    i32::try_from(fields.tracks.len()).map_err(|_|"ActorData260 count exceeds native signed domain")?;
    Ok(fields)
}
impl State{
    pub fn actor_view_source(&mut self,catalog:&Catalog<'_>,bank:&Bank,d:&RuntimeDefinition)->Result<ActorView>{
        let precision=super::validate_object_data(catalog,bank,d)?;
        let data=self.bind_object(catalog,bank,d)?;
        if let Some(fields)=&self.entry(&data)?.actor_view_source{
            if fields.source!=d.proof.source||fields.precision!=precision||fields.actor!=(d.proof.category.native_kind==2){
                return Err("Actor view current source/PC differs from retained Data fields".into())
            }
        }else{
            let fields=project(catalog,d,precision)?;
            self.entry_mut(&data)?.actor_view_source=Some(fields);
        }
        Ok(ActorView{data})
    }
    fn actor_view_fields(&self,view:&ActorView)->Result<&Fields>{
        self.entry(&view.data)?.actor_view_source.as_ref().ok_or_else(||"Actor view source fields absent".into())
    }
    pub fn actor_view_is_actor_38(&self,view:&ActorView)->Result<bool>{Ok(self.actor_view_fields(view)?.actor)}
    pub fn actor_view_turret_count_1fc(&self,view:&ActorView)->Result<i32>{
        let f=self.actor_view_fields(view)?;if !f.actor{return Err("non-ActorData1FC is unbound".into())}
        i32::try_from(f.turrets.len()).map_err(|_|"ActorData1FC count domain changed".into())
    }
    pub fn actor_view_turret_pitch_14(&self,view:&ActorView,index:usize)->Result<u32>{
        let f=self.actor_view_fields(view)?;if !f.actor{return Err("non-ActorData turret is unbound".into())}
        Ok(f.turrets.get(index).ok_or("ActorData turret index outside actual source list")?.pitch_start_bits)
    }
    pub fn actor_view_secondary_count_244(&self,view:&ActorView)->Result<i32>{
        let f=self.actor_view_fields(view)?;if !f.actor{return Err("non-ActorData244 is unbound".into())}
        i32::try_from(f.tires.len()).map_err(|_|"ActorData244 count domain changed".into())
    }
    pub fn actor_view_source_word_23c(&self,view:&ActorView)->Result<u32>{
        let f=self.actor_view_fields(view)?;if !f.actor{return Err("non-ActorData23C is unbound".into())}Ok(f.circumference)
    }
    pub fn actor_view_track_count_260(&self,view:&ActorView)->Result<i32>{
        let f=self.actor_view_fields(view)?;if !f.actor{return Err("non-ActorData260 is unbound".into())}
        i32::try_from(f.tracks.len()).map_err(|_|"ActorData260 count domain changed".into())
    }
}
#[cfg(test)]
#[path="data_actor_view_source_tests.rs"]
mod tests;
