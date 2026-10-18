//!Actual modern renderer device/target references for native camera ownership.
//!These references certify neither a rendered frame nor initialized VWorld.
use super::{SceneRenderer,DEPTH};
use anyhow::{Context,Result,ensure};
use std::sync::{Arc,Weak,atomic::{AtomicU64,Ordering}};

///A retained handle to the SAME wgpu device allocation. Cloning wgpu's device
///handle does not create a device or renderer; the weak owner still expires
///when the actual SceneRenderer is destroyed. Scene uploads do not replace it.
#[derive(Clone)]
pub struct Reference{_device:wgpu::Device,renderer:Weak<AtomicU64>,viewport:Option<Arc<Viewport>>}
impl Reference{
    pub fn require_alive(&self)->Result<()>{ensure!(self.renderer.upgrade().is_some(),"camera's actual renderer allocation was destroyed");Ok(())}
    pub fn is_for(&self,renderer:&SceneRenderer)->bool{
        self.renderer.upgrade().is_some()&&self.renderer.ptr_eq(&Arc::downgrade(&renderer.scene_epoch))
    }
    pub fn viewport_pixels(&self)->Result<[u32;2]>{
        self.require_current()?;Ok(self.viewport.as_ref().context("camera device had no actual allocated viewport at the reached read")?.pixels())
    }
    pub fn require_current(&self)->Result<()>{
        self.require_alive()?;
        self.viewport.as_ref().context("camera device had no actual allocated viewport at the reached read")?.require_current()
    }
}
///Deliberate modern counterpart of710433 after real adapter/device/pipeline
///initialization. This is neither native7101A0 parity nor texture GPU readiness.
pub struct Initialization{device:Reference,epoch:u64}
impl Initialization{
    pub fn require_current(&self)->Result<()>{
        self.device.require_alive()?;
        let owner=self.device.renderer.upgrade().context("initialized renderer allocation was destroyed")?;
        ensure!(owner.load(Ordering::Acquire)==self.epoch,"initialized renderer pipeline epoch changed");Ok(())
    }
    pub fn require_for(&self,renderer:&SceneRenderer)->Result<()>{
        self.require_current()?;ensure!(self.device.is_for(renderer),"renderer initialization belongs to another allocation");Ok(())
    }
}
impl aa_runtime::resource_texture::RendererInitialization for Initialization{
    fn require_current_backend(&self)->std::result::Result<(),String>{self.require_current().map_err(|e|e.to_string())}
}
///Actual allocated render-target dimensions, correlated with the same renderer
///pipeline epoch. Target replacement drops its private identity, so resizing
///cannot leave a prior camera projection looking current.
pub struct Viewport{renderer:Weak<AtomicU64>,target:Weak<()>,epoch:u64,pixels:[u32;2]}
impl Viewport{
    pub fn pixels(&self)->[u32;2]{self.pixels}
    pub fn require_current(&self)->Result<()>{
        let renderer=self.renderer.upgrade().context("camera viewport renderer was destroyed")?;
        ensure!(renderer.load(Ordering::Acquire)==self.epoch&&self.target.upgrade().is_some(),
            "camera viewport target/pipeline changed; actual projection update is required");Ok(())
    }
    pub fn require_for(&self,renderer:&SceneRenderer)->Result<()>{
        self.require_current()?;
        ensure!(self.renderer.ptr_eq(&Arc::downgrade(&renderer.scene_epoch))
            &&renderer.target.as_ref().is_some_and(|v|self.target.ptr_eq(&Arc::downgrade(&v.identity))&&v.size==self.pixels),
            "camera viewport belongs to another renderer/target allocation");Ok(())
    }
}
impl aa_runtime::native_camera::backend::Viewport for Viewport{
    fn pixels(&self)->[u32;2]{self.pixels()}
    fn require_current(&self)->std::result::Result<(),String>{self.require_current().map_err(|e|e.to_string())}
}
impl aa_runtime::native_camera::backend::ViewportFor<Reference> for Viewport{
    fn require_device(&self,device:&Reference)->std::result::Result<(),String>{
        device.require_alive().map_err(|e|e.to_string())?;
        self.require_current().map_err(|e|e.to_string())?;
        if !self.renderer.ptr_eq(&device.renderer){return Err("camera viewport and retained device belong to different renderer allocations".into())}Ok(())
    }
}
impl SceneRenderer{
    pub fn renderer_initialization(&self)->Initialization{
        Initialization{device:self.camera_device_reference(),epoch:self.scene_epoch()}
    }
    ///Read-only loans of the actual initialized GPU owners for concrete
    ///texture backend creation/upload; neither loan grants any readiness.
    pub(crate) fn device_and_queue(&self)->(&wgpu::Device,&wgpu::Queue){(&self.state.device,&self.state.queue)}
    pub fn camera_device_reference(&self)->Reference{
        let viewport=self.target.as_ref().map(|target|Arc::new(Viewport{renderer:Arc::downgrade(&self.scene_epoch),
            target:Arc::downgrade(&target.identity),epoch:self.scene_epoch(),pixels:target.size}));
        Reference{_device:self.state.device.clone(),renderer:Arc::downgrade(&self.scene_epoch),viewport}
    }
    ///Creates the real color/depth target using the current device, without
    ///drawing or publishing camera/VWorld readiness. No dimension clamping is
    ///permitted on this native-camera path.
    pub fn prepare_camera_viewport(&mut self,pixels:[u32;2])->Result<Viewport>{
        let maximum=self.state.device.limits().max_texture_dimension_2d;
        ensure!(!pixels.contains(&0)&&pixels.iter().all(|v|*v<=maximum),"camera viewport exceeds actual GPU dimensions");
        self.resize(pixels)?;
        let target=self.target.as_ref().context("actual camera viewport allocation missing")?;
        ensure!(target.size==pixels,"actual camera viewport differs from requested dimensions");
        Ok(Viewport{renderer:Arc::downgrade(&self.scene_epoch),target:Arc::downgrade(&target.identity),epoch:self.scene_epoch(),pixels})
    }
    ///Deliberate modern counterpart of native Renderer24C's no-stencil branch.
    ///The actual pipeline/attachment format is Depth32Float. This does not
    ///claim that positive native stencil effects are implemented by this GPU.
    pub fn camera_stencil_word_24c(&self)->Result<u32>{
        ensure!(DEPTH==wgpu::TextureFormat::Depth32Float,"camera stencil projection needs the actual changed depth-format mapping");Ok(0)
    }
}
impl aa_runtime::native_camera::directory::RendererSource for SceneRenderer{
    type Reference=Reference;
    fn current_device_40(&mut self)->std::result::Result<Reference,String>{Ok(self.camera_device_reference())}
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn viewport_expires_on_same_renderer_epoch_or_target_replacement(){
        let renderer=Arc::new(AtomicU64::new(0));let target=Arc::new(());
        let viewport=Viewport{renderer:Arc::downgrade(&renderer),target:Arc::downgrade(&target),epoch:0,pixels:[800,600]};
        viewport.require_current().unwrap();drop(target);assert!(viewport.require_current().is_err());
        let target=Arc::new(());let viewport=Viewport{renderer:Arc::downgrade(&renderer),target:Arc::downgrade(&target),epoch:0,pixels:[800,600]};
        renderer.store(1,Ordering::Release);assert!(viewport.require_current().is_err());drop(renderer);assert!(viewport.require_current().is_err());
        fn send<T:Send>(){}send::<Reference>();send::<Viewport>();
    }
    #[test]
    #[ignore="requires a real DX12/Vulkan adapter; no native engine or window"]
    fn actual_gpu_device_viewport_no_stencil_are_correlated(){
        let instance=wgpu::Instance::new(&wgpu::InstanceDescriptor{backends:wgpu::Backends::DX12|wgpu::Backends::VULKAN,..Default::default()});
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let(device,queue)=pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor{required_limits:adapter.limits(),..Default::default()})).unwrap();
        let egui_renderer=egui_wgpu::Renderer::new(&device,wgpu::TextureFormat::Rgba8UnormSrgb,Default::default());
        let state=egui_wgpu::RenderState{adapter,available_adapters:vec![],device,queue,target_format:wgpu::TextureFormat::Rgba8UnormSrgb,
            renderer:Arc::new(eframe::egui::mutex::RwLock::new(egui_renderer))};
        let mut renderer=SceneRenderer::new(state.clone()).unwrap();let foreign=SceneRenderer::new(state).unwrap();
        let device=renderer.camera_device_reference();device.require_alive().unwrap();assert!(device.viewport_pixels().is_err());
        let initialization=renderer.renderer_initialization();initialization.require_for(&renderer).unwrap();assert!(initialization.require_for(&foreign).is_err());
        assert!(device.is_for(&renderer));assert!(!device.is_for(&foreign));assert_eq!(renderer.camera_stencil_word_24c().unwrap(),0);
        assert!(renderer.prepare_camera_viewport([0,480]).is_err());
        let viewport=renderer.prepare_camera_viewport([640,480]).unwrap();assert_eq!(viewport.pixels(),[640,480]);
        viewport.require_for(&renderer).unwrap();assert!(viewport.require_for(&foreign).is_err());
        aa_runtime::native_camera::backend::ViewportFor::require_device(&viewport,&device).unwrap();
        assert!(aa_runtime::native_camera::backend::ViewportFor::require_device(&viewport,&foreign.camera_device_reference()).is_err());
        let current_device=renderer.camera_device_reference();assert_eq!(current_device.viewport_pixels().unwrap(),[640,480]);
        let replacement=renderer.prepare_camera_viewport([800,600]).unwrap();replacement.require_for(&renderer).unwrap();
        assert!(viewport.require_current().is_err());assert!(current_device.require_current().is_err());
        drop(renderer);assert!(device.require_alive().is_err());assert!(replacement.require_current().is_err());assert!(initialization.require_current().is_err());
    }
}
