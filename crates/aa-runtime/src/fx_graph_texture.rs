//!Actual source-preparation loan of the same ResourceManager and TexturePolicy.
//!The contextual compiler retains Resource handles in its existing Node arena.
use super::*;
use crate::{definitions::Bank,resource_depot::Resources,resource_objects::{Handle,TexturePolicy}};

pub(super) trait Textures {
    fn find(&mut self, ids:&str)->Result<Option<Handle>>;
    fn radar(&mut self, texture:&Handle)->Result<()>;
}
struct Current<'a,'source,'manager,T>{
    catalog:&'a Catalog<'source>,bank:&'a Bank,resources:&'a mut Resources<'manager>,policy:&'a mut T,folder:String,
}
impl<T:TexturePolicy> Textures for Current<'_,'_,'_,T>{
    fn find(&mut self,ids:&str)->Result<Option<Handle>>{
        let path=crate::data_pre_request::path::relative(&self.folder,ids)?;
        self.resources.texture_5a77e0_with_policy(self.catalog,self.bank,&path.encode_utf16().collect::<Vec<_>>(),0,self.policy)
    }
    fn radar(&mut self,texture:&Handle)->Result<()>{self.resources.radar_texture_policy_42cd60(texture,self.policy)}
}
pub(super) fn loan<'a,T:TexturePolicy+'a>(catalog:&'a Catalog<'_>,bank:&'a Bank,definition:&RuntimeDefinition,
    resources:&'a mut Resources<'_>,policy:&'a mut T)->Box<dyn Textures+'a>{
    //4763C0 uses the current GameData source-file directory, not the
    //inherited FX node's declaration or the global resource biome folder.
    let normalized=definition.proof.source.path.replace('\\',"/");
    let folder=normalized.rsplit_once('/').map_or("",|(folder,_)|folder).to_owned();
    Box::new(Current{catalog,bank,resources,policy,folder})
}

impl Graph{
    ///Only this actual Resource-aware producer can certify the complete View
    ///arena. The public source-only/request APIs retain their existing guard.
    pub(crate) fn compile_complete_resources(catalog:&Catalog<'_>,bank:&Bank,definition:&RuntimeDefinition,kind:ListKind,
        resources:&mut Resources<'_>,policy:&mut impl TexturePolicy)->std::result::Result<Self,Failure>{
        let source=ObjectFxSource::bind_in_bank(catalog,bank,definition).map_err(|error|Failure{error,partial:Draft::new()})?;
        let textures=loan(catalog,bank,definition,resources,policy);
        Self::compile_bound_textures(catalog,kind,source,false,Some(textures))
    }
    ///The same six-list source compiler with real Texture resources. This is
    ///still request-only source preparation; it does not execute FX commands.
    pub(crate) fn compile_request_resources(catalog:&Catalog<'_>,bank:&Bank,definition:&RuntimeDefinition,kind:ListKind,
        resources:&mut Resources<'_>,policy:&mut impl TexturePolicy)->std::result::Result<Self,Failure>{
        let source=ObjectFxSource::bind_in_bank(catalog,bank,definition).map_err(|error|Failure{error,partial:Draft::new()})?;
        let textures=loan(catalog,bank,definition,resources,policy);
        Self::compile_bound_textures(catalog,kind,source,true,Some(textures))
    }
}
///Scoped read-only access to ONE actual Application DataFXSession. None means
///this definition has no prepared allocation; an attempted/failed producer
///returns its retained error. No resource lookup or count mutation occurs here.
mod sealed{pub trait Sealed{} impl Sealed for crate::resource_depot::Application{} impl Sealed for crate::resource_depot::SourceGraphsRead{}}
pub trait SourceGraphs:sealed::Sealed {
    fn prepared_view_graph(&self,catalog:&Catalog<'_>,bank:&Bank,definition:&RuntimeDefinition,kind:ListKind)
        ->Result<Option<std::sync::Arc<Graph>>>;
}
impl SourceGraphs for crate::resource_depot::Application{
    fn prepared_view_graph(&self,catalog:&Catalog<'_>,bank:&Bank,definition:&RuntimeDefinition,kind:ListKind)->Result<Option<std::sync::Arc<Graph>>>{
        self.with_data_pre_request(|state|state.prepared_view_graph(catalog,bank,definition,kind))
    }
}
impl SourceGraphs for crate::resource_depot::SourceGraphsRead{
    fn prepared_view_graph(&self,catalog:&Catalog<'_>,bank:&Bank,definition:&RuntimeDefinition,kind:ListKind)->Result<Option<std::sync::Arc<Graph>>>{
        self.with_state(|state|state.prepared_view_graph(catalog,bank,definition,kind))
    }
}
#[cfg(test)]
#[path="fx_graph_texture_tests.rs"]
mod tests;
