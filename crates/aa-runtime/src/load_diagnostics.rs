//!The application B3CC88 CString and B3CC8C/90 list, distinct from current
//!GWorld+9C/A0 warnings.4F7910 clears these before the fresh successful load.
//!Only4FBCE0 copies World diagnostics into this list; that producer is reached
//!by the audited Resource/task failure branches, never a successful task.
#[derive(Debug)]
pub(crate) struct State {
    resource_name: Vec<u16>,
    warnings: std::collections::VecDeque<Vec<u16>>,
    cleared_for_load: bool,
}
impl State {
    ///B3CC8C/90 are PE zero-tail words. CRT9CD920 constructs the separate
    ///B3CC88 CString via405B10. This is one actual application owner.
    pub(crate) fn from_fresh_application() -> Self {
        Self {
            resource_name: Vec::new(),
            warnings: std::collections::VecDeque::new(),
            cleared_for_load: false,
        }
    }
    ///4F7910→401170 and4F5EF0. Neither creates an invented warning payload.
    pub(crate) fn clear_4f7910(&mut self) {
        self.resource_name.clear();
        while self.warnings.pop_front().is_some() {}
        self.cleared_for_load = true;
    }
    ///4F7930's actual empty-list branch returns a new empty CString before
    ///localization/formatting. Only genuine successful tasks may reach this
    ///runtime facade; failed resource/catch branches remain unsupported.
    pub(crate) fn success_text_4f7930(&self) -> Result<Vec<u16>, String> {
        if !self.cleared_for_load {
            return Err("actual4F7910 diagnostics clear has not occurred".into());
        }
        if !self.warnings.is_empty() {
            return Err("4F7930 positive global warning formatting is unbound".into());
        }
        Ok(Vec::new())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_global_diagnostics_clear_and_empty_reader() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/development10/world-ready-7757f0/native-world-success.json"
        ))
        .unwrap();
        assert_eq!(fixture["diagnostics_clear"].as_array().unwrap().len(), 6);
        for section in fixture["zero_tails"].as_array().unwrap() {
            assert!(
                section["raw_size"].as_u64().unwrap() <= section["word_offset"].as_u64().unwrap()
            );
        }
        for row in fixture["diagnostics_clear"].as_array().unwrap() {
            let mut state = State::from_fresh_application();
            assert!(state.success_text_4f7930().is_err());
            //Explicit laboratory prior allocations; native proof uses actual
            //4F97B0 CString/list nodes before4F7910 clears them.
            state.resource_name = "previous resource".encode_utf16().collect();
            state
                .warnings
                .extend(["first", "second"].map(|v| v.encode_utf16().collect::<Vec<u16>>()));
            state.clear_4f7910();
            assert!(state.resource_name.is_empty());
            assert!(state.warnings.is_empty());
            assert_eq!(
                serde_json::json!(state.success_text_4f7930().unwrap()),
                row["text"]
            );
            assert_eq!(row["head"], 0);
            assert_eq!(row["tail"], 0);
        }
    }
}
