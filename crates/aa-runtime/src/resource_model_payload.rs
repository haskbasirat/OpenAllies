//!Same selected NIF bytes and a retained modern model object. This boundary
//!does not manufacture Resource flags, native reference counts or GPU readiness.
use crate::resource_objects::{Handle,SourceFile};
use std::sync::{Arc,Mutex};
type Result<T>=std::result::Result<T,String>;

///The field projection returned by87AA10. A Resource obtains this only from
///the actual payload operation; constructor values are not loaded metadata.
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Metadata{
    pub byte1:u8,pub word4:u32,pub radial8:u32,pub max_z0c:u32,pub min_z10:u32,
    pub footprint14:u32,pub footprint18:u32,pub footprint1c:u32,
    pub texture_count20:u32,pub byte24:u8,
}
///Current graphics policy is read at the reached native graph branch. A model
///cannot retain a copied UI byte or borrow an Execution across async work.
pub trait MetadataPolicy{
    fn renderer_remove_normals_current(&mut self)->Result<u8>{Err("87A1E5 actual current indexed UVar52 RendererRemoveNormals is required".into())}
    fn preprocessed_data_missing_warning_current(&mut self)->Result<u8>{Err("878F30 actual current named AVar PreprocessedDataMissingWarning is required".into())}
    fn preprocessed_data_wrong_version_warning_current(&mut self)->Result<u8>{Err("878F30 actual current named AVar PreprocessedDataWrongVersionWarning is required".into())}
    fn preprocessed_data_warning(&mut self,_filename:&[u16],_old_version:Option<u32>)->Result<()>{Err("878F30 positive preprocessing diagnostic sink is required".into())}
    fn graph_diagnostic(&mut self,_filename:&[u16],_node:&str,_line:u32)->Result<()>{Err("879A90 reached graph diagnostic service is required".into())}
    ///87A1B7 405D20(B39938) of the source-created named flag WarningsCheckPartitions.
    fn warnings_check_partitions_current(&mut self)->Result<u8>{Err("87A1B7 actual current named WarningsCheckPartitions is required".into())}
}
struct UnboundPolicy;impl MetadataPolicy for UnboundPolicy{}
///5A94F0's two outcomes as reached in the SAME graph after its Update(0,0).
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Selection{
    ///5A96B2: the NiGeometry selection node's transformed vertex extents.
    Geometry([u32;6]),
    ///5A95CF: no NiGeometry of that name; the root world bound (center, radius).
    RootSphere([u32;4]),
}
///Implemented by the actual decoded source/graph backend. Positive operations
///must alter or inspect that same object; defaults mark reached missing children.
pub trait Model:std::fmt::Debug+Send+Sync{
    fn source_sha256(&self)->&str;
    ///Readonly backend-specific loan of THIS same decoded object. Runtime
    ///does not depend on a renderer's geometry types or accept a ready flag.
    fn backend_data(&self)->Result<&(dyn std::any::Any+Send+Sync)>{Err("selected model has no concrete backend representation".into())}
    fn process_metadata(&mut self,has_skin:bool)->Result<Metadata>{self.process_metadata_with_policy(has_skin,&mut UnboundPolicy)}
    fn process_metadata_with_policy(&mut self,_has_skin:bool,_policy:&mut dyn MetadataPolicy)->Result<Metadata>{Err("87AA10 selected NIF processing is required".into())}
    fn texture_names(&self)->Result<Vec<Vec<u16>>>{Err("8784F0/8873C0 ordered texture names are required".into())}
    fn hardware_transform(&mut self,_count:u32)->Result<()>{Err("5A65B0 hardware graph transform with current AVar51 count is required".into())}
    fn prepare_effect(&mut self,_code:u32)->Result<()>{Err("871BC0/871B60/871B40 actual graph preparation is required".into())}
    fn controllers(&self)->Result<bool>{Err("5A9840 actual first-Bone texturing descriptor-slot3 query is required".into())}
    fn selection_cuboid(&mut self)->Result<[u32;6]>{Err("5A94F0 actual selection cuboid is required".into())}
    ///5A94F0 vf40 GetObjectByName(Bone selection_box) + RTTI NiGeometry (B43CF0)
    ///test, then vf48 Update(0,0): either the selection geometry's cuboid (the
    ///node is then unlinked by 6FF850) or the root's world bound for the
    ///BoundingSphere fallback. The default keeps backends that only port the
    ///geometry branch.
    fn selection_5a94f0(&mut self)->Result<Selection>{self.selection_cuboid().map(Selection::Geometry)}
    fn wrap_root(&mut self)->Result<()>{Err("867690 actual model root wrapper is required".into())}
    fn update_root(&mut self)->Result<()>{Err("86E8D0 actual model update is required".into())}
    fn apply_named_property(&mut self,_name:&[u16])->Result<()>{Err("5EDE60/4D1E10 actual named model property is required".into())}
}

