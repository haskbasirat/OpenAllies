//! Current texture policy and actual modern GPU services over the retained
//! simulation/UI bridge. This object owns no Resource flags or source pixels.
use crate::battle_ui_bridge;
use aa_runtime::{resource_depot::{Application,Resources},resource_objects::{Handle,TexturePolicy},
    resource_texture::{SourceTexture,TextureInput,DecodedTexture},simulation_execution::Execution,texture_resource_load::{self,Variable,Diagnostic}};
use std::sync::Arc;
type Result<T>=std::result::Result<T,String>;

/// Used only by this client's explicit fresh-start/no-imported-profile path,
/// the same scope selected in Engine::start's current variable construction.
pub struct Current<'a,'execution>{
    bridge:&'a battle_ui_bridge::Client,
    execution:&'a mut Execution<'execution>,
}
impl<'a,'execution> Current<'a,'execution>{
    pub fn for_controlled_fresh_start(bridge:&'a battle_ui_bridge::Client,execution:&'a mut Execution<'execution>)->Self{
        Self{bridge,execution}
    }
    /// Deliberate modern renderer-init mapping, performed using a capability
    /// minted after real device/pipeline initialization. It is not a loaded
    /// Texture, completed native7101A0, or completed VWorld certificate.
    pub fn apply_initialized_renderer(&mut self,application:&mut Application)->Result<()>{
        let initialized=self.bridge.call(self.execution,|context|Ok(context.renderer.renderer_initialization()))?;
        application.apply_renderer_init_710433(&initialized)
    }
    fn retain_decoded_error(resources:&Resources<'_>,source:&SourceTexture,decoded:DecodedTexture,error:String)->Result<()>{
        resources.retain_failed_decoded_40(source,decoded).map_err(|failure|
            format!("{error}; same-source failed decoder retention: {}",failure.error))?;
        Err(error)
    }
}
impl TexturePolicy for Current<'_,'_>{
    fn data_compatibility_mode_current(&mut self)->Result<u8>{
        self.bridge.call(self.execution,|context|context.ui.uvars.data_compatibility_mode_current())
    }
    fn data_missing_resource_warning_current(&mut self)->Result<u8>{
        self.bridge.call(self.execution,|context|context.ui.uvars.data_missing_resource_warning_current())
    }
    fn missing_texture_diagnostic_404fc0(&mut self,ids:&[u16],level:u32)->Result<()>{
        log::warn!("native404FC0 level={level}: missing Texture {}",String::from_utf16_lossy(ids));Ok(())
    }
    fn texture_cache_source_overrides(&mut self)->Result<aa_runtime::load_view_variables::NativeOverrides>{
        Ok(aa_runtime::load_view_variables::NativeOverrides::ControlledNone)
    }
}
impl texture_resource_load::Backend for Current<'_,'_>{
    fn variable_current(&mut self,variable:Variable)->Result<u8>{
        self.bridge.call(self.execution,move|context|match variable{
            Variable::ResourceNoNiTextures=>context.ui.uvars.resource_no_ni_textures_current(),
            Variable::ForceLowQualityTextures=>context.ui.uvars.renderer_force_low_quality_textures_current(),
            Variable::UseHighQualityTextures=>context.ui.uvars.renderer_use_high_quality_textures_current(),
        })
    }
    fn diagnostic(&mut self,resource:&Handle,event:Diagnostic)->Result<()>{
        log::info!("native Texture {:?}: {}",event,String::from_utf16_lossy(resource.logical_ids()));Ok(())
    }
    fn renderer_b442c0_present(&mut self)->Result<bool>{
        // Context can only be pumped with the actual retained SceneRenderer.
        // Its real device allocation is checked; no nullable-owner guess.
        self.bridge.call(self.execution,|context|{
            context.renderer.camera_device_reference().require_alive().map_err(|e|e.to_string())?;Ok(true)
        })
    }
    fn create_renderer_data_100(&mut self,resources:&mut Resources<'_>,source:&SourceTexture)->Result<bool>{
        let pixels=resources.source_pixels_for_renderer(source)?
            .ok_or("actual Renderer vf100 source has no decoded PixelData")?;
        let input=pixels.clone();
        let allocation=self.bridge.call(self.execution,move|context|
            crate::gpu::native_texture::upload(context.renderer,&input).map(Arc::new).map_err(|e|e.to_string()))?;
        resources.attach_source_renderer_2c(&pixels,allocation)?;
        Ok(true)
    }
    fn load_selected_stream_40(&mut self,resources:&mut Resources<'_>,source:&SourceTexture,input:&mut Option<TextureInput>)->Result<()>{
        let input=input.as_ref().ok_or("native Source vf40 selected stream is absent")?;
        let mode=resources.source_mode_48(source)?;
        let layout=aa_runtime::texture_pixel_layout::NativePixelLayout::from_input(input,mode)?;
        let decoded=crate::native_texture_decoder::decode_retained(input)?;
        let decoded=match decoded.apply_native_layout(&layout){
            Ok(decoded)=>decoded,
            Err(failure)=>return Self::retain_decoded_error(resources,source,failure.decoded,failure.error),
        };
        if let Err(failure)=resources.publish_decoded_texture_40(source,decoded,&layout){
            return Self::retain_decoded_error(resources,source,failure.decoded,failure.error);
        }
        //Native87BB50 publishes CPU Pixel40; it does not create Renderer2C.
        //A genuine later87BA60/vf100 performs that distinct GPU operation.
        Ok(())
    }
    fn close_selected_stream_8761a0(&mut self,resource:&Handle)->Result<()>{
        if resource.source_file().is_none(){return Err("selected-stream close requires its actual physical Resource".into())}
        //The modern stream is the real retained TextureInput in the loader's
        //Call. Its adapter drops that SAME owner immediately after this
        //transport callback; no native NiStream/OS handle is fabricated here.
        log::debug!("closing actual selected Texture input: {}",String::from_utf16_lossy(resource.logical_ids()));
        Ok(())
    }
    ///5A6A10 -> 87C5C0/87C280 power-of-two pad on the SAME texture arena.
    fn resize_87c5c0(&mut self,resources:&mut Resources<'_>,source:&SourceTexture,target:[u32;2])->Result<Option<SourceTexture>>{
        resources.resize_texture_source_87c5c0(source,target)
    }
    // Unsupported direct reload/assert callbacks retain the trait's precise
    // reached errors; no guessed source or image is substituted.
}
