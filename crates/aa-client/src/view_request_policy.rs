//! Application+2A0's graphics pre-request policy, sourced from current physical
//! RAM.419701 compares rounded physical MiB with700.6F7E90's original wide-RAM
//! path shifts an old32-bit field; the64-bit backend intentionally derives MiB
//! from the actual u64 query instead. This is no allocation/VRAM budget or cap.
use anyhow::{Context,Result,ensure};
pub struct State {physical_total_bytes:u64,rounded_mib:u64,current:u8,sampled_unix_ms:u64}
impl State {
    ///The application retains this same current owner. It is not a per-view
    ///chosen bool, a sealed historical hardware byte or an activation token.
    pub fn from_application_initialize()->Result<Self>{
        let observed=aa_resources::query_memory().context("actual application physical-memory query")?;
        observed.validate()?;
        let rounded_mib=rounded_mib(observed.physical_total_bytes)?;
        Ok(Self{physical_total_bytes:observed.physical_total_bytes,rounded_mib,current:u8::from(rounded_mib<700),sampled_unix_ms:observed.sampled_unix_ms})
    }
    pub fn current(&self)->u8{self.current}
    pub fn physical_total_bytes(&self)->u64{self.physical_total_bytes}
    pub fn rounded_physical_mib(&self)->u64{self.rounded_mib}
    pub fn sampled_unix_ms(&self)->u64{self.sampled_unix_ms}
}
fn rounded_mib(bytes:u64)->Result<u64>{ensure!(bytes!=0,"physical RAM observation is zero");Ok((bytes>>20)+u64::from(bytes&0xfffff!=0))}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn native_legacy_projection_and_comparison_all_pc(){
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-renderer-mode/native-request-policy.json")).unwrap();
        for row in fixture["rows"].as_array().unwrap(){let c=&row["input"];let total=c["total"].as_u64().unwrap();let prior=c["prior_mib_word"].as_u64().unwrap()as u32;
            let legacy=if total>0x7fffffff{prior>>20}else{((total+0xfffff)>>20)as u32};
            assert_eq!(row["output"]["mib_word"],legacy);assert_eq!(row["output"]["application_2a0"],u8::from(legacy<700));
            if total<=0x7fffffff{assert_eq!(rounded_mib(total).unwrap(),u64::from(legacy));}
        }
    }
    #[test]fn modern_physical_memory_uses_full_u64_without_native_saturation(){
        for (bytes,mib,low)in [(699<<20,699,true),((699<<20)+1,700,false),(700<<20,700,false),(8u64<<30,8192,false),(1u64<<50,1u64<<30,false),(u64::MAX,1u64<<44,false)]{
            assert_eq!(rounded_mib(bytes).unwrap(),mib);assert_eq!(mib<700,low);
        }
        assert!(rounded_mib(0).is_err());
    }
    #[test]#[cfg(windows)]fn actual_application_query_binds_the_current_hardware_observation(){
        let state=State::from_application_initialize().unwrap();assert!(state.physical_total_bytes()>0);assert!(state.sampled_unix_ms()>0);
        assert_eq!(state.rounded_physical_mib(),rounded_mib(state.physical_total_bytes()).unwrap());assert_eq!(state.current(),u8::from(state.rounded_physical_mib()<700));
    }
}
