//!Native time-controller updates of the model-processing pass.
//!
//!87AA10 calls NiAVObject::Update 86E8D0 (time 0, update controllers) at
//!87ABC0, and 5AA0DD repeats it on the wrapped root. UpdateDownwardPass
//!(NiNode 866EB0, NiGeometry 86EED0) first runs UpdateObjectControllers
//!866BE0: each Property of the object's +0xA4 list whose controller head is
//!set (vtable +0x38 = 86A520 walks the chain), then the object's own chain
//!(+0x10, next +0x38), each through vtable +0x3C Update; then UpdateWorldData
//!8A2C70 and the children, so controllers run in preorder before the world
//!transform of the object that owns them.
//!
//!Ported Update functions: NiKeyframeController 8C0200 and NiAlphaController
//!8C2740, with DontDoUpdate 87C780 and the clock 89E9B0 (aa-animation
//!ControllerClock). Key interpolation: float 860890 (linear 8639E0 ->
//!8B7CC0, quadratic 860BF0 -> 8B7CE0), position 861000 (linear 863D10,
//!quadratic 861720 with the load-time coefficients of 861900), rotation
//!862170 (linear 864120 = 8B7F80 blend + 8B7E20 normalize, after the
//!load-time pass 862490; Euler 862FE0 = per-axis float keys, 871130 axis
//!quaternions, 870E80 products) and NiQuaternion::ToRotation 4465A0.
//!Every x87 operation rounds at the caller's precision and every binary32
//!store is explicit. FSIN/FCOS (871130) use an enclosure of the true value;
//!a result whose binary32 rounding the hardware error could change is an
//!explicit error. Every other controller class or key type is an explicit
//!unported error, as are the native NULL/out-of-range key reads.
//!
//!872650 with the 8785D0 callback (KKC_NiProcessing::ProcessTimeControllers,
//!metadata processing) is `visit_metadata`.
use aa_animation::controller_clock::{ClockCache,Controller as Clock,ControllerClock};
use aa_body::F32;
use aa_formats::nif::{NifFile,Value};
use aa_numeric::{Binary,Precision};
use std::cmp::Ordering;
use std::collections::BTreeMap;
type Result<T>=std::result::Result<T,String>;
fn e(x:aa_numeric::Error)->String{format!("x87 arithmetic outside the ported domain: {x}")}
fn ld(w:u32)->Result<Binary>{Binary::from_f32_bits(w).map_err(e)}
fn st(x:&Binary)->Result<u32>{x.store_f32().map_err(e)}
const ONE:u32=0x3f80_0000;
const HALF:u32=0x3f00_0000;//9FBDB8
const THREE:u32=0x4040_0000;//A32690
const MINUS_ONE:u32=0xbf80_0000;//9FDC48
///-FLT_MAX: the NiTimeController constructor 89ED80 value of +0x24/+0x2C.
pub(crate) const UNSET:u32=0xff7f_ffff;
///x87 arithmetic at one precision control.
#[derive(Clone,Copy)]
struct X(Precision);
impl X{
 fn add(self,a:&Binary,b:&Binary)->Result<Binary>{a.add(b,self.0).map_err(e)}
 fn sub(self,a:&Binary,b:&Binary)->Result<Binary>{a.sub(b,self.0).map_err(e)}
 fn mul(self,a:&Binary,b:&Binary)->Result<Binary>{a.mul(b,self.0).map_err(e)}
 fn div(self,a:&Binary,b:&Binary)->Result<Binary>{a.div(b,self.0).map_err(e)}
}
fn less(a:&Binary,b:&Binary)->bool{a.compare(b)==Ordering::Less}
fn equal(a:&Binary,b:&Binary)->bool{a.compare(b)==Ordering::Equal}

///Words a controller writes into its target.
pub(crate) trait Targets{
 ///NiAVObject +0x38 rotation (9), +0x5C translation (3), +0x68 scale.
 fn local(&mut self,block:usize)->Result<&mut [u32;13]>;
 ///NiMaterialProperty +0x24: ambient, diffuse, specular, emissive, +0x54 shine, +0x58 alpha.
 fn material(&mut self,block:usize)->Result<&mut [u32;14]>;
}

//---------------------------------------------------------------------------
// Keys as the native loaders and the B45898 fill passes leave them. `None`
// is the NULL key pointer (no keys, or the 4C9FA0 loader of type 0).
#[derive(Clone,Debug)]
enum Floats{Single(u32),Linear(Vec<[u32;2]>),Quadratic(Vec<[u32;4]>)}
#[derive(Clone,Debug)]
enum Points{Single([u32;3]),Linear(Vec<[u32;4]>),Quadratic(Vec<PointKey>)}
///Quadratic position key: +0 time, +8 value, +0x20 backward tangent and the
///861900 coefficients +0x2C (A) and +0x38 (B) of the segment it starts.
#[derive(Clone,Debug)]
struct PointKey{time:u32,value:[u32;3],backward:[u32;3],a:[u32;3],b:[u32;3]}
#[derive(Clone,Debug)]
enum Rotations{Single([u32;4]),Linear(Vec<[u32;5]>),Euler{order:u32,axes:Box<[Option<Floats>;3]>}}

fn rows(keys:&Value,count:u32,width:usize)->Result<Vec<&[u32]>>{
 let Value::List(list)=keys else{return Err("native key array absent".into())};
 if list.len()!=count as usize{return Err("native key count differs from its key array".into())}
 list.iter().map(|k|match k{Value::F32s(w)if w.len()==width=>Ok(w.as_slice()),_=>Err("native key words differ from the key type".into())}).collect()
}
fn key_list(keys:&Value)->Result<&[Value]>{match keys{Value::List(l)=>Ok(l),_=>Err("native key array absent".into())}}
fn field<'a>(record:&'a [aa_formats::nif::Field],name:&str)->Option<&'a Value>{record.iter().find(|f|f.name==name).map(|f|&f.value)}
fn word(record:&[aa_formats::nif::Field],name:&str)->Option<u32>{match field(record,name){Some(Value::U32(v))=>Some(*v),_=>None}}

