//! The current GameSpy manager+BC byte, owned once by the fresh application.
//! This is an explicit constructor-field projection, never a completed manager,
//! worker, Internet-account, network-readiness or application-init capability.

/// A typed present manager projection. Other manager words and all worker/
///service lifetimes remain unbound; presence must not be used as ready=true.
#[derive(Debug)]
pub struct State {
    internet_bc: u8,
}
impl State {
    /// Actual787FE0 publishes a nonnull owner at78800F and sets+BC=BL at
    ///7880EA, after788015 zeroes EBX. This bounded projection assumes the
    ///preceding successful constructor dependencies, which are NOT certified
    ///complete by this type. Only the existing fresh app factory can mint it.
    pub(crate) fn from_fresh_constructor_field_projection() -> Self {
        Self { internet_bc: 0 }
    }
    /// Current field read for770280. It is neither a passed per-player flag nor
    ///a null-manager shortcut. The reference itself denotes the same owner.
    pub fn internet_enabled_bc(&self) -> bool {
        self.internet_bc != 0
    }
    pub fn current_byte_bc(&self) -> u8 {
        self.internet_bc
    }
    /// Actual standalone selector7876F3 writes0 before allocating/enqueuing
    ///the785560 functor. Keep this write in the same application owner.
    pub(crate) fn select_standalone_prefix(&mut self) {
        self.internet_bc = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_present_constructor_field_and_current_selection_clear_replay() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/development10/standalone-network/native-manager-gate.json"
        ))
        .unwrap();
        assert_eq!(fixture["rows"].as_array().unwrap().len(), 48);
        for row in fixture["rows"].as_array().unwrap() {
            let mut state = if row["kind"] == "constructor" {
                let output = &row["output"];
                assert_eq!(output["present"], true);
                assert_eq!(output["alias"], true);
                assert_eq!(output["global_slot"], true);
                assert_eq!(output["vtable"], 0xa982f4u32);
                assert_eq!(output["later_c4"], 0xa5);
                State::from_fresh_constructor_field_projection()
            } else {
                //Explicit laboratory preexisting byte only. There is no raw
                //mutable constructor in the public production API.
                State {
                    internet_bc: row["initial_bc"].as_u64().unwrap() as u8,
                }
            };
            if row["kind"] == "select_standalone" {
                state.select_standalone_prefix();
            }
            assert_eq!(
                state.current_byte_bc() as u64,
                row["output"]["bc"].as_u64().unwrap()
            );
            assert!(!state.internet_enabled_bc());
        }
    }
}
