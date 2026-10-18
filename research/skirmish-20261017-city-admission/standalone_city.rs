extern crate self as aa_actors;
#[path="../../crates/aa-actors/src/city_candidates.rs"]
pub mod city_candidates;
#[path="../../crates/aa-runtime/src/zone_city_admission.rs"]
pub mod zone_city_admission;
pub mod load_city_candidate {
use aa_numeric::{Binary,Precision};
use crate::zone_city_admission::ValueComparison;
pub fn compare_fields(fields:&aa_body::experience::ValueFields,pc:Precision)->Result<ValueComparison,String>{
    let mut infinity=None;
    for field in [fields.regiment,fields.wall,fields.combat,fields.component88].into_iter().flatten(){
        if field.0&0x7fff_ffff>0x7f80_0000{return Ok(ValueComparison::Unordered)}
        if field.0&0x7fff_ffff==0x7f80_0000{
            let negative=field.0>>31!=0;
            if infinity.is_some_and(|old|old!=negative){return Ok(ValueComparison::Unordered)}
            infinity=Some(negative);
        }
    }
    if let Some(negative)=infinity{return Ok(if negative{ValueComparison::Less}else{ValueComparison::Greater})}
    let value=aa_body::experience::actor_value(fields,pc).map_err(|e|e.to_string())?;
    Ok(match value.compare(&Binary::from_u32(0)){
        std::cmp::Ordering::Less=>ValueComparison::Less,
        std::cmp::Ordering::Equal=>ValueComparison::Equal,
        std::cmp::Ordering::Greater=>ValueComparison::Greater,
    })
}

}