///Float keys (B45DC0 loaders 863A50 linear / 860E30 quadratic / 865B10 TBC;
///the B45898 fills of these are 4B2830 no-ops and 865940 for TBC). 860890
///returns key 0 for one key whatever its type.
fn float_keys(count:u32,kind:Option<u32>,keys:Option<&Value>)->Result<Option<Floats>>{
 if count==0{return Ok(None)}
 let kind=kind.ok_or("float key type absent")?;let keys=keys.ok_or("float keys absent")?;
 let width=match kind{0=>return Ok(None),1=>2,2=>4,3=>5,_=>return Err(format!("float key type {kind} has no native loader"))};
 let r=rows(keys,count,width)?;
 if r.len()==1{return Ok(Some(Floats::Single(r[0][1])))}
 match kind{
  1=>Ok(Some(Floats::Linear(r.iter().map(|k|[k[0],k[1]]).collect()))),
  2=>Ok(Some(Floats::Quadratic(r.iter().map(|k|[k[0],k[1],k[2],k[3]]).collect()))),
  _=>Err("multi-key TBC float keys (865820/865940) are not ported".into()),
 }
}
///Position keys (863DF0 linear / 861BC0 quadratic + 861900 / 864E70 TBC).
fn point_keys(count:u32,kind:Option<u32>,keys:Option<&Value>,x:X)->Result<Option<Points>>{
 if count==0{return Ok(None)}
 let kind=kind.ok_or("position key type absent")?;let keys=keys.ok_or("position keys absent")?;
 let width=match kind{0=>return Ok(None),1=>4,2=>10,3=>7,_=>return Err(format!("position key type {kind} has no native loader"))};
 let r=rows(keys,count,width)?;
 if r.len()==1{return Ok(Some(Points::Single([r[0][1],r[0][2],r[0][3]])))}
 match kind{
  1=>Ok(Some(Points::Linear(r.iter().map(|k|[k[0],k[1],k[2],k[3]]).collect()))),
  2=>Ok(Some(Points::Quadratic(point_quadratic_fill(&r,x)?))),
  _=>Err("multi-key TBC position keys (864570/864910) are not ported".into()),
 }
}
///Rotation keys (864160 linear + 862490; 8634C0 Euler + 8632B0). 862170
///returns key 0 for one key except for Euler, which always evaluates key 0.
fn rotation_keys(count:u32,kind:Option<u32>,keys:Option<&Value>,x:X)->Result<Option<Rotations>>{
 if count==0{return Ok(None)}
 let kind=kind.ok_or("rotation key type absent")?;let keys=keys.ok_or("rotation keys absent")?;
 match kind{
  0=>Err("rotation key type 0: the 862490 fill dereferences the NULL key array".into()),
  1=>{let mut k:Vec<[u32;5]>=rows(keys,count,5)?.iter().map(|k|[k[0],k[1],k[2],k[3],k[4]]).collect();rotation_linear_fill(&mut k,x)?;
   if k.len()==1{return Ok(Some(Rotations::Single([k[0][1],k[0][2],k[0][3],k[0][4]])))}Ok(Some(Rotations::Linear(k)))}
  3=>{let r=rows(keys,count,8)?;if r.len()==1{return Ok(Some(Rotations::Single([r[0][1],r[0][2],r[0][3],r[0][4]])))}Err("multi-key TBC rotation keys (865050 squad, 865300 controls) are not ported".into())}
  4=>{
   let list=key_list(keys)?;if list.len()!=count as usize{return Err("native key count differs from its key array".into())}
   let Some(Value::Record(key))=list.first()else{return Err("Euler key record absent".into())};
   let order=word(key,"order").ok_or("Euler order absent")?;
   let Some(Value::List(axes))=field(key,"axes")else{return Err("Euler axes absent".into())};
   if axes.len()!=3{return Err("Euler axis census differs".into())}
   let mut out:[Option<Floats>;3]=[None,None,None];
   for(slot,axis)in out.iter_mut().zip(axes){let Value::Record(a)=axis else{return Err("Euler axis record absent".into())};
    let n=word(a,"count").ok_or("Euler axis count absent")?;
    //862FE0 passes a positive count with its keys to 860890; a NULL key
    //array with a positive count is read through.
    *slot=float_keys(n,word(a,"type"),field(a,"keys"))?;if n>0&&slot.is_none(){return Err("Euler axis with keys of type 0: 860890 reads the NULL key array".into())}}
   Ok(Some(Rotations::Euler{order,axes:Box::new(out)}))
  }
  _=>Err(format!("rotation key type {kind} (862750 quadratic / 862BE0) is not ported")),
 }
}
///862490: in increasing pairs, when the dot ((z z' + y y') + x x') + w w' of
///key i and key i+1 is negative, 870E50 negates key i+1 (FCHS); then every
///w is clamped to [-1, 1]. Key words: time, w, x, y, z.
fn rotation_linear_fill(keys:&mut [[u32;5]],x:X)->Result<()>{
 for i in 0..keys.len().saturating_sub(1){
  let(a,b)=(keys[i],keys[i+1]);
  let mut dot=x.mul(&ld(a[4])?,&ld(b[4])?)?;
  for k in [3,2,1]{dot=x.add(&dot,&x.mul(&ld(a[k])?,&ld(b[k])?)?)?;}
  if less(&dot,&Binary::from_u32(0)){for k in 1..5{keys[i+1][k]^=0x8000_0000;}}
 }
 for k in keys.iter_mut(){let w=ld(k[1])?;if less(&w,&ld(MINUS_ONE)?){k[1]=MINUS_ONE}else if less(&ld(ONE)?,&w){k[1]=ONE}}
 Ok(())
}
///861900, for each segment (key i, key i+1), with d = v1 - v0, m0 = key i
///backward (+0x20), m1 = key i+1 forward (+0x14):
///A = 3d - (2 m0 + m1) (+0x2C), B = (m1 + m0) - 2d (+0x38), with the
///instruction's binary32 spills (A: 3dx, 2m0x + m1x, dz, 3dz, 2m0z; B: 2dx,
///dz, m0z + m1z).
fn point_quadratic_fill(r:&[&[u32]],x:X)->Result<Vec<PointKey>>{
 let mut out:Vec<PointKey>=r.iter().map(|k|PointKey{time:k[0],value:[k[1],k[2],k[3]],backward:[k[7],k[8],k[9]],a:[0;3],b:[0;3]}).collect();
 let three=ld(THREE)?;
 for i in 0..r.len()-1{
  let(k0,k1)=(r[i],r[i+1]);
  let v0=|j:usize|ld(k0[1+j]);let v1=|j:usize|ld(k1[1+j]);let m0=|j:usize|ld(k0[7+j]);let m1=|j:usize|ld(k1[4+j]);
  let d=|j:usize|->Result<Binary>{x.sub(&v1(j)?,&v0(j)?)};
  let mut a=[0u32;3];let mut b=[0u32;3];
  //A.x = f32(f32(3 dx) - f32(2 m0x + m1x))
  let sx=ld(st(&x.add(&x.add(&m0(0)?,&m0(0)?)?,&m1(0)?)?)?)?;
  a[0]=st(&x.sub(&ld(st(&x.mul(&d(0)?,&three)?)?)?,&sx)?)?;
  //A.y = f32(3 dy - (2 m0y + m1y))
  let sy=x.add(&x.add(&m0(1)?,&m0(1)?)?,&m1(1)?)?;
  a[1]=st(&x.sub(&x.mul(&d(1)?,&three)?,&sy)?)?;
  //A.z = f32(f32(3 f32(dz)) - (f32(2 m0z) + m1z))
  let sz=x.add(&ld(st(&x.add(&m0(2)?,&m0(2)?)?)?)?,&m1(2)?)?;
  let dz=ld(st(&d(2)?)?)?;
  a[2]=st(&x.sub(&ld(st(&x.mul(&dz,&three)?)?)?,&sz)?)?;
  //B.x = f32((m1x + m0x) - f32(2 dx))
  let dx=d(0)?;b[0]=st(&x.sub(&x.add(&m1(0)?,&m0(0)?)?,&ld(st(&x.add(&dx,&dx)?)?)?)?)?;
  //B.y = f32((m0y + m1y) - 2 dy)
  let dy=d(1)?;b[1]=st(&x.sub(&x.add(&m0(1)?,&m1(1)?)?,&x.add(&dy,&dy)?)?)?;
  //B.z = f32(f32(m0z + m1z) - 2 f32(dz))
  b[2]=st(&x.sub(&ld(st(&x.add(&m0(2)?,&m1(2)?)?)?)?,&x.add(&dz,&dz)?)?)?;
  out[i].a=a;out[i].b=b;
 }
 Ok(out)
}