///Minted only by the same Resource manager's selected-file read.
pub struct Input{handle:Handle,source:SourceFile,bytes:Vec<u8>,sha256:String,context:Option<crate::resource_model_source::Context>}
impl Input{
    pub(crate) fn new(handle:Handle,source:SourceFile,bytes:Vec<u8>,sha256:String,context:Option<crate::resource_model_source::Context>)->Self{Self{handle,source,bytes,sha256,context}}
    pub fn source(&self)->&SourceFile{&self.source}
    ///The SAME Resource filename CString(+38), independently of selected
    ///physical-file spelling/case. Used by native model processing.
    pub fn resource_ids(&self)->&[u16]{self.handle.logical_ids()}
    pub fn sha256(&self)->&str{&self.sha256}
    pub fn source_context(&self)->Option<&crate::resource_model_source::Context>{self.context.as_ref()}
    pub fn prepare_with(&self,prepare:impl FnOnce(&[u8],&SourceFile)->Result<Box<dyn Model>>)->Result<Payload>{
        let owner=prepare(&self.bytes,&self.source)?;
        if owner.source_sha256()!=self.sha256{return Err("selected Model payload belongs to different source bytes".into())}
        Ok(Payload{handle:self.handle.clone(),source:self.source.clone(),sha256:self.sha256.clone(),owner:Arc::new(Mutex::new(owner))})
    }
}
///Cloning preserves ONE model object allocation. Arc is host ownership, not
///a claim that a native Ni reference or Resource reference operation ran.
#[derive(Clone)]
pub struct Payload{handle:Handle,source:SourceFile,sha256:String,owner:Arc<Mutex<Box<dyn Model>>>}
impl std::fmt::Debug for Payload{
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.debug_struct("ModelPayload").field("source",&self.source).field("sha256",&self.sha256).finish_non_exhaustive()}
}
impl Payload{
    pub fn require_resource(&self,handle:&Handle)->Result<()>{
        if !self.handle.same_identity(handle)||handle.source_file()!=Some(&self.source){return Err("Model payload belongs to another selected Resource allocation".into())}Ok(())
    }
    pub fn same_identity(&self,other:&Self)->bool{Arc::ptr_eq(&self.owner,&other.owner)&&self.handle.same_identity(&other.handle)}
    pub fn source_sha256(&self)->&str{&self.sha256}
    pub fn source(&self)->&SourceFile{&self.source}
    fn with<T>(&self,f:impl FnOnce(&mut dyn Model)->Result<T>)->Result<T>{
        let mut owner=self.owner.lock().map_err(|_|"selected Model payload lock is poisoned")?;f(owner.as_mut())
    }
    ///The callback runs while the sole payload is immutably lent. It may
    ///produce a render-only packet, never another Model/Resource authority.
    pub fn with_backend_data<T:std::any::Any+Send+Sync,R>(&self,read:impl FnOnce(&T)->Result<R>)->Result<R>{
        self.with(|owner|read(owner.backend_data()?.downcast_ref::<T>().ok_or("selected model backend type differs")?))
    }
    pub fn process_metadata(&self,unique:bool)->Result<Metadata>{self.with(|o|o.process_metadata(unique))}
    pub fn process_metadata_with_policy(&self,has_skin:bool,policy:&mut dyn MetadataPolicy)->Result<Metadata>{self.with(|o|o.process_metadata_with_policy(has_skin,policy))}
    pub fn texture_names(&self)->Result<Vec<Vec<u16>>>{self.with(|o|o.texture_names())}
    pub fn hardware_transform(&self,count:u32)->Result<()>{self.with(|o|o.hardware_transform(count))}
    pub fn prepare_effect(&self,code:u32)->Result<()>{self.with(|o|o.prepare_effect(code))}
    pub fn controllers(&self)->Result<bool>{self.with(|o|o.controllers())}
    pub fn selection_cuboid(&self)->Result<[u32;6]>{self.with(|o|o.selection_cuboid())}
    pub fn selection_5a94f0(&self)->Result<Selection>{self.with(|o|o.selection_5a94f0())}
    pub fn wrap_root(&self)->Result<()>{self.with(|o|o.wrap_root())}
    pub fn update_root(&self)->Result<()>{self.with(|o|o.update_root())}
    pub fn apply_named_property(&self,name:&[u16])->Result<()>{self.with(|o|o.apply_named_property(name))}
}
