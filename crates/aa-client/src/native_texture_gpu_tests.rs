use super::*;
use aa_runtime::resource_texture::PixelLevel;
use std::sync::Arc;

#[test]
#[ignore="requires a real graphics adapter; creates no game process or window"]
fn actual_gpu_mip_readback_preserves_pixels_and_alpha() {
    let instance=wgpu::Instance::new(&wgpu::InstanceDescriptor{backends:wgpu::Backends::VULKAN|wgpu::Backends::DX12,..Default::default()});
    let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let(device,queue)=pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor{required_limits:adapter.limits(),..Default::default()})).unwrap();
    let egui_renderer=egui_wgpu::Renderer::new(&device,FORMAT,Default::default());
    let state=egui_wgpu::RenderState{adapter,available_adapters:vec![],device,queue,target_format:FORMAT,
        renderer:Arc::new(eframe::egui::mutex::RwLock::new(egui_renderer))};
    let renderer=SceneRenderer::new(state).unwrap();
    let pixels=PixelData::from_decoded_levels(vec![
        PixelLevel::rgba8(4,4,(0u8..64).collect()).unwrap(),
        PixelLevel::rgba8(2,2,vec![255,0,0,0,0,255,0,255,0,0,255,128,255,255,255,17]).unwrap(),
        PixelLevel::rgba8(1,1,vec![7,8,9,10]).unwrap(),
    ]).unwrap();
    let allocation=upload_allocation(&renderer,&pixels).unwrap();
    assert_eq!(allocation.dimensions,[4,4]);assert_eq!(allocation.mip_count,3);
    let(device,queue)=renderer.device_and_queue();
    for(mip,level)in pixels.levels().iter().enumerate(){
        let row=wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let scope=GpuScope::begin(device);
        let buffer=device.create_buffer(&wgpu::BufferDescriptor{label:Some("source mip readback"),size:u64::from(row)*u64::from(level.height()),usage:wgpu::BufferUsages::COPY_DST|wgpu::BufferUsages::MAP_READ,mapped_at_creation:false});
        let mut encoder=device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(wgpu::TexelCopyTextureInfo{texture:&allocation.texture,mip_level:mip as u32,origin:wgpu::Origin3d::ZERO,aspect:wgpu::TextureAspect::All},
            wgpu::TexelCopyBufferInfo{buffer:&buffer,layout:wgpu::TexelCopyBufferLayout{offset:0,bytes_per_row:Some(row),rows_per_image:Some(level.height())}},
            wgpu::Extent3d{width:level.width(),height:level.height(),depth_or_array_layers:1});
        queue.submit([encoder.finish()]);scope.finish("source mip readback").unwrap();
        let(send,receive)=std::sync::mpsc::sync_channel(1);
        buffer.slice(..).map_async(wgpu::MapMode::Read,move|r|{let _=send.send(r);});
        device.poll(wgpu::PollType::Wait{submission_index:None,timeout:Some(std::time::Duration::from_secs(30))}).unwrap();
        receive.recv_timeout(std::time::Duration::from_secs(1)).unwrap().unwrap();
        let mapped=buffer.slice(..).get_mapped_range();
        let tight:Vec<_>=mapped.chunks(row as usize).flat_map(|r|r[..level.width()as usize*4].iter().copied()).collect();
        assert_eq!(tight,level.pixels(),"actual GPU mip {mip}");drop(mapped);buffer.unmap();
    }
    let malformed=PixelData::from_decoded_levels(vec![PixelLevel::rgba8(4,4,vec![0;64]).unwrap(),PixelLevel::rgba8(1,1,vec![0;4]).unwrap()]).unwrap();
    assert!(upload_allocation(&renderer,&malformed).is_err());
}