//---------------------------------------------------------------------------
///The key search shared by 860890 / 861000 / 862170: restart at key 0 when
///the time precedes the cached key, advance while the time is after the next
///key, and u = (t - t0) / (t1 - t0) at the caller's precision, stored binary32.
fn segment(times:&[u32],time:u32,last:&mut u32,x:X)->Result<(usize,usize,u32)>{
 let t=ld(time)?;let n=times.len();
 let mut index=*last as usize;if index>=n{return Err("cached key index escapes the key array".into())}
 let mut t0=ld(times[index])?;
 if less(&t,&t0){index=0;t0=ld(times[0])?;}
 loop{
  let next=index+1;
  if next>=n{return Err("time after the last key: the native search reads the key past the array".into())}
  let k=ld(times[next])?;
  if !less(&k,&t){
   let u=x.div(&x.sub(&t,&t0)?,&x.sub(&k,&t0)?)?;
   *last=index as u32;return Ok((index,next,st(&u)?));
  }
  t0=k;index=next;
 }
}
///860890 for one float key array; the result is the interpolator's binary32 store.
fn float_at(keys:&Floats,time:u32,last:&mut u32,x:X)->Result<u32>{
 match keys{
  Floats::Single(v)=>Ok(*v),
  Floats::Linear(k)=>{let times:Vec<u32>=k.iter().map(|k|k[0]).collect();let(a,b,u)=segment(&times,time,last,x)?;
   //8B7CC0: (1 - u) * v0 + u * v1
   let u=ld(u)?;st(&x.add(&x.mul(&x.sub(&ld(ONE)?,&u)?,&ld(k[a][1])?)?,&x.mul(&u,&ld(k[b][1])?)?)?)}
  Floats::Quadratic(k)=>{let times:Vec<u32>=k.iter().map(|k|k[0]).collect();let(a,b,u)=segment(&times,time,last,x)?;
   //860BF0: 8B7CE0(u, v0, m0 = key a backward (+0x10), v1, m1 = key b forward (+0x0C))
   st(&hermite(u,k[a][1],k[a][3],k[b][1],k[b][2],x)?)}
 }
}
///8B7CE0(u, v0, m0, v1, m1): ((D u + C) u + m0) u + v0 with
///D = (m0 + m1) - 2(v1 - v0) and C = 3(v1 - v0) - (2 m0 + m1).
fn hermite(u:u32,v0:u32,m0:u32,v1:u32,m1:u32,x:X)->Result<Binary>{
 let(u,v0,m0,v1,m1)=(ld(u)?,ld(v0)?,ld(m0)?,ld(v1)?,ld(m1)?);
 let d=x.sub(&v1,&v0)?;
 let big_d=x.mul(&x.sub(&x.add(&m0,&m1)?,&x.add(&d,&d)?)?,&u)?;
 let c=x.sub(&x.mul(&d,&ld(THREE)?)?,&x.add(&x.add(&m0,&m0)?,&m1)?)?;
 x.add(&x.mul(&x.add(&x.mul(&x.add(&big_d,&c)?,&u)?,&m0)?,&u)?,&v0)
}
///861000 for position keys.
fn point_at(keys:&Points,time:u32,last:&mut u32,x:X)->Result<[u32;3]>{
 match keys{
  Points::Single(v)=>Ok(*v),
  Points::Linear(k)=>{let times:Vec<u32>=k.iter().map(|k|k[0]).collect();let(a,b,u)=segment(&times,time,last,x)?;
   //863D10: s = 1 - u; x keeps both products extended, y and z add stored products.
   let u=ld(u)?;let s=x.sub(&ld(ONE)?,&u)?;let(p,q)=(k[a],k[b]);
   let sx=x.mul(&s,&ld(p[1])?)?;let ux=x.mul(&u,&ld(q[1])?)?;
   let yz=|j:usize|->Result<u32>{let sp=ld(st(&x.mul(&s,&ld(p[j])?)?)?)?;let up=ld(st(&x.mul(&u,&ld(q[j])?)?)?)?;st(&x.add(&up,&sp)?)};
   Ok([st(&x.add(&ux,&sx)?)?,yz(2)?,yz(3)?])}
  Points::Quadratic(k)=>{let times:Vec<u32>=k.iter().map(|k|k.time).collect();let(a,_,u)=segment(&times,time,last,x)?;
   //861720: ((B u + A) u + m0) u + v0 with its binary32 spills per axis.
   let u=ld(u)?;let k=&k[a];let(av,bv,m,v)=(|j:usize|ld(k.a[j]),|j:usize|ld(k.b[j]),|j:usize|ld(k.backward[j]),|j:usize|ld(k.value[j]));
   let px=ld(st(&x.add(&x.mul(&u,&bv(0)?)?,&av(0)?)?)?)?;
   let qx=x.add(&ld(st(&x.mul(&px,&u)?)?)?,&m(0)?)?;
   let ox=st(&x.add(&x.mul(&qx,&u)?,&v(0)?)?)?;
   let py=x.add(&x.mul(&u,&bv(1)?)?,&av(1)?)?;
   let qy=ld(st(&x.add(&ld(st(&x.mul(&py,&u)?)?)?,&m(1)?)?)?)?;
   let oy=st(&x.add(&x.mul(&qy,&u)?,&v(1)?)?)?;
   let pz=x.add(&ld(st(&x.mul(&u,&bv(2)?)?)?)?,&av(2)?)?;
   let qz=ld(st(&x.add(&x.mul(&pz,&u)?,&m(2)?)?)?)?;
   let oz=st(&x.add(&ld(st(&x.mul(&qz,&u)?)?)?,&v(2)?)?)?;
   Ok([ox,oy,oz])}
 }
}

