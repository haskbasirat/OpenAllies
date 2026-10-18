//!One source-owned ObjectAnimationSet49BF20. The same allocation is nested
//!in its Aspect and shares canonical Data/Resource identity domains. This
//!owner has no Clone/Default/raw-ready or arbitrary-Handle constructor.
use crate::{data_pre_request::Data,resource_objects::Handle};
use crate::{catalog::Catalog,definitions::Bank,resource_depot::{Backend,Resources}};
use aa_rules::{actors::ResolvedNode,native::{ScalarType,TypedValue},resources::SourceIdentity};
use aa_numeric::Precision;
type Result<T>=std::result::Result<T,String>;
#[path="object_animation_core.rs"]
pub(crate) mod core;
pub struct Set {
    data:Data,
    ///49C020's allocation exists before49D900. A failed field/child load
    ///retains it here before either native vector append has occurred.
    core:core::Core<Handle>,
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
    pub(crate) fn load_source_node(&mut self,catalog:&Catalog<'_>,bank:&Bank,node:&ResolvedNode<'_>,
        resource_prefix:&str,source_folder:&str,resources:&mut Resources<'_>)->Result<()> {
        if let Some(error)=&self.failed{return Err(error.clone())}
        let result=self.load(catalog,bank,node,resource_prefix,source_folder,resources);
        if let Err(error)=&result{self.failed=Some(error.clone());}
        result
    }
    fn load(&mut self,catalog:&Catalog<'_>,bank:&Bank,node:&ResolvedNode<'_>,prefix:&str,folder:&str,resources:&mut Resources<'_>)->Result<()> {
        self.data.require_live()?;catalog.require_source_owner()?;
        let source=bank.combat_source()?;source.completed_means()?;
        if source.source_identities()!=catalog.actors.sources.as_slice(){return Err("ObjectAnimation source Bank/universe differs".into())}
        let pc=source.source_precision();let identity=(catalog.actors.sources.clone(),pc);
        if self.source.as_ref().is_some_and(|old|old!=&identity){return Err("ObjectAnimation source owner/PC changed".into())}
        for position in node.sources(){
            if !identity.0.iter().any(|s|s.path==position.path&&s.sha256==position.sha256){return Err("ObjectAnimation node has foreign source provenance".into())}
        }
        self.source=Some(identity);
        self.core.load(prefix,&mut Current{catalog,bank,node,folder,resources,pc})
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
struct Current<'a,'source,'node,'resource>{catalog:&'a Catalog<'source>,bank:&'a Bank,node:&'a ResolvedNode<'node>,folder:&'a str,resources:&'a mut Resources<'resource>,pc:Precision}
impl core::Source<Handle> for Current<'_,'_,'_,'_>{
    fn string(&mut self,key:&str)->Result<String>{string(self.node,key,self.pc)}
    fn resource(&mut self,expanded:&str)->Result<Option<Handle>>{
        let path=crate::data_pre_request::path::relative(self.folder,expanded)?;
        self.resources.sequence_5a80d0(self.catalog,self.bank,&path.encode_utf16().collect::<Vec<_>>()).map(Some)
    }
    fn float(&mut self,key:&str)->Result<u32>{float(self.node,key,self.pc)}
    fn boolean(&mut self,key:&str)->Result<u8>{self.node.field(key,self.pc).map_err(error)?.ok_or("ObjectAnimation native Bool absent")?.boolean().map(|v|v as u8).map_err(error)}
    fn create_vfx(&mut self)->Result<()>{if self.node.children("CreateVFX").map_err(error)?.is_empty(){Ok(())}else{Err("49D0A0 ObjectAnimation positive CreateVFX requires actual source-position VFX/location lookup".into())}}
    fn play_sfx(&mut self)->Result<()>{if self.node.children("PlaySFX").map_err(error)?.is_empty(){Ok(())}else{Err("49CF80 ObjectAnimation positive PlaySFX requires actual source-position Audio lookup".into())}}
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

