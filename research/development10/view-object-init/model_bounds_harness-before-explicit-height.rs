//!878630/878790 scalar geometry processing. Inputs are explicit world words
//!and unskinned source vertices; this mathematical API grants no load receipt.
use aa_numeric::{Binary,Precision};
use std::cmp::Ordering;
type Result<T>=std::result::Result<T,String>;
fn b(w:u32)->Result<Binary>{Binary::from_f32_bits(w).map_err(|e|e.to_string())}
fn s(v:Binary)->Result<u32>{v.store_f32().map_err(|e|e.to_string())}
fn mul(a:Binary,c:Binary,p:Precision)->Result<Binary>{a.mul(&c,p).map_err(|e|e.to_string())}
fn add(a:Binary,c:Binary,p:Precision)->Result<Binary>{a.add(&c,p).map_err(|e|e.to_string())}
fn sub(a:Binary,c:Binary,p:Precision)->Result<Binary>{a.sub(&c,p).map_err(|e|e.to_string())}
fn square(v:Binary,p:Precision)->Result<Binary>{mul(v.clone(),v,p)}
fn rotated(t:&[u32;13],v:[u32;3],p:Precision)->Result<[u32;3]>{
    let mut out=[0;3];
    for r in 0..3{
        let[i,j,k]=if r==0{[2,1,0]}else{[2,0,1]};
        out[r]=s(add(add(mul(b(t[r*3+i])?,b(v[i])?,p)?,mul(b(t[r*3+j])?,b(v[j])?,p)?,p)?,mul(b(t[r*3+k])?,b(v[k])?,p)?,p)?)?;
    }Ok(out)
}
///4516A0 spills scaledX/Y before translation, but keeps scaledZ extended;
///all three final coordinates are binary32. Used by footprint/selection.
pub fn transform_point(t:&[u32;13],v:[u32;3],p:Precision)->Result<[u32;3]>{
    let r=rotated(t,v,p)?;let mut out=[0;3];
    for i in 0..3{let scaled=mul(b(t[12])?,b(r[i])?,p)?;let scaled=if i<2{b(s(scaled)?)?}else{scaled};out[i]=s(add(scaled,b(t[9+i])?,p)?)?;}Ok(out)
}
///89FBF0 transforms the authored GeometryData sphere, independently of the
///vertex extrema used by87AA10. Y/Z scale products spill before translation.
pub fn transform_sphere(t:&[u32;13],sphere:[u32;4],p:Precision)->Result<[u32;4]>{
    let r=rotated(t,[sphere[0],sphere[1],sphere[2]],p)?;let mut out=[0;4];
    for i in 0..3{let scale=mul(b(t[12])?,b(r[i])?,p)?;let scale=if i==0{scale}else{b(s(scale)?)?};out[i]=s(add(scale,b(t[9+i])?,p)?)?;}
    out[3]=s(mul(b(sphere[3])?,b(t[12])?,p)?)?;Ok(out)
}
///89FAD0 updates one ordered native sphere owner. The small-distance current
///global is explicit; callers use its proved image-backed value, not a limit.
pub fn merge_sphere(current:&mut[u32;4],other:[u32;4],epsilon:u32,p:Precision)->Result<()>{
    let delta=[s(sub(b(current[0])?,b(other[0])?,p)?)?,s(sub(b(current[1])?,b(other[1])?,p)?)?,s(sub(b(current[2])?,b(other[2])?,p)?)?];
    let distance_squared=s(add(add(square(b(delta[0])?,p)?,square(b(delta[2])?,p)?,p)?,square(b(delta[1])?,p)?,p)?)?;
    let dr=sub(b(other[3])?,b(current[3])?,p)?;let stored_dr=s(dr.clone())?;
    let radius_squared=mul(dr,b(stored_dr)?,p)?;
    let contains=radius_squared.compare(&b(distance_squared)?)!=Ordering::Less;
    if b(stored_dr)?.compare(&b(0)?)==Ordering::Less{if contains{return Ok(())}}
    else if contains{*current=other;return Ok(())}
    let distance=b(distance_squared)?.sqrt(p).map_err(|e|e.to_string())?;
    if distance.compare(&b(epsilon)?)==Ordering::Greater{
        let ratio=sub(distance.clone(),b(stored_dr)?,p)?.div(&add(distance.clone(),distance.clone(),p)?,p).map_err(|e|e.to_string())?;
        for i in 0..3{let product=mul(b(delta[i])?,ratio.clone(),p)?;let product=if i==0{product}else{b(s(product)?)?};current[i]=s(add(product,b(other[i])?,p)?)?;}
    }
    current[3]=s(mul(add(add(distance,b(current[3])?,p)?,b(other[3])?,p)?,b(0x3f000000)?,p)?)?;Ok(())
}
///5A96B2 computes the named SelectionBox's extents. It deliberately uses
///world rotation+translation without world scale, as the actual instructions.
///The caller owns the separate native parent unlink after these field stores.
pub fn selection_geometry(t:&[u32;13],vertices:&[[u32;3]],p:Precision)->Result<[u32;6]>{
    let mut out=[0x7f7fffff,0x7f7fffff,0x7f7fffff,0xff7fffff,0xff7fffff,0xff7fffff];
    for &v in vertices{let r=rotated(t,v,p)?;for i in 0..3{let value=add(b(r[i])?,b(t[9+i])?,p)?;let value=if i==0{value}else{b(s(value)?)?};if value.compare(&b(out[i])?)==Ordering::Less{out[i]=s(value.clone())?;}if value.compare(&b(out[3+i])?)==Ordering::Greater{out[3+i]=s(value)?;}}}Ok(out)
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Bounds{pub radial:u32,pub min_z:u32,pub max_z:u32,pub footprint:[u32;3]}
impl Bounds{
    pub fn empty_processing()->Self{Self{radial:0,min_z:0x7f7fffff,max_z:0xff7fffff,footprint:[0;3]}}
    pub fn include_geometry(&mut self,t:&[u32;13],vertices:&[[u32;3]],hidden:bool,p:Precision)->Result<()>{
        if hidden{return Ok(())}
        let mut radius_squared=s(square(b(self.radial)?,p)?)?;
        for &v in vertices{
            let r=rotated(t,v,p)?;
            //878630 spills X and scaledZ, while Y stays extended until its
            //square. Comparing only already-rounded XYZ would change results.
            let x=b(s(add(b(t[9])?,mul(b(t[12])?,b(r[0])?,p)?,p)?)?)?;
            let y=add(b(t[10])?,mul(b(t[12])?,b(r[1])?,p)?,p)?;
            let z=add(b(t[11])?,b(s(mul(b(t[12])?,b(r[2])?,p)?)?)?,p)?;
            if z.compare(&b(self.max_z)?)==Ordering::Greater{self.max_z=s(z.clone())?;}
            if z.compare(&b(self.min_z)?)==Ordering::Less{self.min_z=s(z)?;}
            let squared=add(square(y,p)?,square(x,p)?,p)?;
            if squared.compare(&b(radius_squared)?)==Ordering::Greater{
                self.radial=s(squared.sqrt(p).map_err(|e|e.to_string())?)?;radius_squared=s(squared)?;
            }
        }Ok(())
    }
    pub fn include_footprint(&mut self,t:&[u32;13],vertices:&[[u32;3]],hidden:bool,threshold:u32,p:Precision)->Result<()>{
        if hidden{return Ok(())}
        let mut radius_squared=s(square(b(self.footprint[0])?,p)?)?;
        for &v in vertices{
            let w=transform_point(t,v,p)?;
            if sub(b(w[2])?,b(self.min_z)?,p)?.compare(&b(threshold)?)!=Ordering::Less{continue}
            let squared=add(square(b(w[1])?,p)?,square(b(w[0])?,p)?,p)?;
            if squared.compare(&b(radius_squared)?)==Ordering::Greater{
                self.footprint[0]=s(squared.sqrt(p).map_err(|e|e.to_string())?)?;radius_squared=s(squared)?;
            }
            for i in 0..2{let absolute=w[i]&0x7fffffff;if b(absolute)?.compare(&b(self.footprint[1+i])?)==Ordering::Greater{self.footprint[1+i]=absolute;}}
        }Ok(())
    }
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn original_authored_spheres_and_scale_free_selection_extents_all_precision_modes(){
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-model-spheres.json")).unwrap();
        let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),150);
        for row in rows{
            let pc=match row["precision"].as_str().unwrap(){"Single"=>Precision::Single24,"Double"=>Precision::Double53,"Extended"=>Precision::Extended64,_=>unreachable!()};
            let output=match row["mode"].as_str().unwrap(){
                "transform"=>{let a=serde_json::from_value(row["a"].clone()).unwrap();let t=serde_json::from_value(row["b"].clone()).unwrap();serde_json::json!(transform_sphere(&t,a,pc).unwrap())},
                "merge"=>{let mut a=serde_json::from_value::<[u32;4]>(row["a"].clone()).unwrap();let other=serde_json::from_value(row["b"].clone()).unwrap();merge_sphere(&mut a,other,fixture["merge_epsilon"].as_u64().unwrap()as u32,pc).unwrap();serde_json::json!(a)},
                "selection"=>{let v=serde_json::from_value::<Vec<[u32;3]>>(row["a"].clone()).unwrap();let t=serde_json::from_value(row["b"].clone()).unwrap();serde_json::json!(selection_geometry(&t,&v,pc).unwrap())},_=>unreachable!()};
            assert_eq!(output,row["output"],"{pc:?} {} {} {}",row["mode"],row["a"],row["b"]);
        }
    }
    #[test]fn original_geometry_radius_z_and_footprint_all_precision_modes(){
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-model-bounds.json")).unwrap();
        let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),192);
        for row in rows{
            let pc=match row["precision"].as_str().unwrap(){"Single"=>Precision::Single24,"Double"=>Precision::Double53,"Extended"=>Precision::Extended64,_=>unreachable!()};
            let t=serde_json::from_value::<[u32;13]>(row["input"]["transform"].clone()).unwrap();
            let v=serde_json::from_value::<Vec<[u32;3]>>(row["input"]["vertices"].clone()).unwrap();
            let hidden=row["input"]["hidden"].as_bool().unwrap();let mut out=Bounds::empty_processing();
            out.include_geometry(&t,&v,hidden,pc).unwrap();
            out.include_footprint(&t,&v,hidden,fixture["threshold"].as_u64().unwrap()as u32,pc).unwrap();
            assert_eq!(serde_json::json!({"radial":out.radial,"min_z":out.min_z,"max_z":out.max_z,"footprint":out.footprint}),row["output"],"{pc:?} {}",row["input"]);
        }
    }
}
