//! Actual retained native scene semantics -> modern GPU uniform installation.
//! This is a backend child receipt, never AD/74FF50 or battle activation.
use crate::gpu::{RendererWitness,SceneRenderer};
use aa_runtime::{view_scene_init::{State,SceneWitness,RendererBinding,LightInfo},world_revision::ConstructedWorldRevision};
use anyhow::{Context,Result,ensure};
use glam::{Mat4,Vec3,Vec4};
use std::sync::{Arc,Weak};

#[derive(Clone,PartialEq,Eq)]
struct Plan{root:[u32;13],light:LightInfo}
impl Plan{
    fn current(state:&State,world:&ConstructedWorldRevision)->Result<Self>{
        state.require_cpu_complete(world).map_err(anyhow::Error::msg)?;
        let root=state.root().context("native F0 scene root is absent")?;
        ensure!(root.has_parent()&&root.property_count()==1&&root.effect_count()==1&&root.child_count()>=1,
            "native root/property/light attachment semantics differ");
        ensure!(!state.wireframe().context("native wireframe property absent")?.enabled(),"positive native wireframe backend is required");
        let texture=state.texturing().context("native root texturing property absent")?;
        ensure!(texture.apply_mode_word()==2&&texture.base_filter_word()==Some(2),"native root texturing apply/filter backend is required");
        let effect=state.texture_effect().context("native texture effect absent")?;
        ensure!(!effect.source_texture_present()&&effect.filter_word()==2&&effect.clamp_word()==3&&effect.kind_word()==0&&effect.enabled(),
            "positive/different native texture effect backend is required");
        Ok(Self{root:*root.world_transform().words(),light:state.light_info().context("native directional light absent")?})
    }
    fn uniforms(&self)->Result<Uniforms>{
        let root=render_root(self.root)?;
        let direction=Vec3::from_array(self.light.direction.map(f32::from_bits));
        ensure!(direction.is_finite()&&direction.length_squared()>0.0,"native light direction is nonfinite/zero");
        //A directional light's direction follows the rays. Surface lighting
        //uses the opposite direction, in the established native->render basis.
        let direction=-Vec3::from_array(aa_assets::native_to_render(direction.to_array()));
        let dimmer=f32::from_bits(self.light.dimmer);ensure!(dimmer.is_finite(),"native light dimmer is nonfinite");
        let color=|words:[u32;3]|->Result<[f32;4]>{let value=words.map(f32::from_bits);ensure!(value.iter().all(|v|v.is_finite()),"native light color is nonfinite");
            let out=value.map(|v|v*dimmer);ensure!(out.iter().all(|v|v.is_finite()),"native light color overflow");Ok([out[0],out[1],out[2],1.0])};
        Ok(Uniforms{root:root.to_cols_array_2d(),inverse:root.inverse().to_cols_array_2d(),normal:root.inverse().transpose().to_cols_array_2d(),
            direction:[direction.x,direction.y,direction.z,1.0],ambient:color(self.light.ambient)?,diffuse:color(self.light.diffuse)?,specular:color(self.light.specular)?})
    }
}
fn render_root(words:[u32;13])->Result<Mat4>{
    let w=words.map(f32::from_bits);ensure!(w.iter().all(|v|v.is_finite()),"native scene root transform is nonfinite");
    let scale=w[12];let native=Mat4::from_cols(
        Vec4::new(w[0]*scale,w[3]*scale,w[6]*scale,0.0),Vec4::new(w[1]*scale,w[4]*scale,w[7]*scale,0.0),
        Vec4::new(w[2]*scale,w[5]*scale,w[8]*scale,0.0),Vec4::new(w[9],w[10],w[11],1.0));
    let basis=Mat4::from_cols(Vec4::X,Vec4::new(0.0,0.0,-1.0,0.0),Vec4::Y,Vec4::W);
    let root=basis*native*basis.transpose();
    ensure!(root.is_finite()&&root.determinant().is_finite()&&root.determinant()!=0.0,"native scene root is singular/unsupported");
    ensure!(root.inverse().is_finite(),"native scene inverse is nonfinite");Ok(root)
}
pub(crate) struct Uniforms{
    pub root:[[f32;4];4],pub inverse:[[f32;4];4],pub normal:[[f32;4];4],
    pub direction:[f32;4],pub ambient:[f32;4],pub diffuse:[f32;4],pub specular:[f32;4],
}
struct Owner{scene:SceneWitness,plan:Plan}
pub(crate) struct Installed{owner:Arc<Owner>,uniforms:Uniforms,accepted:bool}
impl Installed{
    pub(crate) fn uniforms(&self)->&Uniforms{&self.uniforms}
    pub(crate) fn accept_gpu_writes(&mut self){self.accepted=true;}
    pub(crate) fn require_alive(&self)->Result<()>{ensure!(self.accepted,"native GPU scene installation failed before accepted writes");ensure!(self.owner.scene.is_for(&self.owner.scene),"native GPU scene semantic owner expired");Ok(())}
}
///Immutable semantics only; no Rc NiNode or authoritative World is transferred.
pub struct Request{scene:SceneWitness,plan:Plan}
pub struct Receipt{owner:Weak<Owner>,renderer:RendererWitness}
impl Request{
    pub fn from_current(state:&State,world:&ConstructedWorldRevision)->Result<Self>{
        Ok(Self{scene:state.witness().map_err(anyhow::Error::msg)?,plan:Plan::current(state,world)?})
    }
    pub fn install(self,renderer:&mut SceneRenderer)->Result<Receipt>{
        ensure!(self.scene.is_for(&self.scene),"native scene request owner expired");
        let installed=Installed{uniforms:self.plan.uniforms()?,owner:Arc::new(Owner{scene:self.scene,plan:self.plan}),accepted:false};
        let owner=Arc::downgrade(&installed.owner);
        renderer.install_native_scene(installed)?;
        Ok(Receipt{owner,renderer:renderer.identity_witness()})
    }
}
impl RendererBinding for Receipt{
    fn validate(&self,scene:&SceneWitness,world:&ConstructedWorldRevision,current:&State)->std::result::Result<(),String>{
        (||->Result<()>{let owner=self.owner.upgrade().context("actual GPU scene installation was replaced/dropped")?;
            ensure!(self.renderer.is_current(),"actual GPU renderer scene epoch was replaced/dropped");
            ensure!(owner.scene.is_for(scene),"GPU binding belongs to another semantic scene allocation");
            scene.require_world(world).map_err(anyhow::Error::msg)?;
            ensure!(owner.plan==Plan::current(current,world)?,"current native root/light semantics changed after GPU installation");Ok(())
        })().map_err(|e|e.to_string())
    }
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn actual_native_to_render_root_is_applied_once(){
        let native=[0.0f32,-1.0,0.0,1.0,0.0,0.0,0.0,0.0,1.0,3.0,5.0,7.0,2.0];
        let root=render_root(native.map(f32::to_bits)).unwrap();
        let point=Vec3::new(11.0,13.0,17.0);let n=Vec3::new(3.0-2.0*13.0,5.0+2.0*11.0,7.0+2.0*17.0);
        let transformed=root.transform_point3(Vec3::from_array(aa_assets::native_to_render(point.to_array())));
        assert_eq!(transformed,Vec3::from_array(aa_assets::native_to_render(n.to_array())));
        assert_eq!(root.inverse().transform_point3(transformed),Vec3::from_array(aa_assets::native_to_render(point.to_array())));
        let mut zero=native;zero[12]=0.0;assert!(render_root(zero.map(f32::to_bits)).is_err());
        fn send<T:Send>(){}send::<Request>();send::<Receipt>();
    }
}
