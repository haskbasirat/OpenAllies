//The selected factory is current production source. Support crates are the
//retained warm build; this does not certify a whole current netcode binary.
mod bootstrap {
    use aa_netcode::{Error, Result};
    use aa_netcode::lobby::{GameOptions, LaunchState};
    use aa_netcode::local_bootstrap::{constructor_launch_state, launch};
    use std::sync::Arc;
    fn missing(message: impl Into<String>) -> Error { Error::Protocol(message.into()) }
    #[path="../../../../crates/aa-netcode/src/local_selected_map_confirm.rs"]
    pub mod selected;
}