//---------------------------------------------------------------------------
// Quaternions: w, x, y, z.
fn quat(q:[u32;4])->Result<[Binary;4]>{Ok([ld(q[0])?,ld(q[1])?,ld(q[2])?,ld(q[3])?])}
///8B7F80(t, q0, q1): no per-call hemisphere test. d = ((y y' + x x') + w w')
///+ z' z (stored); k = 1 - d * 0.8227969 (ABD8F8); h(s) = (1 + ((k k) *
///0.5854922 (ABD8F4)) * (((s + s) - 3) s + 1)) s; t' = h(t) for t <= 0.5,
///else 1 - h(1 - t); component (q1 - q0) t' + q0 stored; then 8B7E20.
fn blend(t:u32,q0:[u32;4],q1:[u32;4],c:&Constants,x:X)->Result<[u32;4]>{
 let(a,b)=(quat(q0)?,quat(q1)?);
 let d=x.add(&x.add(&x.add(&x.mul(&a[2],&b[2])?,&x.mul(&a[1],&b[1])?)?,&x.mul(&a[0],&b[0])?)?,&x.mul(&b[3],&a[3])?)?;let d=ld(st(&d)?)?;
 let one=ld(ONE)?;let k=x.sub(&one,&x.mul(&d,&ld(0x3f52_a2d1)?)?)?;
 let h=|s:&Binary|->Result<Binary>{
  let bias=x.add(&x.mul(&x.sub(&x.add(s,s)?,&ld(THREE)?)?,s)?,&one)?;
  x.mul(&x.add(&x.mul(&x.mul(&x.mul(&k,&k)?,&ld(0x3f15_e2d1)?)?,&bias)?,&one)?,s)
 };
 let t=ld(t)?;
 let w=if !less(&ld(HALF)?,&t){h(&t)?}else{x.sub(&one,&h(&x.sub(&one,&t)?)?)?};
 let mut r=[0u32;4];for i in [3,2,1,0]{r[i]=st(&x.add(&x.mul(&x.sub(&b[i],&a[i])?,&w)?,&a[i])?)?;}
 normalize(r,c,x)
}
///8B7E20: n = ((w w + x x) + y y) + z z (stored); r = (n - C0) G1 + G0 with
///C0 = 0.9590660 (ABD8E4); refined r *= (r r n - C0) G1 + G0 when n <=
///0.9152120 (ABD8F0) and again when also n <= 0.6521197 (ABD8EC).
fn normalize(q:[u32;4],c:&Constants,x:X)->Result<[u32;4]>{
 let v=quat(q)?;
 let n=x.add(&x.add(&x.add(&x.mul(&v[0],&v[0])?,&x.mul(&v[1],&v[1])?)?,&x.mul(&v[2],&v[2])?)?,&x.mul(&v[3],&v[3])?)?;let n=ld(st(&n)?)?;
 let(c0,g0,g1)=(ld(0x3f75_8559)?,ld(c.g0)?,ld(c.g1)?);
 let mut r=x.add(&x.mul(&x.sub(&n,&c0)?,&g1)?,&g0)?;
 for limit in [0x3f6a_4b55u32,0x3f26_f151]{
  if less(&ld(limit)?,&n){break}
  let step=x.add(&x.mul(&x.sub(&x.mul(&x.mul(&r,&r)?,&n)?,&c0)?,&g1)?,&g0)?;r=x.mul(&r,&step)?;
 }
 Ok([st(&x.mul(&r,&v[0])?)?,st(&x.mul(&r,&v[1])?)?,st(&x.mul(&r,&v[2])?)?,st(&x.mul(&r,&v[3])?)?])
}
///870E80: this(a) * b, each component reduced in the instruction order and stored.
fn multiply(a:[u32;4],b:[u32;4],x:X)->Result<[u32;4]>{
 let(a,b)=(quat(a)?,quat(b)?);let m=|p:&Binary,q:&Binary|x.mul(p,q);
 let(w0,x0,y0,z0)=(&a[0],&a[1],&a[2],&a[3]);let(w1,x1,y1,z1)=(&b[0],&b[1],&b[2],&b[3]);
 let z=x.sub(&x.add(&x.add(&m(z1,w0)?,&m(x0,y1)?)?,&m(z0,w1)?)?,&m(y0,x1)?)?;
 let y=x.sub(&x.add(&x.add(&m(z0,x1)?,&m(w0,y1)?)?,&m(y0,w1)?)?,&m(z1,x0)?)?;
 let xx=x.sub(&x.add(&x.add(&m(x0,w1)?,&m(z1,y0)?)?,&m(x1,w0)?)?,&m(z0,y1)?)?;
 let w=x.sub(&x.sub(&x.sub(&m(w1,w0)?,&m(x1,x0)?)?,&m(y0,y1)?)?,&m(z0,z1)?)?;
 Ok([st(&w)?,st(&xx)?,st(&y)?,st(&z)?])
}
///4465A0: NiQuaternion -> NiMatrix3 (row-major words), with its binary32
///temporaries 2z, wx, wy, wz, xx, xy, xz and extended 2x, 2y, yy, yz, zz.
fn to_rotation(q:[u32;4],x:X)->Result<[u32;9]>{
 let[w,qx,qy,qz]=quat(q)?;let m=|a:&Binary,b:&Binary|x.mul(a,b);let s=|v:&Binary|->Result<Binary>{ld(st(v)?)};
 let x2=x.add(&qx,&qx)?;let y2=x.add(&qy,&qy)?;let z2=s(&x.add(&qz,&qz)?)?;
 let(wx,wy,wz)=(s(&m(&x2,&w)?)?,s(&m(&y2,&w)?)?,s(&m(&z2,&w)?)?);
 let(xx,xy,xz)=(s(&m(&x2,&qx)?)?,s(&m(&y2,&qx)?)?,s(&m(&z2,&qx)?)?);
 let(yy,yz,zz)=(m(&y2,&qy)?,m(&z2,&qy)?,m(&z2,&qz)?);
 let one=ld(ONE)?;
 Ok([st(&x.sub(&one,&x.add(&zz,&yy)?)?)?,st(&x.sub(&xy,&wz)?)?,st(&x.add(&xz,&wy)?)?,
     st(&x.add(&xy,&wz)?)?,st(&x.sub(&one,&x.add(&zz,&xx)?)?)?,st(&x.sub(&yz,&wx)?)?,
     st(&x.sub(&xz,&wy)?)?,st(&x.add(&wx,&yz)?)?,st(&x.sub(&one,&x.add(&yy,&xx)?)?)?])
}
///B31F60: the axis sequence of the nine Euler orders.
const EULER_ORDER:[[usize;3];9]=[[0,1,2],[0,2,1],[1,2,0],[1,0,2],[2,0,1],[2,1,0],[0,1,0],[1,2,1],[2,0,2]];
///862FE0: per axis angle (860890, 0 for an empty axis, last key in the
///Euler key +0x40), axis quaternions 871130 about B3206C/B32078/B32084, then
///q = A[order[i]] * q from the identity.
fn euler(order:u32,axes:&[Option<Floats>;3],time:u32,last:&mut [u32;3],x:X)->Result<[u32;4]>{
 let mut angle=[0u32;3];
 for a in 0..3{if let Some(k)=&axes[a]{angle[a]=float_at(k,time,&mut last[a],x)?}}
 let mut axis=[[0u32;4];3];for a in 0..3{axis[a]=angle_axis(angle[a],a,x)?;}
 let table=EULER_ORDER.get(order as usize).ok_or("Euler order outside the B31F60 table")?;
 let mut q=[ONE,0,0,0];for &i in table{q=multiply(axis[i],q,x)?;}
 Ok(q)
}
///871130(angle, unit axis): h = angle * 0.5; w = FCOS h (stored); the axis
///components are FSIN h times 1.0 or 0.0 (stored).
fn angle_axis(angle:u32,axis:usize,x:X)->Result<[u32;4]>{
 let h=x.mul(&ld(angle)?,&ld(HALF)?)?;
 let(sin,cos)=trig::sin_cos_f32(&h)?;
 let mut q=[cos,0,0,0];for i in 0..3{q[i+1]=if i==axis{sin}else{sin&0x8000_0000}}
 Ok(q)
}
///862170 for rotation keys.
fn rotation_at(keys:&Rotations,time:u32,last:&mut u32,euler_last:&mut [u32;3],c:&Constants,x:X)->Result<[u32;4]>{
 match keys{
  Rotations::Single(q)=>Ok(*q),
  Rotations::Euler{order,axes}=>euler(*order,axes,time,euler_last,x),
  Rotations::Linear(k)=>{let times:Vec<u32>=k.iter().map(|k|k[0]).collect();let(a,b,u)=segment(&times,time,last,x)?;
   //864120 -> 8B7F80(u, key a quaternion, key b quaternion)
   blend(u,[k[a][1],k[a][2],k[a][3],k[a][4]],[k[b][1],k[b][2],k[b][3],k[b][4]],c,x)}
 }
}

