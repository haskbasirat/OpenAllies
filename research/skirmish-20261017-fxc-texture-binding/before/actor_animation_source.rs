//!ActorData1BC/1C0's source-owned animation allocations. This is a selected
//!464BD0 animation-source producer, not whole ActorData/source readiness.
//!Real Sequence factories run before Animation construction; each Animation
//!owns one contextual FX arena shared by its fixed lists and timed cues.
use crate::{catalog::Catalog,definitions::Bank,data_pre_request::{Data,aspect::Layout},
    resource_objects::Handle,resource_depot::{Resources,Backend},fx_graph::{self,Graph,NodeId}};
use aa_mapworld::restore::RuntimeDefinition;
use aa_rules::{actors::ResolvedNode,native::{TypedValue,ScalarType},resources::SourceIdentity};
use aa_numeric::Precision;
#[path="actor_animation_core.rs"]
mod core;
type Result<T>=std::result::Result<T,String>;
struct FxOwner{session:Option<fx_graph::animation_source::Session>,graph:Option<Graph>}
type Set=core::Set<Handle,FxOwner>;
///A read-only identity into ONE completed source owner, never a raw resource
///or gameplay timing snapshot. Data lifetime and exact indices are rechecked.
pub struct AnimationHandle{data:Data,set:usize,index:usize}
pub struct Source{
    data:Data,layout:Layout,sets:Vec<Option<Set>>,blend_time:Option<u32>,graph_order:Vec<(usize,usize)>,
    definition:Option<RuntimeDefinition>,precision:Option<Precision>,universe:Vec<SourceIdentity>,
    failed:Option<String>,complete:bool,
}
impl std::fmt::Debug for Source{
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.debug_struct("ActorAnimationSource").field("sets",&self.sets.len()).field("complete",&self.complete).field("failed",&self.failed).finish()}
}
///Real42B8B0 services supplied by the canonical Data/Resource dispatcher.
///The empty-list branch below executes without a child call; positive FX
///must use their actual linked source owners and ordered request kernels.
pub trait FxRequests{
    fn request_list(&mut self,graph:&Graph,roots:&[NodeId],delta:u32,mode:u32,
        resources:&mut Resources<'_>,backend:&mut impl Backend)->Result<()>;
}
impl Source{
    pub(crate) fn from_native_constructor(data:&Data,layout:Layout)->Result<Self>{
        data.require_live()?;layout.require_data(data)?;
        Ok(Self{data:data.clone(),layout,sets:vec![],blend_time:None,graph_order:vec![],definition:None,precision:None,
            universe:vec![],failed:None,complete:false})
    }
    pub(crate) fn load(&mut self,catalog:&Catalog<'_>,bank:&Bank,definition:&RuntimeDefinition,
        resources:&mut Resources<'_>)->Result<()>{
        if let Some(error)=&self.failed{return Err(error.clone())}
        let expected=self.layout.definition();
        if expected.proof.id!=definition.proof.id||expected.proof.source!=definition.proof.source
            ||expected.proof.template_chain!=definition.proof.template_chain||expected.proof.category!=definition.proof.category
            ||expected.native_class!=definition.native_class||expected.flags!=definition.flags||expected.component_mask!=definition.component_mask{
            return Err("Actor Animation definition differs from actual Data/Aspect owner".into())
        }
        if self.complete{return self.require(catalog,bank)}
        let result=self.load_inner(catalog,bank,definition,resources);
        if let Err(error)=&result{self.failed=Some(error.clone());}result
    }
    fn require(&self,catalog:&Catalog<'_>,bank:&Bank)->Result<()>{
        self.data.require_live()?;self.layout.require_data(&self.data)?;
        let definition=self.definition.as_ref().ok_or("Actor Animation canonical source unbound")?;
        let source=crate::definition_fx::Source::bind_in_bank(catalog,bank,definition)?;
        if self.precision!=Some(source.source_precision())||self.universe!=catalog.actors.sources{return Err("Actor Animation source universe/PC changed".into())}
        Ok(())
    }
    fn load_inner(&mut self,catalog:&Catalog<'_>,bank:&Bank,definition:&RuntimeDefinition,resources:&mut Resources<'_>)->Result<()>{
        self.data.require_live()?;self.layout.require_data(&self.data)?;
        if !matches!(definition.native_class,3|4){return Err("Actor Animation requires actual ActorData class".into())}
        let proof=crate::definition_fx::Source::bind_in_bank(catalog,bank,definition)?;
        let pc=proof.source_precision();self.definition=Some(definition.clone());self.precision=Some(pc);self.universe=catalog.actors.sources.clone();
        let actor=catalog.actors.resolved_actor(&definition.proof.id).map_err(error)?;
        if actor.sources()!=definition.proof.template_chain{return Err("Actor Animation registered source layers differ".into())}
        let Some(character)=actor.child("CharacterComponent").map_err(error)? else{self.complete=true;return Ok(())};
        //465125 precedes the1BC resize and base-set allocation. This source
        //Float is retained only in this selected ActorData field projection.
        self.blend_time=Some(number(&character,"animation_blend_time",pc,ScalarType::Float)?);
        self.sets.resize_with(self.layout.names().len()+1,||None);
        let base=self.layout.names().len();self.sets[base]=Some(Set::constructed());
        let view=actor.child("View").map_err(error)?.ok_or("Actor Animation requires actual ObjectData View prefix")?;
        let prefix=text(&view,"resource_prefix",pc)?;
        let normalized=definition.proof.source.path.replace('\\',"/");let folder=normalized.rsplit_once('/').map_or("",|(d,_)|d);
        let base_nodes=character.children("Animation").map_err(error)?;
        for node in &base_nodes{
            let set=self.sets[base].as_mut().unwrap();let before=set.rows.len();
            set.load(&prefix,false,&mut Current::new(catalog,bank,definition,node,folder,resources,pc),pc)?;
            if set.rows.len()!=before{self.graph_order.push((base,before));}
        }
        for scope in character.children("AspectAnimationSet").map_err(error)?{
            let ids=text(&scope,"aspect_ids",pc)?;
            let index=self.layout.names().iter().position(|v|v.eq_ignore_ascii_case(&ids)).ok_or("410C40 unknown Aspect for Actor Animation set")?;
            if self.sets[index].is_some(){return Err("410C40 duplicate Actor Animation aspect set".into())}
            self.sets[index]=Some(Set::constructed());
            let set=self.sets[index].as_mut().unwrap();
            for node in scope.children("Animation").map_err(error)?{
                let before=set.rows.len();set.load(&prefix,false,&mut Current::new(catalog,bank,definition,&node,folder,resources,pc),pc)?;
                if set.rows.len()!=before{self.graph_order.push((index,before));}
            }
            //463F10 reparses the SAME base source with inherit=true. It does
            //not clone completed base Animation or FX allocations.
            for node in &base_nodes{
                let before=set.rows.len();set.load(&prefix,true,&mut Current::new(catalog,bank,definition,node,folder,resources,pc),pc)?;
                if set.rows.len()!=before{self.graph_order.push((index,before));}
            }
        }
        for set in self.sets.iter_mut().flatten(){set.finalize(pc)?;}
        self.complete=true;Ok(())
    }
    pub(crate) fn set_count(&self)->Result<usize>{self.ready()?;Ok(self.sets.len())}
    pub(crate) fn set_len(&self,set:usize)->Result<Option<usize>>{
        self.ready()?;self.sets.get(set).map(|set|set.as_ref().map(|v|v.rows.len())).ok_or_else(||"Actor Animation set outside actual1BC".into())
    }
    pub(crate) fn animation(&self,set:usize,index:usize)->Result<AnimationHandle>{
        self.ready()?;self.sets.get(set).and_then(Option::as_ref).and_then(|s|s.rows.get(index)).ok_or("Actor Animation allocation index absent")?;
        Ok(AnimationHandle{data:self.data.clone(),set,index})
    }
    fn ready(&self)->Result<()>{self.data.require_live()?;if !self.complete{return Err(self.failed.clone().unwrap_or_else(||"Actor Animation source is unfinished".into()))}Ok(())}
    ///Borrow each SAME graph once in actual compilation order (base first,
    ///then each explicit/fallback set). This metadata loan does not execute
    ///or certify a source preparation phase, Resource request or decoder.
    pub(crate) fn visit_graphs(&self,catalog:&Catalog<'_>,bank:&Bank,mut visit:impl FnMut(&Graph)->Result<()>)->Result<()>{
        self.ready()?;self.require(catalog,bank)?;
        for &(set,index)in &self.graph_order{
            let graph=self.sets[set].as_ref().and_then(|s|s.rows.get(index)).and_then(|a|a.fx.as_ref()).and_then(|f|f.graph.as_ref()).ok_or("Actor Animation compiled Graph owner absent")?;
            graph.require_in_bank(catalog,bank)?;visit(graph)?;
        }
        Ok(())
    }
    pub(crate) fn request_animation(&self,catalog:&Catalog<'_>,bank:&Bank,handle:&AnimationHandle,delta:u32,
        resources:&mut Resources<'_>,backend:&mut impl Backend,fx:&mut impl FxRequests)->Result<()>{
        self.ready()?;self.require(catalog,bank)?;
        if !self.data.same_allocation(&handle.data){return Err("Animation handle belongs to another Data".into())}
        let animation=self.sets.get(handle.set).and_then(Option::as_ref).and_then(|set|set.rows.get(handle.index)).ok_or("Animation handle outside retained source owner")?;
        animation.request(delta,|call,delta,mode|match call{
            core::Request::Resource(resource)=>resources.request_0c(resource,delta as i32,mode,backend),
            core::Request::Fx(owner,key)=>{
                let graph=owner.graph.as_ref().ok_or("Actor Animation contextual Graph is unfinished")?;
                graph.require_in_bank(catalog,bank)?;let roots=graph.animation_list_roots(key)?;
                if roots.is_empty(){Ok(())}else{fx.request_list(graph,roots,delta,mode,resources,backend)}
            }
        })
    }
}
struct Current<'a,'source,'node,'resource>{catalog:&'a Catalog<'source>,bank:&'a Bank,definition:&'a RuntimeDefinition,node:&'a ResolvedNode<'node>,folder:&'a str,resources:&'a mut Resources<'resource>,pc:Precision,cues:Option<Vec<ResolvedNode<'node>>>}
impl<'a,'source,'node,'resource> Current<'a,'source,'node,'resource>{
    fn new(catalog:&'a Catalog<'source>,bank:&'a Bank,definition:&'a RuntimeDefinition,node:&'a ResolvedNode<'node>,folder:&'a str,resources:&'a mut Resources<'resource>,pc:Precision)->Self{Self{catalog,bank,definition,node,folder,resources,pc,cues:None}}
}
impl core::Source<Handle,FxOwner> for Current<'_,'_,'_,'_>{
    fn string(&mut self,key:&str)->Result<String>{text(self.node,key,self.pc)}
    fn resource(&mut self,expanded:&str)->Result<Option<Handle>>{let path=crate::data_pre_request::path::relative(self.folder,expanded)?;self.resources.sequence_5a80d0(self.catalog,self.bank,&path.encode_utf16().collect::<Vec<_>>()).map(Some)}
    fn fixed(&mut self,key:&str)->Result<u32>{number(self.node,key,self.pc,ScalarType::Fixed)}
    fn boolean(&mut self,key:&str)->Result<u8>{self.node.field(key,self.pc).map_err(error)?.ok_or("Actor Animation Bool source absent")?.boolean().map(|v|v as u8).map_err(error)}
    fn integer(&mut self,key:&str)->Result<i32>{match self.node.field(key,self.pc).map_err(error)?{Some(TypedValue::Integer{value,..})=>Ok(value),_=>Err("Actor Animation requires native Int field".into())}}
    fn resolve_kind(&mut self,value:&str)->Result<i32>{
        let kind=core::registered_kind(value);if kind>=0{Ok(kind)}else{Err("497020 requires actual additional Animation alias registration or unknown-kind diagnostic boundary".into())}
    }
    fn validate_damage_filter(&mut self,_:i32)->Result<()>{Err("461F70 positive death damage_filter requires actual DamageFilterData+2C registry".into())}
    fn begin_fx(&mut self)->Result<FxOwner>{Ok(FxOwner{session:Some(fx_graph::animation_source::Session::new(self.catalog,self.bank,self.definition,self.node)?),graph:None})}
    fn append_fx(&mut self,context:&mut FxOwner,list:core::FxList)->Result<usize>{
        let session=context.session.as_mut().ok_or("Actor Animation FX compiler was already moved")?;
        match list{
            core::FxList::Standard(index)=>session.append(self.catalog,self.bank,self.node,["WhenStart","WhenDo","WhenEnd"][index]),
            core::FxList::Timed(index)=>session.append_timed(self.catalog,self.bank,self.cues.as_ref().and_then(|c|c.get(index)).ok_or("Actor Animation current timed source node absent")?),
        }
    }
    fn permanent(&mut self,context:&FxOwner,key:usize)->Result<bool>{context.session.as_ref().ok_or("Actor Animation source compiler absent")?.permanent(self.catalog,self.bank,key)}
    fn timed_count(&mut self)->Result<usize>{self.cues=Some(self.node.children("WhenDoAtTime").map_err(error)?);Ok(self.cues.as_ref().unwrap().len())}
    fn timed_time(&mut self,index:usize)->Result<u32>{number(self.cues.as_ref().and_then(|c|c.get(index)).ok_or("Actor Animation timed source node absent")?,"time",self.pc,ScalarType::Fixed)}
    fn finish_fx(&mut self,context:&mut FxOwner)->Result<()>{let graph=context.session.as_mut().ok_or("Actor Animation FX source absent")?.finish(self.catalog,self.bank)?;context.graph=Some(graph);context.session=None;Ok(())}
}
fn error(e:impl std::fmt::Display)->String{e.to_string()}
fn text(node:&ResolvedNode<'_>,name:&str,pc:Precision)->Result<String>{match node.field(name,pc).map_err(error)?{Some(TypedValue::String{value,..})if value.is_ascii()&&!value.contains('\0')=>Ok(value),_=>Err(format!("Actor Animation {name} requires native plain ASCII String"))}}
fn number(node:&ResolvedNode<'_>,name:&str,pc:Precision,kind:ScalarType)->Result<u32>{match node.field(name,pc).map_err(error)?{Some(TypedValue::Scalar(v))if v.field_type==kind&&f32::from_bits(v.bits).is_finite()=>Ok(v.bits),_=>Err(format!("Actor Animation {name} requires native finite {kind:?} field"))}}

