//!Deliberate modern graphics adaptation of73A560, not D3D9 shader parity.
//!Native model filename selection still reads the actual current UVar113.
use crate::battle_ui_bridge::Client;
use aa_runtime::{resource_objects::ModelPolicy,simulation_execution::Execution};
use std::sync::{Arc,Weak,atomic::{AtomicU64,Ordering}};
#[path="view_model_policy_kernel.rs"]mod kernel;
type Result<T>=std::result::Result<T,String>;

///Retained by the actual SceneRenderer after its device/shader initialization
///succeeds. DX12/Vulkan do not run the legacy D3D9 GeForce3/4 workaround.
///The actual backend renders pre-baked vertex frames, with no bone-index/
///weight lit/unlit shader pair. Neither fact is a projected native NULL/byte.
#[derive(Clone,Copy)]enum Compatibility{ModernDx12,ModernVulkan}
impl Compatibility{
    fn completed_for(adapter:&wgpu::AdapterInfo)->Result<Self>{match adapter.backend{
        wgpu::Backend::Dx12=>Ok(Self::ModernDx12),wgpu::Backend::Vulkan=>Ok(Self::ModernVulkan),
        _=>Err("modern model compatibility mapping requires the actual DX12/Vulkan backend".into()),
    }}
    fn problematic_275(self)->u8{match self{Self::ModernDx12|Self::ModernVulkan=>0}}
}
pub(crate) struct BackendState{adapter:wgpu::AdapterInfo,compatibility:Compatibility}
impl BackendState{
    pub(crate) fn from_actual_adapter(adapter:wgpu::AdapterInfo)->Result<Self>{
        let compatibility=Compatibility::completed_for(&adapter)?;
        if adapter.name.is_empty(){return Err("actual model renderer adapter description is empty".into())}
        Ok(Self{adapter,compatibility})
    }
    pub(crate) fn capability(&self,owner:&Arc<AtomicU64>)->Capabilities{
        Capabilities{owner:Arc::downgrade(owner),epoch:owner.load(Ordering::Acquire),compatibility:self.compatibility}
    }
}
///No raw/default constructor. This nonowning packet expires on renderer drop
///or scene/pipeline epoch change, including before the first scene upload.
pub struct Capabilities{owner:Weak<AtomicU64>,epoch:u64,compatibility:Compatibility}
impl Capabilities{
    fn require_current(&self)->Result<()>{
        let current=self.owner.upgrade().ok_or("model renderer allocation is destroyed")?;
        if current.load(Ordering::Acquire)!=self.epoch{return Err("model renderer pipeline epoch changed".into())}Ok(())
    }
    pub fn use_hardware_model_current(&self,variables:&aa_ui::uvars::UVars)->Result<bool>{
        self.require_current()?;
        let mut fields=Current{capability:self,variables};
        let result=kernel::use_hardware_model(&mut fields)?;
        self.require_current()?;Ok(result)
    }
}
struct Current<'a>{capability:&'a Capabilities,variables:&'a aa_ui::uvars::UVars}
impl kernel::Fields for Current<'_>{
    //5A6FEF reads B40690,739D10's actual skinned-family publication. This
    //selects the model family, not proof that a GPU bone shader exists.
    fn receiver_191(&mut self)->Result<u8>{Ok(1)}
    fn problematic_driver_275(&mut self)->Result<u8>{
        self.capability.require_current()?;
        Ok(self.capability.compatibility.problematic_275())
    }
    fn uvar_113(&mut self)->Result<u8>{self.variables.renderer_use_hw_skinning_shader()}
    //The current actual GPU implementation owns only static/baked-frame
    //deformation pipelines. Adding bone skinning must supply both actual
    //shader owners and advance the sole renderer epoch before changing this.
    fn lit_bone_shader_128(&mut self)->Result<bool>{Ok(false)}
    fn unlit_bone_shader_12c(&mut self)->Result<bool>{Ok(false)}
}

///Each Model5A8520 candidate boundary calls the SAME actual UI/backend owner.
///There is no cached false result or pre-sampled UVar replacing a current read.
pub struct UiPolicy<'a,'run>{client:&'a Client,execution:&'a mut Execution<'run>}
impl<'a,'run> UiPolicy<'a,'run>{pub fn new(client:&'a Client,execution:&'a mut Execution<'run>)->Self{Self{client,execution}}}
impl ModelPolicy for UiPolicy<'_,'_>{
    fn use_hardware_model_73a560(&mut self)->Result<bool>{
        self.client.call(self.execution,|context|{
            context.renderer.model_variant_capabilities().use_hardware_model_current(&context.ui.uvars)
        })
    }
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn same_renderer_epoch_and_drop_invalidate_capability_even_at_zero(){
        let origin=Arc::new(AtomicU64::new(0));
        let cap=Capabilities{owner:Arc::downgrade(&origin),epoch:0,compatibility:Compatibility::ModernVulkan};
        cap.require_current().unwrap();origin.store(1,Ordering::Release);assert!(cap.require_current().is_err());
        let next=Capabilities{owner:Arc::downgrade(&origin),epoch:1,compatibility:Compatibility::ModernVulkan};
        next.require_current().unwrap();drop(origin);assert!(next.require_current().is_err());
        fn send<T:Send+Sync>(){}send::<Capabilities>();
    }
}
#[cfg(test)]#[path="view_model_policy_gpu_tests.rs"]mod gpu_tests;