//---------------------------------------------------------------------------
///Start-up constants of 8B7E20: B45E38 = f32(1.000311 / sqrt(d)) (9E8380)
///and B45E3C = f32((-0.5 / (sqrt(d) * 0.9590660)) * 1.000311) (9E83A0), d
///the binary64 [ABD900] = 0.9590660 (3F758559 widened), computed by the C++
///static initializers at the start-up precision PC53.
#[derive(Clone,Debug)]
pub(crate) struct Constants{g0:u32,g1:u32}
impl Constants{
 pub(crate) fn startup()->Result<Self>{
  let x=X(Precision::Double53);let d=ld(0x3f75_8559)?;let root=d.sqrt(Precision::Double53).map_err(e)?;let c=ld(0x3f80_0a31)?;
  Ok(Self{g0:st(&x.div(&c,&root)?)?,g1:st(&x.mul(&x.div(&ld(0xbf00_0000)?,&x.mul(&root,&d)?)?,&c)?)?})
 }
}

///NiTimeController state the update reads and writes, constructor 89ED80
///values: clock +0x20/+0x24/+0x28, +0x2C scaled time, +0x40 force byte,
///flags +0x0C, last-key indices (NiKeyframeController +0x44/+0x48/+0x4C,
///NiAlphaController +0x44). +0x30 (compute scaled time) is the constructor's
///1; no loader stores it.
#[derive(Clone,Debug)]
pub(crate) struct State{pub clock:ControllerClock,pub scaled:u32,pub force:bool,pub flags:u16,pub last:[u32;3]}
#[derive(Clone,Debug)]
struct Keyframe{rotation:Option<Rotations>,position:Option<Points>,scale:Option<Floats>}

