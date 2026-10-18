//! Render/picking transport of the actual NiCamera8737B0 output. This never
//! recomputes an inspector FOV or grants AD,74FF50 or battle activation.
use crate::{gpu::RendererWitness,inspection::Ray};
use aa_runtime::{native_camera::backend::Snapshot,world_revision::ConstructedWorldRevision};
use anyhow::{Context,Result,ensure};
use eframe::egui;
use glam::{Mat4,Vec3};
#[path="view_camera_projection.rs"]
mod projection;
use projection::Projection;
use std::sync::Arc;

///The same immutable camera packet may be shared by UI rendering and geometric
///picking. Its weak native-camera/World witnesses own neither allocation.
pub struct Frame{source:Snapshot,projection:Projection}
impl Frame{
    pub fn from_current(source:Snapshot,world:&ConstructedWorldRevision)->Result<Arc<Self>>{
        source.require_world(world).map_err(anyhow::Error::msg)?;
        let projection=Projection::from_native(source.matrix_words(),source.eye_words(),source.direction_words())?;
        let viewport=source.viewport_pixels();ensure!(!viewport.contains(&0),"native camera viewport is empty");
        //Non-full native viewport offsets need an actual GPU viewport/scissor
        //binding. Do not silently interpret that state as the whole target.
        let viewport_words=source.normalized_viewport_words().map(f32::from_bits);
        ensure!(viewport_words==[0.0,1.0,1.0,0.0],"native camera sub-viewport backend is required");
        Ok(Arc::new(Self{source,projection}))
    }
    pub fn viewport_pixels(&self)->[u32;2]{self.source.viewport_pixels()}
    pub(crate) fn require_current(&self,size:[u32;2])->Result<()>{
        self.source.require_current().map_err(anyhow::Error::msg)?;
        ensure!(size==self.viewport_pixels(),"render target resize requires an actual native camera projection update");Ok(())
    }
    pub(crate) fn matrix(&self)->Mat4{self.projection.matrix}
    pub(crate) fn eye(&self)->Vec3{self.projection.eye}
    pub(crate) fn direction(&self)->Vec3{self.projection.direction}
}

///Created only after a successful draw with the same camera and renderer.
///This is geometric picking of the rendered scene, not native order selection.
pub struct Rendered{camera:Arc<Frame>,renderer:RendererWitness,root_inverse:Mat4}
impl Rendered{
    pub(crate) fn after_draw(camera:&Arc<Frame>,renderer:RendererWitness,root_inverse:Mat4)->Self{
        Self{camera:Arc::clone(camera),renderer,root_inverse}
    }
    pub fn pointer_ray(&self,rect:egui::Rect,pointer:egui::Pos2,pixels:[u32;2])->Result<Option<Ray>>{
        self.camera.require_current(pixels)?;
        ensure!(self.renderer.is_current(),"picking snapshot renderer was dropped/reloaded");
        if !rect.contains(pointer)||rect.width()<=0.0||rect.height()<=0.0{return Ok(None)}
        let normalized=(pointer-rect.min)/rect.size();
        let Some(ray)=self.camera.projection.pointer_ray([normalized.x,normalized.y]) else{return Ok(None)};
        //SceneObjects holds pre-root render coordinates. Undo the same native
        //scene-root transform that the actual draw applied, exactly once.
        let origin=self.root_inverse.transform_point3(ray.origin);
        let direction=self.root_inverse.transform_vector3(ray.direction).try_normalize()
            .context("native scene inverse has no usable picking direction")?;
        ensure!(origin.is_finite()&&direction.is_finite(),"native picking ray is nonfinite");
        Ok(Some(Ray{origin,direction}))
    }
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn camera_transport_contains_only_send_metadata(){fn send<T:Send+Sync>(){}send::<Frame>();send::<Rendered>();}
}

