//!NiSkinInstance transform math of the update and of metadata processing.
//!
//!* 8708E0 NiMatrix3::Transpose (m0..m2 through FLD/FSTP, the rest moved);
//!* 870720 NiMatrix3 * NiPoint3 (z, y, x reduced in instruction order, each
//!  component stored);
//!* 8751F0 NiTransform::Invert: transposed rotation, 1 / scale (stored),
//!  translation = (1/s) * (R^T * -t) with every product stored;
//!* 877F40 NiSkinInstance skin-to-root transform: the skin data's root
//!  transform (+0x10) composed (45CAB0) with the inverse of the root parent's
//!  world transform;
//!* 8C3A90 NiSkinInstance::UpdateModelBound: each bone's bind bound (skin
//!  data bone +0x34) through that bone's world transform (89FBF0), merged in
//!  bone order (89FAD0), then through the 877F40 transform; 868020 stores the
//!  result in the GeometryData model bound (+0x10) before the world bound.
use aa_numeric::{Binary,Precision};
use aa_runtime::resource_model_bounds as math;
type Result<T>=std::result::Result<T,String>;
fn ld(w:u32)->Result<Binary>{Binary::from_f32_bits(w).map_err(|e|e.to_string())}
fn st(v:&Binary)->Result<u32>{v.store_f32().map_err(|e|e.to_string())}
///B32D94, the 89FAD0 small-distance constant the bounds use.
const MERGE_EPSILON:u32=897988541;

///8708E0.
pub(crate) fn transpose(m:&[u32]) ->[u32;9]{[m[0],m[3],m[6],m[1],m[4],m[7],m[2],m[5],m[8]]}
///870720: out = M p.
pub(crate) fn rotate(m:&[u32],p:[u32;3],pc:Precision)->Result<[u32;3]>{
 let mul=|a:usize,j:usize|->Result<Binary>{ld(m[a])?.mul(&ld(p[j])?,pc).map_err(|e|e.to_string())};
 let add=|a:Binary,b:Binary|->Result<Binary>{a.add(&b,pc).map_err(|e|e.to_string())};
 let z=add(add(mul(8,2)?,mul(6,0)?)?,mul(7,1)?)?;
 let y=add(add(mul(5,2)?,mul(3,0)?)?,mul(4,1)?)?;
 let x=add(add(mul(2,2)?,mul(1,1)?)?,mul(0,0)?)?;
 Ok([st(&x)?,st(&y)?,st(&z)?])
}
///8751F0 on a 13-word transform (rotation 9, translation 3, scale).
pub(crate) fn invert(t:&[u32;13],pc:Precision)->Result<[u32;13]>{
 let r=transpose(&t[..9]);
 let s=st(&ld(0x3f80_0000)?.div(&ld(t[12])?,pc).map_err(|e|e.to_string())?)?;
 let p=rotate(&r,[t[9]^0x8000_0000,t[10]^0x8000_0000,t[11]^0x8000_0000],pc)?;
 let mut out=[0u32;13];out[..9].copy_from_slice(&r);
 for i in 0..3{out[9+i]=st(&ld(s)?.mul(&ld(p[i])?,pc).map_err(|e|e.to_string())?)?;}
 out[12]=s;Ok(out)
}
///877F40.
pub(crate) fn skin_to_root(root_transform:&[u32;13],root_parent_world:&[u32;13],pc:Precision)->Result<[u32;13]>{
 math::compose_world(*root_transform,invert(root_parent_world,pc)?,pc)
}
///8C3A90: `bones` pairs each bone's current world transform with its skin
///data bind bound (centre, radius).
pub(crate) fn model_bound(bones:&[([u32;13],[u32;4])],root_transform:&[u32;13],root_parent_world:&[u32;13],pc:Precision)->Result<[u32;4]>{
 let((first_world,first_bound),rest)=bones.split_first().ok_or("8C3AA3 reads bone 0 of a skin without bones")?;
 let mut s=math::transform_sphere(first_world,*first_bound,pc)?;
 for(world,bound)in rest{let t=math::transform_sphere(world,*bound,pc)?;math::merge_sphere(&mut s,t,MERGE_EPSILON,pc)?;}
 math::transform_sphere(&skin_to_root(root_transform,root_parent_world,pc)?,s,pc)
}