///Controller state of one model. The 89E9B0 result cache (B44630..B4463C,
///B32454) is kept per model: for controllers without the reverse bit a hit
///returns what the evaluation computes, so a cache shared with other models
///changes no result.
pub(crate) struct Runtime{pc:Precision,constants:Constants,cache:ClockCache,states:BTreeMap<usize,State>,keyframes:BTreeMap<usize,Keyframe>,floats:BTreeMap<usize,Option<Floats>>,euler_last:BTreeMap<usize,[u32;3]>}
impl std::fmt::Debug for Runtime{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.debug_struct("Runtime").field("pc",&self.pc).field("states",&self.states).finish_non_exhaustive()}}
impl Runtime{
 pub(crate) fn new(pc:Precision)->Result<Self>{Ok(Self{pc,constants:Constants::startup()?,cache:ClockCache::default(),states:BTreeMap::new(),keyframes:BTreeMap::new(),floats:BTreeMap::new(),euler_last:BTreeMap::new()})}
 pub(crate) fn state(&self,controller:usize)->Option<&State>{self.states.get(&controller)}
 #[cfg_attr(not(test),allow(dead_code))]
 pub(crate) fn euler_last(&self,data:usize)->Option<[u32;3]>{self.euler_last.get(&data).copied()}
 fn state_mut(&mut self,nif:&NifFile,c:usize)->Result<&mut State>{
  if !self.states.contains_key(&c){let flags=nif.objects[c].fields.u16("flags").ok_or("89F070 controller flags absent")?;
   self.states.insert(c,State{clock:ControllerClock::default(),scaled:UNSET,force:false,flags,last:[0;3]});}
  Ok(self.states.get_mut(&c).expect("state"))
 }
 fn clock(&mut self,nif:&NifFile,c:usize)->Result<Clock>{
  let flags=self.state_mut(nif,c)?.flags;let f=&nif.objects[c].fields;let w=|n:&str|f.f32(n).ok_or_else(||format!("89F070 controller {n} absent"));
  //The cache key (start, stop, raw, cycle) omits the reverse bit.
  if flags&0x10!=0{return Err("reverse-cycle controller: the shared 89E9B0 result cache does not key the reverse bit".into())}
  Ok(Clock{flags,frequency:F32(w("frequency")?),phase:F32(w("phase")?),start:F32(w("start")?),stop:F32(w("stop")?)})
 }
 fn evaluate(&mut self,nif:&NifFile,c:usize,time:u32)->Result<u32>{
  let clock=self.clock(nif,c)?;let pc=self.pc;let state=self.states.get_mut(&c).expect("state");
  Ok(state.clock.update(F32(time),clock,&mut self.cache,pc).map_err(|x|format!("89E9B0 controller clock: {x}"))?.0)
 }
 ///87C780 DontDoUpdate.
 fn dont_do_update(&mut self,nif:&NifFile,c:usize,time:u32)->Result<bool>{
  let state=self.state_mut(nif,c)?;
  if state.flags>>3&1==0{return Ok(true)}
  if equal(&ld(time)?,&ld(state.clock.last.0)?)&&!state.force{return Ok(true)}
  let scaled=self.evaluate(nif,c,time)?;let state=self.states.get_mut(&c).expect("state");
  //87C7B7 compares the unrounded 89E9B0 result with +0x2C; a stored result
  //equal to +0x2C does not decide that comparison.
  if equal(&ld(scaled)?,&ld(state.scaled)?){return Err("87C780 compares the unrounded 89E9B0 result with an equal stored scaled time".into())}
  state.scaled=scaled;state.force=false;Ok(false)
 }
 fn keyframe(&mut self,nif:&NifFile,data:usize)->Result<Keyframe>{
  if let Some(k)=self.keyframes.get(&data){return Ok(k.clone())}
  let o=&nif.objects[data];if o.class!="NiKeyframeData"{return Err(format!("NiKeyframeController data is {}",o.class))}
  let f=&o.fields;let x=X(self.pc);let count=|n:&str|f.u32(n).unwrap_or(0);
  let k=Keyframe{
   rotation:rotation_keys(count("rotation_count"),f.u32("rotation_type"),f.get("rotations"),x)?,
   position:point_keys(count("translation_count"),f.u32("translation_type"),f.get("translations"),x)?,
   scale:float_keys(count("scale_count"),f.u32("scale_type"),f.get("scales"))?,
  };
  self.keyframes.insert(data,k.clone());Ok(k)
 }
 ///866BE0(time, 1): each Property of the native list with a controller
 ///head (86A520), then the object's own chain.
 pub(crate) fn update_object(&mut self,nif:&NifFile,object:usize,properties:&[usize],time:u32,targets:&mut dyn Targets)->Result<()>{
  for &p in properties{for c in nif.controllers(p).map_err(|e|e.to_string())?{self.update(nif,c,time,targets)?;}}
  for c in nif.controllers(object).map_err(|e|e.to_string())?{self.update(nif,c,time,targets)?;}
  Ok(())
 }
 ///One controller's Update (vtable +0x3C).
 pub(crate) fn update(&mut self,nif:&NifFile,c:usize,time:u32,targets:&mut dyn Targets)->Result<()>{
  let x=X(self.pc);
  match nif.objects[c].class{
   "NiKeyframeController"=>{
    //8C0200
    if self.dont_do_update(nif,c,time)?{return Ok(())}
    let Some(data)=nif.linked(c,"data")else{return Ok(())};
    let keys=self.keyframe(nif,data)?;
    let scaled=self.evaluate(nif,c,time)?;self.states.get_mut(&c).expect("state").scaled=scaled;
    //+0x34 is dereferenced without a test.
    let target=nif.linked(c,"target").ok_or("8C0200 NiKeyframeController target is NULL")?;
    if let Some(r)=&keys.rotation{
     let mut euler=self.euler_last.get(&data).copied().unwrap_or([0;3]);
     let state=self.states.get_mut(&c).expect("state");
     let q=rotation_at(r,scaled,&mut state.last[0],&mut euler,&self.constants,x)?;
     if matches!(r,Rotations::Euler{..}){self.euler_last.insert(data,euler);}
     targets.local(target)?[..9].copy_from_slice(&to_rotation(q,x)?);
    }
    if let Some(p)=&keys.position{let state=self.states.get_mut(&c).expect("state");let v=point_at(p,scaled,&mut state.last[1],x)?;targets.local(target)?[9..12].copy_from_slice(&v);}
    if let Some(s)=&keys.scale{let state=self.states.get_mut(&c).expect("state");let v=float_at(s,scaled,&mut state.last[2],x)?;
     //8C02CE: below 0.0 stores 0.0; FABS clears the sign of -0.
     targets.local(target)?[12]=if less(&ld(v)?,&Binary::from_u32(0)){0}else{v&0x7fff_ffff};}
    Ok(())
   }
   "NiAlphaController"=>{
    //8C2740: a NULL target returns before DontDoUpdate.
    let Some(target)=nif.linked(c,"target")else{return Ok(())};
    if self.dont_do_update(nif,c,time)?{return Ok(())}
    if nif.objects[target].class!="NiMaterialProperty"{return Err(format!("8C2740 NiAlphaController target is {}",nif.objects[target].class))}
    let data=nif.linked(c,"data").ok_or("8C2740 NULL NiFloatData: 9091F0 returns no keys and 860890 reads them")?;
    let keys=match self.floats.get(&data){Some(k)=>k.clone(),None=>{let o=&nif.objects[data];if o.class!="NiFloatData"{return Err(format!("NiAlphaController data is {}",o.class))}
     let f=&o.fields;let k=float_keys(f.u32("key_count").unwrap_or(0),f.u32("key_type"),f.get("keys"))?;self.floats.insert(data,k.clone());k}};
    let keys=keys.ok_or("8C2740 NiFloatData without keys: 860890 reads the NULL key array")?;
    let state=self.states.get_mut(&c).expect("state");let scaled=state.scaled;
    let v=float_at(&keys,scaled,&mut state.last[0],x)?;
    targets.material(target)?[13]=v;Ok(())
   }
   other=>Err(format!("native {other}::Update is not ported")),
  }
 }
 ///872650 with the 8785D0 callback: the object's own chain, each Property's
 ///chain (native list order), then for an NiNode each effect's chain. A
 ///controller with flags bit 0 is reported ("Time Controller in %s not
 ///APP_TIME", line 0x64E) and becomes (flags & 0xFFFE) | 8; the metadata
 ///word +4 becomes each larger stop time.
 pub(crate) fn visit_metadata(&mut self,nif:&NifFile,object:usize,properties:&[usize],effects:&[usize],stop:&mut u32,diagnostic:&mut dyn FnMut(&str)->Result<()>)->Result<()>{
  let name=|i:usize|->String{nif.objects[i].fields.string("name").flatten().map(|b|b.split(|&c|c==0).next().unwrap_or(&[]).iter().map(|&c|c as char).collect()).unwrap_or_default()};
  let mut groups:Vec<(Vec<usize>,String)>=vec![(nif.controllers(object).map_err(|e|e.to_string())?,format!("object {}",name(object)))];
  for &p in properties{groups.push((nif.controllers(p).map_err(|e|e.to_string())?,format!("Property {} (for object {})",name(p),name(object))));}
  if aa_formats::nif::is_kind_of(nif.objects[object].class,"NiNode"){for &f in effects{groups.push((nif.controllers(f).map_err(|e|e.to_string())?,format!("Effect {} (for object {})",name(f),name(object))));}}
  for(chain,text)in groups{for c in chain{
   if self.state_mut(nif,c)?.flags&1!=0{diagnostic(&format!("Time Controller in {text} not APP_TIME"))?;let s=self.state_mut(nif,c)?;s.flags=(s.flags&0xfffe)|8;}
   let value=nif.objects[c].fields.f32("stop").ok_or("8785D0 stop time absent")?;
   if less(&ld(*stop)?,&ld(value)?){*stop=value}
  }}
  Ok(())
 }
}

