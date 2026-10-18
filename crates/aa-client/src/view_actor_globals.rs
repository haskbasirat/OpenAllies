//!One process-owned RGB seed at native B322A4/A8/AC. Both pinned PE images
//!initialize these writable-data cells to1.0. The bounded raw-reference audit
//!is not a native lifetime alias proof. No ported writer currently exposes a
//!mutable reference; a later concrete writer must update this SAME owner.
use anyhow::{Result,bail};
pub struct State{words:[u32;3]}
impl State{
    pub(super) fn from_process_image()->Self{Self{words:[0x3f800000;3]}}
    pub(super) fn word(&self,address:u32)->Result<u32>{
        let index=match address{0xb322a4=>0,0xb322a8=>1,0xb322ac=>2,_=>bail!("VActor RGB owner was asked for an unrelated global")};
        Ok(self.words[index])
    }
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn process_rgb_initializers_match_both_pinned_images_without_lifetime_claim(){
        let audit:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/white-global-audit.json")).unwrap();
        let owner=State::from_process_image();
        for image in audit["images"].as_array().unwrap(){
            for cell in image["cells"].as_array().unwrap(){
                let address=u32::from_str_radix(cell["va"].as_str().unwrap().trim_start_matches("0x"),16).unwrap();
                assert_eq!(owner.word(address).unwrap(),cell["word"].as_u64().unwrap()as u32);
                assert!(cell["writable"].as_bool().unwrap());
            }
        }
        assert!(owner.word(0xb43cf8).is_err());
    }
}
