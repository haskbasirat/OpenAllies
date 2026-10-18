//!Immutable fields of the SAME completed canonical Bone directory consumed
//!by Model5A94F0/5A9840. This is neither a second manager nor authored-order
//!selection; the actual native registration directory provides every slot.
use crate::{catalog::Catalog,definitions::Bank};
use aa_rules::{native::TypedValue,resources::{Provenance,SourceIdentity}};
use std::sync::{Arc,Weak};
type Result<T>=std::result::Result<T,String>;
#[derive(Debug)]struct Bone{ids:String,name:Vec<u16>,source:Provenance}
#[derive(Debug)]struct Fields{application:Weak<()>,sources:Vec<SourceIdentity>,bones:Vec<Bone>,selection:Option<usize>,defaults:crate::resource_model_defaults::Capability,loading:crate::resource_ni_cache::LoadingContext}
///Cloning loans the same immutable canonical field allocation. No mutable
///Bone/count registry or caller-supplied replacement strings are introduced.
#[derive(Clone,Debug)]pub struct Context(Arc<Fields>);
impl Context{
    pub(crate) fn bind(catalog:&Catalog<'_>,bank:&Bank,application:&Arc<()>,defaults:crate::resource_model_defaults::Capability,loading:crate::resource_ni_cache::LoadingContext)->Result<Self>{
        defaults.require_application(application)?;
        loading.require_application(application)?;
        catalog.require_source_owner()?;
        let actual=bank.combat_source()?;actual.completed_means()?;
        if actual.source_identities()!=catalog.actors.sources.as_slice(){return Err("Model Bone context source Bank differs".into())}
        let numeric=crate::combat_value_source_context::bind_numeric(catalog,actual.context())?;
        crate::combat_source_receipt::require_reference_numeric(&numeric)?;
        let registry=catalog.actors.native_data_registry("Bone").ok_or("Model requires actual native Bone registry, not authored roots")?;
        if registry.mode!=Some(0){return Err("Model Bone native registry mode differs".into())}
        let mut bones=Vec::new();bones.try_reserve(registry.entries.len()).map_err(|e|e.to_string())?;
        for entry in &registry.entries{
            let node=catalog.actors.native_data_node(entry).map_err(|e|e.to_string())?;
            let value=node.field("bone_name",actual.source_precision()).map_err(|e|e.to_string())?;
            let Some(TypedValue::String{value,..})=value else{return Err("47E270 requires actual typed Bone bone_name".into())};
            //454810's ASCII domain is proved separately. Other conversion
            //semantics must be bound before accepting a non-ASCII source.
            if !value.is_ascii()||value.contains('\0'){return Err("Bone47E270 non-ASCII/embedded-null narrow conversion is required".into())}
            bones.push(Bone{ids:entry.ids.clone(),name:value.encode_utf16().collect(),source:entry.source.clone()});
        }
        let selection=registry.find("selection_box").map(|found|registry.entries.iter().position(|entry|std::ptr::eq(entry,found)).ok_or("Bone selection lookup escaped actual directory")).transpose()?;
        Ok(Self(Arc::new(Fields{application:Arc::downgrade(application),sources:catalog.actors.sources.clone(),bones,selection,defaults,loading})))
    }
    pub(crate) fn require_application(&self,application:&Arc<()>)->Result<()>{
        if self.0.application.upgrade().is_none_or(|actual|!Arc::ptr_eq(&actual,application)){return Err("Model Bone context belongs to another retired Application".into())}Ok(())
    }
    pub fn selection_box_name(&self)->Result<Option<&[u16]>>{
        if self.0.application.upgrade().is_none(){return Err("Model Bone context Application is retired".into())}
        Ok(self.0.selection.map(|index|self.0.bones[index].name.as_slice()))
    }
    pub fn first_controller_bone_name(&self)->Result<&[u16]>{
        if self.0.application.upgrade().is_none(){return Err("Model Bone context Application is retired".into())}
        self.0.bones.first().map(|bone|bone.name.as_slice()).ok_or_else(||"5A9840 actual first Bone vector entry is absent".into())
    }
    ///The SAME initialized process property owner used by Model propagation.
    pub fn default_property(&self,kind:crate::resource_model_defaults::Kind)->Result<crate::resource_model_defaults::Handle>{
        if self.0.application.upgrade().is_none(){return Err("Model source context Application is retired".into())}
        self.0.defaults.get(kind)
    }
    pub fn remove_base_mapping_current(&self)->Result<bool>{self.0.loading.remove_base_mapping_current()}
}
