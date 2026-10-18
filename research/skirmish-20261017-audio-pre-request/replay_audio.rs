//Exact production core + exact tests, linked to existing serde_json only.
//The runtime source/manager adapter is not copied or stubbed by this host.
#![allow(dead_code)]
mod data_pre_request {
    #[path="E:/Games/Axis and Alies/rust-rebuild/crates/aa-runtime/src/data_pre_request_path.rs"]
    pub mod path;
}
mod audio_data_pre_request {
    use std::collections::BTreeMap;
    type Result<T>=std::result::Result<T,String>;
    #[path="E:/Games/Axis and Alies/rust-rebuild/crates/aa-runtime/src/audio_data_request_core.rs"]
    mod core;
    #[path="E:/Games/Axis and Alies/rust-rebuild/crates/aa-runtime/src/audio_data_pre_request_tests.rs"]
    mod tests;
}
