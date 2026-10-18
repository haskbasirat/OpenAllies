// Verbatim production function extraction; no shipping-source edits.
use serde_json::Value;
type Result<T>=std::result::Result<T,String>;
fn finite(bits: u32) -> Result<f32> {
    let value = f32::from_bits(bits);
    if value.is_finite() {
        Ok(value)
    } else {
        Err("replacement containment requires the reached finite source field".into())
    }
}
///Exact46D8F0 finite-field branches. `container` is replacement+38 and
///`item` is original+38. The native lower-kind recursive result is negated;
///the rect/circle branch calls444420 on ITEM, not on the container.
fn contains(container: &[u32; 6], item: &[u32; 6]) -> Result<bool> {
    if item[0] > container[0] {
        return Ok(!contains(item, container)?);
    }
    match container[0] {
        0 => Ok(true),
        1 => {
            if item[0] == 0 {
                Ok(true)
            } else {
                Ok(finite(container[1])? >= finite(item[1])?)
            }
        }
        2 | 3 => {
            if item[0] <= 1 {
                //444420(item) returns item.radius for these two kinds.
                let extent = finite(item[1])?;
                Ok(extent >= finite(item[1])?)
            } else {
                let x = finite(container[4])?;
                if finite(item[4])? > x {
                    return Ok(false);
                }
                let y = finite(container[5])?;
                Ok(finite(item[5])? <= y)
            }
        }
        _ => Ok(false),
    }
}
///ActorData ctor4624B0 installs A05540; its+3C is real4C9FA0(FALSE).
///It does NOT select the other mask just because the definition is an Actor.
const STANDARD_ACTOR_MASK: u32 = 0xa74c00;
fn fixed_pair(
    original: u32,
    replacement: u32,
    original_shape: &[u32; 6],
    replacement_shape: &[u32; 6],
    virtual_true: bool,
) -> Result<u32> {
    if !contains(replacement_shape, original_shape)? {
        return Err("4BC5E0 reached required containment diagnostic before flag comparison".into());
    }
    let mask = if virtual_true {
        0xa34c00
    } else {
        STANDARD_ACTOR_MASK
    };
    if original & mask != replacement & mask {
        return Err("4BC5E0 reached warning and mutable replacement-flag storage; unchanged source projection refused".into());
    }
    Ok(original)
}
fn main(){
 let fixture:Value=serde_json::from_slice(&std::fs::read("research/skirmish-20261017-placeholder-flags/native-placeholder-flags.json").unwrap()).unwrap();
 let mut count=0;
 for row in fixture["rows"].as_array().unwrap().iter().chain(fixture["actual"].as_array().unwrap()) {
  let c=&row["input"];let n=&row["native"];
  let words=|key:&str|->[u32;6]{c[key].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect::<Vec<_>>().try_into().unwrap()};
  let original=c["subject_flags"].as_u64().unwrap()as u32;
  let result=fixed_pair(original,c["target_flags"].as_u64().unwrap()as u32,&words("subject_shape"),&words("target_shape"),c["virtual_true"].as_bool().unwrap());
  if n["outcome"]=="complete"&&n["events"].as_array().unwrap().is_empty(){assert_eq!(result.unwrap(),n["flags"].as_u64().unwrap()as u32)}
  else if n["outcome"]=="containment"{assert!(result.unwrap_err().contains("containment"));assert_eq!(n["flags"].as_u64().unwrap()as u32,original)}
  else {assert!(result.unwrap_err().contains("mutable"));if c["stop_warning"].as_bool().unwrap(){assert_eq!(n["flags"].as_u64().unwrap()as u32,original)}}
  count+=1;
 }
 assert_eq!(count,294);println!("exact production finite/contains/fixed_pair extraction replayed294 native rows");
}
