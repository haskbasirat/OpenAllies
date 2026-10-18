//! Exact NiCamera row-to-render-basis conversion, independent of lifecycle.
//! No raw matrix constructed here is a native current-camera capability.
use anyhow::{Result,ensure};
use glam::{Mat4,Vec3,Vec4};
#[derive(Clone,Copy)]
pub(super) struct Ray{pub(super) origin:Vec3,pub(super) direction:Vec3}
pub(super) struct Projection{pub(super) matrix:Mat4,inverse:glam::DMat4,pub(super) eye:Vec3,pub(super) direction:Vec3}
impl Projection{
    pub(super) fn from_native(matrix:[u32;16],eye:[u32;3],direction:[u32;3])->Result<Self>{
        let words=matrix.map(f32::from_bits);
        ensure!(words.iter().all(|v|v.is_finite()),"native NiCamera matrix is nonfinite");
        //8737B0 writes four consecutive ROWS. Native [x,y,z] becomes render
        //[x,z,-y], so M_render=M_native*C^-1. This is a column permutation
        //and sign change, not a rebuilt projection/depth conversion.
        let row=|i:usize|Vec4::new(words[4*i],words[4*i+2],-words[4*i+1],words[4*i+3]);
        let matrix=Mat4::from_cols(row(0),row(1),row(2),row(3)).transpose();
        let determinant=matrix.as_dmat4().determinant();
        ensure!(determinant.is_finite()&&determinant!=0.0,"native NiCamera projection is singular");
        let inverse=matrix.as_dmat4().inverse();ensure!(inverse.is_finite(),"native NiCamera inverse is nonfinite");
        let eye=Vec3::from_array(aa_assets::native_to_render(eye.map(f32::from_bits)));
        let direction=Vec3::from_array(aa_assets::native_to_render(direction.map(f32::from_bits)));
        ensure!(eye.is_finite()&&direction.is_finite(),"native camera eye/direction is nonfinite");
        ensure!(direction.length_squared()>0.0,"native camera direction is zero");
        Ok(Self{matrix,inverse,eye,direction})
    }
    pub(super) fn pointer_ray(&self,normalized:[f32;2])->Option<Ray>{
        if normalized.iter().any(|v|!v.is_finite()||!(0.0..=1.0).contains(v)){return None}
        let ndc=glam::DVec3::new(f64::from(normalized[0])*2.0-1.0,1.0-f64::from(normalized[1])*2.0,0.0);
        let near=self.inverse.project_point3(ndc);let far=self.inverse.project_point3(ndc.with_z(1.0));
        let direction=(far-near).try_normalize()?.as_vec3();let origin=near.as_vec3();
        (origin.is_finite()&&direction.is_finite()).then_some(Ray{origin,direction})
    }
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn native_rows_keep_clip_depth_and_render_basis_without_fov_reconstruction(){
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-native-camera/native-camera-ni.json")).unwrap();
        let offsets=fixture["output_offsets"].as_array().unwrap();let mut count=0;let mut refused=0;
        for row in fixture["rows"].as_array().unwrap(){
            if !row["output"]["error"].is_null(){continue}
            let out=row["output"]["words"].as_array().unwrap();
            let word=|offset:u32|out[offsets.iter().position(|v|v.as_u64()==Some(u64::from(offset))).unwrap()].as_u64().unwrap()as u32;
            let matrix=std::array::from_fn(|i|word(0xc8+i as u32*4));
            let eye=std::array::from_fn(|i|word(0x90+i as u32*4));
            let direction=std::array::from_fn(|i|word(0x114+i as u32*4));
            let projection=Projection::from_native(matrix,eye,direction);
            if row["input"]["local"].as_array().unwrap()[..9].iter().all(|v|f32::from_bits(v.as_u64().unwrap()as u32)==0.0){
                assert!(projection.is_err(),"zero native basis cannot yield an inverse picking projection");refused+=1;continue
            }
            let projection=projection.unwrap();
            let native=Mat4::from_cols_array(&matrix.map(f32::from_bits)).transpose();
            for point in [[1.0,2.0,3.0],[-2.0,4.0,-8.0],[0.0,0.0,0.0]]{
                let expected=native*Vec3::from_array(point).extend(1.0);
                let actual=projection.matrix*Vec3::from_array(aa_assets::native_to_render(point)).extend(1.0);
                for i in 0..4{let tolerance=expected[i].abs().max(1.0)*2.0e-6;assert!((actual[i]-expected[i]).abs()<=tolerance,"row{} component{i}",row["case"]);}
            }
            assert_eq!(projection.eye.to_array(),aa_assets::native_to_render(eye.map(f32::from_bits)));count+=1;
        }
        assert_eq!(count,120);assert_eq!(refused,24);
    }
    #[test]fn actual_projection_inverse_picks_clip_endpoints_and_refuses_invalid_matrices(){
        let rows=[2.0f32,0.0,0.0,0.0,0.0,3.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,1.0];
        let projection=Projection::from_native(rows.map(f32::to_bits),[0;3],[0,0,1.0f32.to_bits()]).unwrap();
        let ray=projection.pointer_ray([0.5,0.5]).unwrap();
        assert_eq!(ray.origin,Vec3::ZERO);assert_eq!(ray.direction,Vec3::Y);
        assert!(projection.pointer_ray([-0.1,0.5]).is_none());
        assert!(Projection::from_native([0;16],[0;3],[1.0f32.to_bits(),0,0]).is_err());
        let mut bad=rows.map(f32::to_bits);bad[3]=f32::INFINITY.to_bits();
        assert!(Projection::from_native(bad,[0;3],[1.0f32.to_bits(),0,0]).is_err());
    }
}
