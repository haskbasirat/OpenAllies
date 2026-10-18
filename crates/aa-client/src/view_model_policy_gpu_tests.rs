use super::*;

#[test]
#[ignore="requires actual original/v14 UVars and a DX12/Vulkan adapter; no native engine/window"]
fn actual_gpu_model_capability_reads_current_uvar_and_expires_with_renderer(){
    use crate::gpu::SceneRenderer;
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let root=aa_ui::fs::DataRoot::with_overlay(game.join("reconstruction/assets/original/Data"),
        game.join("reconstruction/native-game/runtime-expanded-v14/Data"));
    let mut store=aa_ui::tgi::TgiStore::new(root);
    let mut variables=aa_ui::uvars::UVars::load_with_view_scope(&mut store,
        aa_ui::view_variables::SourceOverrides::ControlledNone);
    assert_eq!(variables.renderer_use_hw_skinning_shader().unwrap(),1);
    let instance=wgpu::Instance::new(&wgpu::InstanceDescriptor{backends:wgpu::Backends::DX12|wgpu::Backends::VULKAN,..Default::default()});
    let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let(device,queue)=pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor{required_limits:adapter.limits(),..Default::default()})).unwrap();
    let egui_renderer=egui_wgpu::Renderer::new(&device,wgpu::TextureFormat::Rgba8UnormSrgb,Default::default());
    let renderer=SceneRenderer::new(egui_wgpu::RenderState{adapter,available_adapters:Vec::new(),device,queue,
        target_format:wgpu::TextureFormat::Rgba8UnormSrgb,
        renderer:Arc::new(eframe::egui::mutex::RwLock::new(egui_renderer))}).unwrap();
    let capability=renderer.model_variant_capabilities();
    //This actual backend owns no bone-skinning shader pair. The true current
    //UVar is still read before reaching that absence; an unbound UVar fails.
    assert!(!capability.use_hardware_model_current(&variables).unwrap());
    let unbound=aa_ui::uvars::UVars::load_with_view_scope(&mut store,aa_ui::view_variables::SourceOverrides::Unbound);
    assert!(capability.use_hardware_model_current(&unbound).is_err());
    variables.assign_view_bool("RendererUseHWSkinningShader",128,None).unwrap();
    assert_eq!(variables.renderer_use_hw_skinning_shader().unwrap(),128);
    assert!(!capability.use_hardware_model_current(&variables).unwrap());
    variables.assign_view_bool("RendererUseHWSkinningShader",0,None).unwrap();
    assert!(!capability.use_hardware_model_current(&variables).unwrap());
    println!("actual current model capability: {}; native-equivalent bone shader pair absent; source113/current writes read",renderer.adapter_name);
    drop(renderer);assert!(capability.use_hardware_model_current(&variables).is_err());
}
