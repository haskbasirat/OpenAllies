//!One source-owned ObjectAnimationSet49BF20. The same allocation is nested
//!in its Aspect and shares canonical Data/Resource identity domains. This
//!owner has no Clone/Default/raw-ready or arbitrary-Handle constructor.
use crate::{data_pre_request::Data,fx_graph::{OwnerLookups,Reference},resource_objects::Handle};
use crate::{catalog::Catalog,definitions::Bank,resource_depot::{Backend,Resources}};
use aa_rules::{actors::ResolvedNode,native::{ScalarType,TypedValue},resources::{Provenance,SourceIdentity}};
use aa_numeric::Precision;
type Result<T>=std::result::Result<T,String>;
#[path="object_animation_core.rs"]
pub(crate) mod core;
#[path="object_animation_release.rs"]
mod release;
#[cfg(test)]
#[path="object_animation_set_tests.rs"]
mod tests;
pub struct Set {
    data:Data,
    ///49C020's allocation exists before49D900. A failed field/child load
    ///retains it here before either native vector append has occurred.
    core:core::Core<Handle,Reference,Reference>,
    source:Option<(Vec<SourceIdentity>,Precision)>,
    failed:Option<String>,
}
impl Set {
    ///49BF20/49BC70 initializes one empty vector and eight empty typed
    ///vectors. Native initial capacities16/2 are allocation policies, not
    ///gameplay caps; portable vectors grow without reproducing those pools.
    pub fn from_native_constructor(data:&Data)->Self{
        Self{data:data.clone(),core:core::Core::constructed(),source:None,failed:None}
    }
    ///Called only by the real pre-World Aspect source loader, at49DA40.
    ///The caller owns the exact canonical registration/node/source-folder
    ///selection; the resolved node cannot be synthesized outside aa-rules.
    ///A failure retains every earlier Resource/field store and forbids reuse.
    ///Without the caller's current UVar61 policy a missing Sequence remains
    ///the explicit required-policy error (RequiredTexturePolicy).
    pub(crate) fn load_source_node(&mut self,catalog:&Catalog<'_>,bank:&Bank,node:&ResolvedNode<'_>,
        resource_prefix:&str,source_folder:&str,resources:&mut Resources<'_>)->Result<()> {
        self.load_source_node_with_policy(catalog,bank,node,resource_prefix,source_folder,resources,
            &mut crate::resource_objects::RequiredTexturePolicy)
    }
    ///The caller's current DataCompatibilityMode/DataMissingResourceWarning
    ///reach5A80D0's TGC_MissingResourceException catch: NULL then skips this
    ///Animation exactly as49DA40 does (no allocation, reads or appends).
    pub(crate) fn load_source_node_with_policy<P:crate::resource_objects::TexturePolicy>(&mut self,catalog:&Catalog<'_>,bank:&Bank,
        node:&ResolvedNode<'_>,resource_prefix:&str,source_folder:&str,resources:&mut Resources<'_>,texture_policy:&mut P)->Result<()> {
        if let Some(error)=&self.failed{return Err(error.clone())}
        let result=self.load(catalog,bank,node,resource_prefix,source_folder,resources,texture_policy);
        if let Err(error)=&result{self.failed=Some(error.clone());}
        result
    }
    fn load<P:crate::resource_objects::TexturePolicy>(&mut self,catalog:&Catalog<'_>,bank:&Bank,node:&ResolvedNode<'_>,prefix:&str,folder:&str,
        resources:&mut Resources<'_>,texture_policy:&mut P)->Result<()> {
        self.data.require_live()?;catalog.require_source_owner()?;
        let source=bank.combat_source()?;source.completed_means()?;
        if !crate::catalog_lookups::same_sources(catalog,source.source_identities()){return Err("ObjectAnimation source Bank/universe differs".into())}
        let pc=source.source_precision();let identity=(catalog.actors.sources.clone(),pc);
        if self.source.as_ref().is_some_and(|old|old!=&identity){return Err("ObjectAnimation source owner/PC changed".into())}
        for position in node.sources(){
            if !identity.0.iter().any(|s|s.path==position.path&&s.sha256==position.sha256){return Err("ObjectAnimation node has foreign source provenance".into())}
        }
        self.source=Some(identity);
        self.core.load(prefix,&mut Current{catalog,bank,node,folder,resources,texture_policy,pc,lookups:None})
    }
    ///49B7E0. This source-owned topology is immutable during actual resource
    ///requests; no copied list or request-success/loaded flag is manufactured.
    pub(crate) fn request(&mut self,delta:u32,resources:&mut Resources<'_>,backend:&mut impl Backend)->Result<()> {
        self.data.require_live()?;
        if let Some(error)=&self.failed{return Err(format!("ObjectAnimationSet source load failed: {error}"))}
        self.core.request(delta,|resource,delta|{
            let priority=resources.current_priority_b282f0();
            resources.request_0c(resource,delta as i32,priority,backend)
        })
    }
}
impl std::fmt::Debug for Set{
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.debug_struct("ObjectAnimationSet").field("count",&self.core.animations.len()).field("pending",&self.core.pending.is_some()).field("failed",&self.failed).finish()}
}
struct Current<'a,'source,'node,'resource,P>{catalog:&'a Catalog<'source>,bank:&'a Bank,node:&'a ResolvedNode<'node>,folder:&'a str,resources:&'a mut Resources<'resource>,
    texture_policy:&'a mut P,pc:Precision,lookups:Option<OwnerLookups<'a>>}
