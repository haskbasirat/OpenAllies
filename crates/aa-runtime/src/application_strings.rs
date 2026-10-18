//! The fresh application's CRT-owned default CStrings atB43B40/B43B48.
//!9E7400 calls405B10→4013E0→401380, requesting the type2 StringManager's
//!empty allocation.9EF100 destroys it at process exit. This is not a missing
//!localization fallback invented per player or a mutable profile string.
#[derive(Debug)]
pub struct State {
    default_b43b40: Vec<u16>,
    default_b43b48: Vec<u16>,
}
impl State {
    ///Only the actual fresh application composition mints the CRT projection.
    pub(crate) fn from_native_crt_constructor() -> Self {
        Self {
            default_b43b40: Vec::new(),
            default_b43b48: Vec::new(),
        }
    }
    pub fn default_b43b48(&self) -> &[u16] {
        &self.default_b43b48
    }
    ///9E73C0→4013E0→401380 constructs this separate empty CString.7757F0
    ///copies its current contents to Session150 before checking SessionE4.
    pub fn default_b43b40(&self) -> &[u16] {
        &self.default_b43b40
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_default_cstring_constructor_replay() {
        let fixture:serde_json::Value=serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-local-bootstrap/implicit-ai/native-default-string.json"
        )).unwrap();
        assert_eq!(fixture["passed"], true);
        for row in fixture["cases"].as_array().unwrap() {
            let owner = State::from_native_crt_constructor();
            assert_eq!(
                serde_json::to_value(owner.default_b43b48()).unwrap(),
                row["value"]
            );
            assert_eq!(row["atexit"], 0x9ef100u32);
        }
    }
    #[test]
    fn native_film_cstring_constructor_replay() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/development10/world-ready-7757f0/native-application-film-string.json"
        ))
        .unwrap();
        assert_eq!(fixture["passed"], true);
        assert_eq!(fixture["cases"].as_array().unwrap().len(), 18);
        for row in fixture["cases"].as_array().unwrap() {
            let owner = State::from_native_crt_constructor();
            assert_eq!(
                serde_json::to_value(owner.default_b43b40()).unwrap(),
                row["value"]
            );
            assert_eq!(row["atexit"], 0x9ef0e0u32);
        }
    }
}