mod trig{
 //!FSIN/FCOS of an x87 value, rounded to binary32. The true values come
 //!from fixed-point Taylor sums (2^-120 units, error below 2^-110) after a
 //!120-bit pi/2 reduction. The window allowed around them covers the x87
 //!instructions (66-bit pi reduction: |h| 2^-64, plus 1 ulp), a binary64
 //!libm (2^-52) and the later 24- or 53-bit FMUL rounding of 871130. Its
 //!binary32 rounding must not change over the window; otherwise the call
 //!is an explicit error.
 use aa_numeric::Binary;
 type Result<T>=std::result::Result<T,String>;
 const F:u32=120;
 ///pi/2 * 2^120, rounded to nearest.
 const PIO2:i128=0x1921fb54442d18469898cc51701b83a;
 const ONE:i128=1<<F;
 ///(a * b) >> F for |a|, |b| < 2^121 with |a b| < 2^246, via a 256-bit product.
 fn mul(a:i128,b:i128)->i128{
  let neg=(a<0)!=(b<0);let(a,b)=(a.unsigned_abs(),b.unsigned_abs());const M:u128=u64::MAX as u128;
  let(a1,a0,b1,b0)=(a>>64,a&M,b>>64,b&M);
  let(lo,m1,m2,hi)=(a0*b0,a1*b0,a0*b1,a1*b1);
  let(mid,c1)=m1.overflowing_add(m2);
  let(low,c2)=lo.overflowing_add(mid<<64);
  let high=hi+(mid>>64)+((c1 as u128)<<64)+c2 as u128;
  assert!(high>>119==0,"fixed-point product range");
  let r=((high<<(128-F))|(low>>F)) as i128;if neg{-r}else{r}
 }
 fn to_fixed(h:&Binary)->Result<i128>{
  let bits=h.to_x87_extended_bits().ok_or("FSIN/FCOS operand outside the extended domain")?;
  let neg=(bits>>79)&1!=0;let field=((bits>>64)&0x7fff) as i64;let sig=bits as u64 as u128;
  if sig==0{return Ok(0)}
  let shift=field-16383-63+F as i64;
  let m=if shift>=0{if shift>62{return Err("FSIN/FCOS operand beyond the ported range".into())}sig<<shift}else if shift>-128{sig>>(-shift)}else{0};
  let m=m as i128;Ok(if neg{-m}else{m})
 }
 fn series(r:i128)->(i128,i128){
  let r2=mul(r,r);let(mut s,mut c)=(0i128,0i128);
  let(mut ts,mut tc,mut n)=(r,ONE,0i128);
  while ts!=0||tc!=0{s+=ts;c+=tc;ts=-mul(ts,r2)/((n+2)*(n+3));tc=-mul(tc,r2)/((n+1)*(n+2));n+=2;}
  (s,c)
 }
 ///Binary32 rounding (nearest even) of a fixed-point value of at least 2^-94.
 fn round(v:i128)->Result<u32>{
  let neg=v<0;let m=v.unsigned_abs();
  let b=128-m.leading_zeros() as i64;if b<27{return Err("FSIN/FCOS result too small for the enclosure".into())}
  let mut exp=b-1-F as i64;
  let shift=(b-24) as u32;let mut q=m>>shift;let rem=m-(q<<shift);let half=1u128<<(shift-1);
  if rem>half||(rem==half&&q&1==1){q+=1;}if q>>24!=0{q>>=1;exp+=1;}
  if exp< -126{return Err("FSIN/FCOS binary32 subnormal result".into())}
  Ok(((neg as u32)<<31)|(((exp+127) as u32)<<23)|(q as u32&0x7f_ffff))
 }
 fn decided(v:i128,h:i128)->Result<u32>{
  let err=((v.unsigned_abs()>>50)+(h.unsigned_abs()>>60)+(1u128<<20)) as i128;
  let(lo,hi)=(round(v-err)?,round(v+err)?);
  if lo!=hi{return Err("FSIN/FCOS binary32 rounding is not decided within the x87 error bound".into())}Ok(lo)
 }
 ///(f32 sin h, f32 cos h).
 pub(super) fn sin_cos_f32(h:&Binary)->Result<(u32,u32)>{
  if h.is_zero(){let negative=h.to_x87_extended_bits().is_some_and(|b|b>>79&1!=0);return Ok((if negative{0x8000_0000}else{0},0x3f80_0000))}
  let x=to_fixed(h)?;
  let k=if x>=0{(x+PIO2/2)/PIO2}else{(x-PIO2/2)/PIO2};
  let r=x-k*PIO2;let(s,c)=series(r);
  let(sin,cos)=match k.rem_euclid(4){0=>(s,c),1=>(c,-s),2=>(-s,-c),_=>(-c,s)};
  Ok((decided(sin,x)?,decided(cos,x)?))
 }
 #[cfg(test)]mod tests{
  use super::*;
  #[test]fn sin_cos_agree_with_binary64_where_decided(){
   let mut checked=0;
   for i in 0..40000u32{
    let bits=0x3000_0000u32+(i.wrapping_mul(0x9e37_79b1)%0x1270_0000);let v=f32::from_bits(bits);
    for sign in [0,0x8000_0000]{let h=Binary::from_f32_bits(bits|sign).unwrap();let v=f32::from_bits(bits|sign) as f64;
     if let Ok((s,c))=sin_cos_f32(&h){assert_eq!(s,(v.sin() as f32).to_bits(),"sin {v}");assert_eq!(c,(v.cos() as f32).to_bits(),"cos {v}");checked+=1;}}
    let _=v;
   }
   assert!(checked>70000,"{checked}");
   assert_eq!(sin_cos_f32(&Binary::from_f32_bits(0).unwrap()).unwrap(),(0,0x3f80_0000));
  }
 }
}

