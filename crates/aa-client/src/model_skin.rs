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

///868020 for a skinned NiGeometry of the native object graph: 8C3A90 over
///its NiSkinInstance (+0xD0): bones (+0x14) with the skin data bones' bind
///bounds (+0x48 array, +0x34 of each), the skin data root transform
///(+0x10) and the root parent (+0x10 of the instance). `world` gives an
///object's current world transform.
pub(crate) fn geometry_model_bound(nif:&aa_formats::nif::NifFile,geometry:usize,world:&dyn Fn(usize)->Result<[u32;13]>,pc:Precision)->Result<[u32;4]>{
 let skin=nif.linked(geometry,"skin_instance").ok_or("868020 geometry without a skin instance")?;
 let instance=&nif.objects[skin];if instance.class!="NiSkinInstance"{return Err(format!("skin instance link to {}",instance.class))}
 let data=nif.linked(skin,"data").ok_or("8C3A9E NULL skin data")?;let parent=nif.linked(skin,"root_parent").ok_or("8C3AFA NULL skin root parent")?;
 let d=&nif.objects[data].fields;
 let root:[u32;13]=d.f32s("root_transform").ok_or("9096F0 root transform absent")?.try_into().map_err(|_|"skin root transform words")?;
 let bones:Vec<usize>=instance.fields.links("bones").ok_or("8C3810 bone links absent")?.iter().map(|&l|nif.target(l).ok_or_else(||"8C3AD3 NULL bone".to_string())).collect::<Result<_>>()?;
 let list=d.list("bone_list").ok_or("9096F0 bone list absent")?;
 if list.len()!=bones.len(){return Err("8C3ABB skin data bone count differs from the instance bone array".into())}
 let mut pairs=Vec::with_capacity(bones.len());
 for(&b,record)in bones.iter().zip(list){
  let aa_formats::nif::Value::Record(r)=record else{return Err("skin bone record".into())};
  let get=|n:&str|r.iter().find(|x|x.name==n).map(|x|&x.value);
  let c=match get("center"){Some(aa_formats::nif::Value::F32s(w))if w.len()==3=>[w[0],w[1],w[2]],_=>return Err("skin bone bound centre".into())};
  let radius=match get("radius"){Some(aa_formats::nif::Value::F32(w))=>*w,_=>return Err("skin bone bound radius".into())};
  pairs.push((world(b)?,[c[0],c[1],c[2],radius]));
 }
 model_bound(&pairs,&root,&world(parent)?,pc)
}
///877F40 for a skinned NiGeometry of the native object graph.
pub(crate) fn geometry_skin_to_root(nif:&aa_formats::nif::NifFile,geometry:usize,world:&dyn Fn(usize)->Result<[u32;13]>,pc:Precision)->Result<[u32;13]>{
 let skin=nif.linked(geometry,"skin_instance").ok_or("877F40 geometry without a skin instance")?;
 let data=nif.linked(skin,"data").ok_or("877F57 NULL skin data")?;let parent=nif.linked(skin,"root_parent").ok_or("877F46 NULL skin root parent")?;
 let root:[u32;13]=nif.objects[data].fields.f32s("root_transform").ok_or("9096F0 root transform absent")?.try_into().map_err(|_|"skin root transform words")?;
 skin_to_root(&root,&world(parent)?,pc)
}