impl<'a,P> Current<'a,'_,'_,'_,P>{
    ///Lazily bound only by a positive49D0A0/49CF80 child, so an Animation
    ///without children never depends on the owner derivation below.
    fn lookups(&mut self)->Result<&mut OwnerLookups<'a>>{
        if self.lookups.is_none(){
            let owner=owner(self.catalog,self.node)?;
            self.lookups=Some(OwnerLookups::for_catalog(self.catalog,self.pc,owner)?);
        }
        Ok(self.lookups.as_mut().expect("bound owner lookups"))
    }
}
impl<'node,P:crate::resource_objects::TexturePolicy> core::Source<Handle> for Current<'_,'_,'node,'_,P>{
    type Audio=Reference;type Vfx=Reference;type Child=ResolvedNode<'node>;
    fn string(&mut self,key:&str)->Result<String>{string(self.node,key,self.pc)}
    fn resource(&mut self,expanded:&str)->Result<Option<Handle>>{
        let path=crate::data_pre_request::path::relative(self.folder,expanded)?;
        self.resources.sequence_5a80d0_with_policy(self.catalog,self.bank,&path.encode_utf16().collect::<Vec<_>>(),&mut *self.texture_policy)
    }
    fn float(&mut self,key:&str)->Result<u32>{float(self.node,key,self.pc)}
    fn boolean(&mut self,key:&str)->Result<u8>{boolean(self.node,key,self.pc)}
    fn count(&mut self,name:&str)->Result<usize>{Ok(self.node.children(name).map_err(error)?.len())}
    fn child(&mut self,name:&str,index:usize)->Result<ResolvedNode<'node>>{
        self.node.children(name).map_err(error)?.into_iter().nth(index).ok_or_else(||format!("42BE10 ObjectAnimation {name} child {index} absent"))
    }
    fn child_string(&mut self,child:&ResolvedNode<'node>,key:&str)->Result<String>{string(child,key,self.pc)}
    fn child_boolean(&mut self,child:&ResolvedNode<'node>,key:&str)->Result<u8>{boolean(child,key,self.pc)}
    fn vfx(&mut self,ids:&str)->Result<Option<Reference>>{self.lookups()?.vfx(ids)}
    fn bone(&mut self,ids:&str)->Result<Option<i32>>{
        //42C08A: an empty IDS returns null before any manager access.
        if ids.is_empty(){return Ok(None)}
        match self.lookups()?.bone(ids)?{None=>Ok(None),Some(v)=>i32::try_from(v).map(Some).map_err(error)}
    }
    fn audio(&mut self,ids:&str)->Result<Option<Reference>>{self.lookups()?.audio(ids)}
    fn audio_manager(&mut self)->Result<String>{self.lookups()?.audio_manager()}
}
///49DA40 runs inside its owner's parse-time LoadData (464BD0/4718B0/4922E0
///->49DC40->49C670->46A8A0), so416370/4BDB70/42C070 see only roots reached
///before that owner. The owner is the one registered Thing root whose
///authored span contains this Animation instance. A template-authored
///instance has no proved owner position here and stays an explicit error.
fn owner(catalog:&Catalog<'_>,node:&ResolvedNode<'_>)->Result<Provenance>{
    let sources=node.sources();
    let first=sources.first().ok_or("ObjectAnimation source node absent")?;
    let mut found=None;
    for actor in &catalog.actors.actors{
        let s=&actor.source;
        if s.path==first.path&&s.sha256==first.sha256&&s.span.start<=first.span.start&&s.span.end>=first.span.end{
            if found.is_some(){return Err("ObjectAnimation owning registered Thing root is ambiguous".into())}
            found=Some(s.clone());
        }
    }
    found.ok_or_else(||"ObjectAnimation child lookup requires an Animation authored inside its registered Thing root".into())
}
fn error(e:impl std::fmt::Display)->String{e.to_string()}
fn string(node:&ResolvedNode<'_>,name:&str,pc:Precision)->Result<String>{
    match node.field(name,pc).map_err(error)?{
        Some(TypedValue::String{value,..})if value.is_ascii()&&!value.contains('\0')=>Ok(value),
        _=>Err(format!("ObjectAnimation {name} requires actual plain ASCII String field")),
    }
}
fn float(node:&ResolvedNode<'_>,name:&str,pc:Precision)->Result<u32>{
    match node.field(name,pc).map_err(error)?{
        Some(TypedValue::Scalar(value))if value.field_type==ScalarType::Float&&f32::from_bits(value.bits).is_finite()=>Ok(value.bits),
        _=>Err(format!("ObjectAnimation {name} requires actual source Float field")),
    }
}
fn boolean(node:&ResolvedNode<'_>,name:&str,pc:Precision)->Result<u8>{
    node.field(name,pc).map_err(error)?.ok_or("ObjectAnimation native Bool absent")?.boolean().map(|v|v as u8).map_err(error)
}