#[cfg(test)]mod tests{
 //!Replays native-model-update.tsv (research/phase8-nif/native_model_update.py:
 //!the original NiStream load and NiAVObject::Update(0.0, 1) of the
 //!top-level object at PC24/PC53/PC64 under Unicorn) for every shipped NIF.
 use super::*;
 use aa_formats::nif::{self,is_kind_of};
 use aa_runtime::resource_model_bounds as math;
 use sha2::{Digest,Sha256};
 const MERGE_EPSILON:u32=897988541;
 struct Scene{local:BTreeMap<usize,[u32;13]>,material:BTreeMap<usize,[u32;14]>}
 impl Targets for Scene{
  fn local(&mut self,block:usize)->Result<&mut [u32;13]>{self.local.get_mut(&block).ok_or_else(||"controller target is not a NiAVObject".into())}
  fn material(&mut self,block:usize)->Result<&mut [u32;14]>{self.material.get_mut(&block).ok_or_else(||"controller target is not a NiMaterialProperty".into())}
 }
 fn words(f:&aa_formats::nif::Fields,name:&str)->Result<Vec<u32>>{f.f32s(name).map(|w|w.to_vec()).ok_or_else(||format!("{name} absent"))}
 ///Domain: NiNode/NiTriShape/NiTriStrips trees without collision objects,
 ///skins or effects, every NiAVObject of the file in the updated tree.
 fn replay(nif:&NifFile,pc:Precision)->Result<String>{
  let root=nif.top_level.first().copied().flatten().ok_or("no top-level object")?;
  let mut scene=Scene{local:BTreeMap::new(),material:BTreeMap::new()};
  for(i,o)in nif.objects.iter().enumerate(){
   if is_kind_of(o.class,"NiAVObject"){let f=&o.fields;let mut l=words(f,"rotation")?;l.extend(words(f,"translation")?);l.push(f.f32("scale").ok_or("scale")?);scene.local.insert(i,l.try_into().unwrap());}
   if o.class=="NiMaterialProperty"{let f=&o.fields;let mut w=Vec::new();for n in ["ambient","diffuse","specular","emissive"]{w.extend(words(f,n)?)}w.push(f.f32("glossiness").ok_or("gloss")?);w.push(f.f32("alpha").ok_or("alpha")?);scene.material.insert(i,w.try_into().unwrap());}
  }
  let mut runtime=Runtime::new(pc)?;
  let mut world:BTreeMap<usize,[u32;13]>=BTreeMap::new();let mut bound:BTreeMap<usize,[u32;4]>=BTreeMap::new();
  //Preorder: controllers, then the world transform (8A2C70), then children.
  let mut order=vec![];let mut parent:BTreeMap<usize,usize>=BTreeMap::new();let mut work=vec![root];
  while let Some(i)=work.pop(){
   let o=&nif.objects[i];if !matches!(o.class,"NiNode"|"NiTriShape"|"NiTriStrips"){return Err(format!("unported {}",o.class))}
   if nif.linked(i,"collision").is_some(){return Err("collision object".into())}
   if !nif.node_effects(i).map_err(|e|e.to_string())?.is_empty(){return Err("dynamic effects".into())}
   if is_kind_of(o.class,"NiGeometry")&&nif.linked(i,"skin_instance").is_some(){return Err("skin".into())}
   if order.contains(&i){return Err("aliased object".into())}order.push(i);
   let properties:Vec<usize>=nif.av_properties(i).map_err(|e|e.to_string())?.into_iter().map(|p|p.ok_or_else(||"NULL property".to_string())).collect::<Result<_>>()?;
   runtime.update_object(nif,i,&properties,0,&mut scene)?;
   let local=scene.local[&i];let w=match parent.get(&i){Some(p)=>math::compose_world(world[p],local,pc)?,None=>local};world.insert(i,w);
   let children:Vec<usize>=nif.node_children(i).map_err(|e|e.to_string())?.into_iter().flatten().collect();
   for &c in &children{parent.insert(c,i);}work.extend(children.into_iter().rev());
  }
  //World bounds: geometry 868020 (89FBF0 of the data bound); NiNode 866EB0
  //zeroes its radius, then copies the first nonzero child bound and merges
  //the later ones (89FAD0). The centre of a node no child bounds keeps the
  //constructor value.
  for &i in order.iter().rev(){
   let o=&nif.objects[i];
   if is_kind_of(o.class,"NiGeometry"){let d=nif.linked(i,"data").ok_or("geometry without data")?;let f=&nif.objects[d].fields;let c=words(f,"center")?;
    bound.insert(i,math::transform_sphere(&world[&i],[c[0],c[1],c[2],f.f32("radius").ok_or("radius")?],pc)?);continue}
   let mut b=[0u32;4];
   for c in nif.node_children(i).map_err(|e|e.to_string())?.into_iter().flatten(){let s=bound[&c];if s[3]&0x7fff_ffff==0{continue}if b[3]&0x7fff_ffff==0{b=s}else{math::merge_sphere(&mut b,s,MERGE_EPSILON,pc)?}}
   bound.insert(i,b);
  }
  let list=|w:&[u32]|format!("[{}]",w.iter().map(|v|v.to_string()).collect::<Vec<_>>().join(","));
  let mut out=String::from("[");
  for(i,o)in nif.objects.iter().enumerate(){
   if i>0{out.push(',')}
   let mut r=vec![format!("[\"class\",\"{}\"]",o.class)];
   if is_kind_of(o.class,"NiAVObject"){
    if !order.contains(&i){return Err("NiAVObject outside the updated tree".into())}
    r.push(format!("[\"flags\",{}]",o.fields.u16("flags").ok_or("flags")?));r.push(format!("[\"local\",{}]",list(&scene.local[&i])));
    r.push(format!("[\"world\",{}]",list(&world[&i])));r.push(format!("[\"bound\",{}]",list(&bound[&i])));
   }
   if o.class=="NiMaterialProperty"{r.push(format!("[\"material\",{}]",list(&scene.material[&i])));}
   if is_kind_of(o.class,"NiTimeController"){
    let s=runtime.state(i);let clock=s.map(|s|[s.clock.first.0,s.clock.last.0,s.clock.accumulated.0,s.scaled,s.force as u32]).unwrap_or([UNSET,UNSET,0,UNSET,0]);
    r.push(format!("[\"clock\",{}]",list(&clock)));
    let last=s.map(|s|s.last).unwrap_or([0;3]);
    if o.class=="NiKeyframeController"{r.push(format!("[\"last_keys\",{}]",list(&last)));}
    if o.class=="NiAlphaController"{r.push(format!("[\"last_keys\",{}]",list(&last[..1])));}
   }
   if o.class=="NiKeyframeData"&&o.fields.u32("rotation_type")==Some(4)&&o.fields.u32("rotation_count").unwrap_or(0)>0{
    r.push(format!("[\"euler_last_keys\",{}]",list(&runtime.euler_last(i).unwrap_or([0;3]))));
   }
   out.push('[');out.push_str(&r.join(","));out.push(']');
  }
  out.push(']');Ok(out)
 }
 #[test]#[ignore="needs the original Data.rwd (AA_GAME_ROOT) and the native fixture"]
 fn every_shipped_model_update_matches_the_native_update(){
  let here=std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
  let root=std::env::var_os("AA_GAME_ROOT").map(std::path::PathBuf::from).unwrap_or_else(||here.join("../../.."));
  let tsv=std::fs::read_to_string(here.join("../../research/phase8-nif/native-model-update.tsv")).expect("native update fixture");
  assert!(tsv.starts_with("path\tpc\tstatus\tdigest\tclock_cache\n"),"fixture schema");
  let mut archive=aa_formats::RwdArchive::open(root.join("Data.rwd")).expect("Data.rwd");
  let index:BTreeMap<String,usize>=archive.entries().iter().enumerate().map(|(i,e)|(e.path().to_string(),i)).collect();
  let only=std::env::var("NIF_UPDATE_ONLY").ok();
  let(mut matched,mut controllers_matched,mut differ)=(0,0,Vec::new());let mut unported:BTreeMap<String,usize>=BTreeMap::new();
  let mut cache:Option<(String,NifFile)>=None;
  for line in tsv.lines().skip(1){
   let c:Vec<&str>=line.split('\t').collect();let(path,pc,status,digest)=(c[0],c[1],c[2],c[3]);
   if only.as_deref().is_some_and(|o|o!=path){continue}
   if status!="ok"{*unported.entry(format!("native {status}")).or_default()+=1;continue}
   if cache.as_ref().is_none_or(|(p,_)|p!=path){let i=*index.get(path).unwrap_or_else(||panic!("{path} not in Data.rwd"));let mut data=Vec::new();archive.copy_entry(i,&mut data).unwrap();cache=Some((path.to_string(),nif::load(&data).unwrap()));}
   let nif=&cache.as_ref().unwrap().1;
   let pc=match pc{"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!("pc {pc}")};
   match replay(nif,pc){
    Ok(text)=>{if format!("{:x}",Sha256::digest(text.as_bytes()))==digest{matched+=1;if nif.objects.iter().any(|o|is_kind_of(o.class,"NiTimeController")){controllers_matched+=1}}else{
     if only.is_some(){std::fs::write(here.join("../../target/model-update-rust.json"),&text).unwrap();}differ.push(format!("{path} {pc:?}"))}}
    Err(reason)=>{*unported.entry(reason).or_default()+=1;}
   }
  }
  eprintln!("matched {matched} file/precision updates ({controllers_matched} with time controllers); outside the ported domain: {unported:#?}");
  assert!(differ.is_empty(),"{} updates differ from the native update:\n{}",differ.len(),differ.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
  assert!(matched>0);
 }
}
