use aa_runtime::combat_source_receipt as receipt;
use aa_rules::sources::sha256;
use serde_json::Value;
type Result<T>=std::result::Result<T,String>;
const REFERENCE: &[u8] = include_bytes!(
    "E:/Games/Axis and Alies/rust-rebuild/research/skirmish-20261017-view-grid/hardware-grid-seed-reference.json"
);
pub const REFERENCE_SHA256: &str =
    "64039c63149ed59c3698fa7b6fdabd40f901c5ae861d12ff1054cdb60e7f671b";
const CAPTURES: [&[u8]; 2] = [
    include_bytes!("E:/Games/Axis and Alies/rust-rebuild/research/skirmish-20261017-view-grid/capture-02/capture.json"),
    include_bytes!("E:/Games/Axis and Alies/rust-rebuild/research/skirmish-20261017-view-grid/capture-03/capture.json"),
];
const ADDRESSES: [u32; 5] = [0xb43cf8, 0xb43cfc, 0xb43d00, 0xb43dbc, 0xb43dc0];

fn parse(bytes: &[u8]) -> Result<Value> {
    serde_json::from_slice(bytes).map_err(|e| e.to_string())
}
fn verified_words() -> Result<[u32; 5]> {
    if sha256(REFERENCE) != REFERENCE_SHA256 {
        return Err("sealed VGrid seed reference digest differs".into());
    }
    verify_document(&parse(REFERENCE)?, CAPTURES)
}
fn verify_document(reference: &Value, captures: [&[u8]; 2]) -> Result<[u32; 5]> {
    if reference["schema"] != 1
        || reference["native_exe_sha256"] != receipt::EXE_SHA256
        || reference["source_identities_sha256"] != receipt::SOURCES_SHA256
        || reference["all_five_words_watched"] != true
        || reference["addresses"] != serde_json::json!(ADDRESSES)
    {
        return Err("VGrid hardware reference target/word coverage differs".into());
    }
    let words: [u32; 5] =
        serde_json::from_value(reference["words"].clone()).map_err(|e| e.to_string())?;
    let summaries = reference["captures"]
        .as_array()
        .ok_or("VGrid capture summaries absent")?;
    if summaries.len() != 2 {
        return Err("VGrid capture pair absent".into());
    }
    let mut covered = [false; 5];
    for (raw, summary) in captures.into_iter().zip(summaries) {
        if summary["capture_sha256"] != sha256(raw) {
            return Err("VGrid capture digest differs".into());
        }
        let capture = parse(raw)?;
        if capture["schema"] != 2
            || capture["completed_grid_constructor"] != true
            || capture["completed_source_return"] != true
            || capture["completed_session_constructor"] != true
            || !capture["original_error"].is_null()
            || capture["original_archives_preserved"] != true
            || capture["folder_redirection_observed"] != true
            || capture["fp_state_never_written_by_debugger"] != true
            || capture["temporary_process_only_breakpoints"] != true
            || capture["cleanup"]["wait_result"] != 0
            || capture["cleanup"]["exit_code"] != 0
            || capture["cleanup"]["exit_process_event_observed"] != true
            || !capture["cleanup"]["errors"]
                .as_array()
                .is_some_and(Vec::is_empty)
            || capture["seed_addresses"] != serde_json::json!(ADDRESSES)
            || !capture["observed_native_grid_seed_writes"]
                .as_array()
                .is_some_and(Vec::is_empty)
        {
            return Err("VGrid native trace did not complete its exact observed prefix".into());
        }
        for (name, expected) in [
            ("AA.exe", receipt::EXE_SHA256),
            ("D3D9.dll", receipt::RENDERER_SHA256),
            ("dgVoodoo.conf", receipt::RENDERER_CONFIG_SHA256),
        ] {
            if capture["runtime_pins"][name] != expected {
                return Err("VGrid captured runtime differs".into());
            }
        }
        let snapshots = capture["global_captures"]
            .as_array()
            .ok_or("VGrid snapshots absent")?;
        let entries: Vec<_> = snapshots
            .iter()
            .filter(|r| r["label"] == "grid_ctor_entry")
            .collect();
        let returned: Vec<_> = snapshots
            .iter()
            .filter(|r| r["label"] == "grid_ctor_return")
            .collect();
        if entries.len() != 1 || returned.len() != 1 {
            return Err("VGrid exact constructor call absent".into());
        }
        let (entry, returned) = (entries[0], returned[0]);
        if entry["eip"] != 0x430290
            || returned["eip"] != 0x43f124
            || entry["tid"] != returned["tid"]
            || entry["grid_pointer"] != entry["ecx"]
            || entry["grid_pointer"] != returned["eax"]
            || snapshots
                .iter()
                .any(|r| r["seed_words"] != serde_json::json!(words))
        {
            return Err("VGrid reached constructor receiver/seed words differ".into());
        }
        let indexes: [usize; 4] = serde_json::from_value(capture["watched_word_indices"].clone())
            .map_err(|e| e.to_string())?;
        let watches: Vec<_> = indexes
            .iter()
            .map(|i| {
                ADDRESSES
                    .get(*i)
                    .copied()
                    .ok_or("VGrid watch index invalid")
            })
            .collect::<std::result::Result<_, _>>()?;
        for index in indexes {
            covered[index] = true;
        }
        let arms = capture["watch_arms"]
            .as_array()
            .ok_or("VGrid thread watches absent")?;
        if arms.is_empty()
            || arms.iter().any(|r| {
                r["addresses"] != serde_json::json!(watches)
                    || r["requested_dr7"] != r["observed_dr7"]
                    || r["observed_dr6"].as_u64().is_none_or(|n| n & 15 != 0)
            })
        {
            return Err("VGrid thread watches were not armed".into());
        }
        let probes = capture["isolated_post_cutoff_watch_calibration_probes"]
            .as_array()
            .ok_or("VGrid watch calibration absent")?;
        if probes.len() != 4 {
            return Err("VGrid calibration count differs".into());
        }
        for (index, row) in probes.iter().enumerate() {
            if row["index"] != index
                || row["address"] != watches[index]
                || row["eip"] != 0x76fae7
                || row["dr6"].as_u64().is_none_or(|n| n & 15 != 1 << index)
                || row["value"] != row["observed_bits"]
                || row["value"] != words[indexes[index]]
            {
                return Err("VGrid post-cutoff watch calibration differs".into());
            }
        }
        if capture["observation_cutoff"]["write_count"] != 0
            || capture["observation_cutoff"]["seed_words"] != serde_json::json!(words)
        {
            return Err("VGrid observation cutoff differs".into());
        }
    }
    if covered.iter().any(|n| !*n) {
        return Err("VGrid traces omit a seed word".into());
    }
    let pins = reference["code_pins"]
        .as_array()
        .ok_or("VGrid constructor code pins absent")?;
    if pins.len() != 5 {
        return Err("VGrid constructor/caller/calibration pins incomplete".into());
    }
    for pin in pins {
        let hex = pin["bytes_hex"].as_str().ok_or("VGrid code bytes absent")?;
        if hex.len() % 2 != 0 {
            return Err("VGrid code bytes odd length".into());
        }
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>>>()?;
        if pin["length"].as_u64() != Some(bytes.len() as u64) || pin["sha256"] != sha256(&bytes) {
            return Err("VGrid code pin digest differs".into());
        }
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn two_genuine_skirmish_traces_cover_all_five_seed_words() {
        assert_eq!(verified_words().unwrap(), [0; 5]);
    }
    #[test]
    fn incomplete_or_changed_hardware_trace_cannot_mint_seed_owner() {
        let mut reference = parse(REFERENCE).unwrap();
        reference["all_five_words_watched"] = Value::Bool(false);
        assert!(verify_document(&reference, CAPTURES).is_err());
        let reference = parse(REFERENCE).unwrap();
        let mut capture = parse(CAPTURES[0]).unwrap();
        capture["completed_grid_constructor"] = Value::Bool(false);
        let changed = serde_json::to_vec(&capture).unwrap();
        assert!(verify_document(&reference, [&changed, CAPTURES[1]]).is_err());
    }
}
