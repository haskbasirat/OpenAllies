//!The actual World+275 CutsceneMode byte (4F8042 constructor0), not an
//!editor/session-mode inference. Kept in SAME load/liveWorld allocation.
//!Positive4F1080 SetCutsceneMode also changes actual Session/clock/speed owners
//!and remains required until that complete writer is bound; no raw setter.
pub struct State{cutscene_275:u8}
impl State{
 pub(crate) fn from_native_world_constructor()->Self{Self{cutscene_275:0}}
 pub fn current_cutscene_275(&self)->u8{self.cutscene_275}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn original_world_constructor_sets_cutscene_byte_without_session_inference(){
  let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-native-camera/native-world-camera-flags.json")).unwrap();
  assert_eq!(fixture["rows"].as_array().unwrap().len(),12);
  for row in fixture["rows"].as_array().unwrap(){assert_eq!(row["after"],u64::from(State::from_native_world_constructor().current_cutscene_275()));}
 }
}
