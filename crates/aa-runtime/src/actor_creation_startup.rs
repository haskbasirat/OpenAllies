//!Sealed9C8680 direction-table startup from the actual Windows/v14 reference.
//!The capture stopped before WinMain/source loading. Its entry/return CW027F,
//!not a source-Bank precision or equality of the PC53/64 tables, chooses the
//!startup PC. Live actor arithmetic remains a separate caller-owned input.
use crate::combat_source_receipt::{self, VerifiedStartupReceipt};
use aa_actors::creation::DirectionTable;
use aa_numeric::Precision;
use aa_rules::sources::sha256;
use serde_json::Value;

type Result<T> = std::result::Result<T, String>;
const CAPTURE: &[u8] =
    include_bytes!("../../../research/development10/native-directions-capture-02/capture.json");
const NATIVE_TABLE: &[u8] = include_bytes!(
    "../../../research/development10/native-directions-capture-02/actual-direction-table.bin"
);
const TABLES: &[u8] = include_bytes!("../../../research/phase5-actors/direction-tables.bin");
const REFERENCE: &[u8] = include_bytes!(
    "../../../research/skirmish-20261017-direction-startup/hardware-direction-reference.json"
);
pub const CAPTURE_SHA256: &str = "6e674da7f51963b98e3331a583150f5d5bf760c61ae9bac66322cd7ef66dc18e";
pub const TABLE_SHA256: &str = "a0afa5e1cb34b5caedeadf9ced0020bdf3a8b4a5f553da33fe80b018176a1c02";
const TABLES_SHA256: &str = "20826310088f1a222dd5e480efe7697a8e47814f7f5743175b26ca6238dc0e95";
const REFERENCE_SHA256: &str = "f97ee667f7773c7616b968b926593c06569a7b211122bbc649e6b69e17824e54";

///No public raw/default/Clone/Deserialize constructor. One application retains
///this capability and lends its immutable table to actual fresh creation.
pub struct StartupInputs {
    startup_precision: Precision,
}
impl StartupInputs {
    pub fn from_verified_startup_reference(receipt: &VerifiedStartupReceipt) -> Result<Self> {
        let target = receipt
            .context()
            .reference_startup()
            .ok_or("actor direction startup requires a verified native reference target")?;
        if target.native_exe_sha256 != combat_source_receipt::EXE_SHA256 {
            return Err("actor direction startup reference executable differs".into());
        }
        //Deliberately never inspect receipt.context().precision(). Source
        //loading occurs later than this independently observed CRT initializer.
        let startup_precision = verify_reference()?;
        Ok(Self { startup_precision })
    }
    pub fn startup_precision(&self) -> Precision {
        self.startup_precision
    }
    pub fn direction_table(&self) -> DirectionTable {
        DirectionTable::original(self.startup_precision)
    }
    pub fn capture_sha256(&self) -> &'static str {
        CAPTURE_SHA256
    }
}
fn control(raw: &Value, name: &str) -> Result<u16> {
    let hex = raw[name]
        .as_str()
        .ok_or("direction capture raw FP block missing")?;
    let word = hex
        .get(..4)
        .ok_or("direction capture raw FP block is short")?;
    let low = u8::from_str_radix(&word[..2], 16).map_err(|e| e.to_string())?;
    let high = u8::from_str_radix(&word[2..], 16).map_err(|e| e.to_string())?;
    Ok(u16::from_le_bytes([low, high]))
}
fn captured_precision(capture: &Value) -> Result<Precision> {
    if capture["schema"] != 2
        || capture["completed_direction_return"] != true
        || !capture["original_error"].is_null()
        || capture["fp_state_never_written_by_debugger"] != true
        || capture["temporary_process_only_breakpoints"] != true
        || capture["source_exe_sha256"] != combat_source_receipt::EXE_SHA256
        || capture["runtime_pins"]["AA.exe"] != combat_source_receipt::EXE_SHA256
        || capture["cleanup"]["wait_result"] != 0
        || capture["cleanup"]["exit_code"] != 0
        || capture["cleanup"]["exit_process_event_observed"] != true
        || !capture["cleanup"]["errors"]
            .as_array()
            .is_some_and(Vec::is_empty)
    {
        return Err(
            "direction capture is outside the completed isolated native startup domain".into(),
        );
    }
    let suspended = capture["cleanup"]["manual_suspend_counts_at_termination"]
        .as_object()
        .ok_or("direction capture cleanup thread counts missing")?;
    if suspended.is_empty() || suspended.values().any(|n| n != 0) {
        return Err("direction capture left manual thread suspensions".into());
    }
    let captures = capture["captures"]
        .as_array()
        .ok_or("direction capture contexts missing")?;
    let [entry, returned] = captures.as_slice() else {
        return Err("direction capture entry/return pair differs".into());
    };
    if entry["label"] != "direction_entry"
        || returned["label"] != "direction_return"
        || entry["eip"] != 0x9c8680
        || returned["eip"] != 0x96ecd4
        || entry["thread_id"] != returned["thread_id"]
        || entry["thread_id"].as_u64().unwrap_or(0) == 0
        || returned["esp"].as_u64() != entry["esp"].as_u64().and_then(|n| n.checked_add(4))
    {
        return Err(
            "direction capture did not return through the same native initializer call".into(),
        );
    }
    for raw in captures {
        if raw["x87_control"] != 0x027f
            || raw["legacy_x87_control"] != 0x027f
            || raw["fxsave_x87_control"] != 0x027f
            || raw["legacy_fxsave_match"] != true
            || control(raw, "raw_legacy_float_save")? != 0x027f
            || control(raw, "raw_fxsave")? != 0x027f
        {
            return Err("direction initializer hardware control word differs from the sealed PC53 observation".into());
        }
    }
    let table = &returned["directions"];
    if table["count"] != 2048
        || table["bytes"] != 16384
        || table["sha256"] != TABLE_SHA256
        || table["pointer"].as_u64().unwrap_or(0) == 0
    {
        return Err("direction capture table count/data receipt differs".into());
    }
    Ok(Precision::Double53)
}
fn verify_reference() -> Result<Precision> {
    for (name, bytes, expected) in [
        ("capture", CAPTURE, CAPTURE_SHA256),
        ("native table", NATIVE_TABLE, TABLE_SHA256),
        ("prior oracle tables", TABLES, TABLES_SHA256),
        ("native code reference", REFERENCE, REFERENCE_SHA256),
    ] {
        if sha256(bytes) != expected {
            return Err(format!("sealed actor direction {name} digest differs"));
        }
    }
    let capture: Value = serde_json::from_slice(CAPTURE).map_err(|e| e.to_string())?;
    let precision = captured_precision(&capture)?;
    let reference: Value = serde_json::from_slice(REFERENCE).map_err(|e| e.to_string())?;
    if reference["native_exe_sha256"] != combat_source_receipt::EXE_SHA256
        || reference["capture_sha256"] != CAPTURE_SHA256
        || reference["native_table_sha256"] != TABLE_SHA256
        || reference["oracle_tables_sha256"] != TABLES_SHA256
        || reference["startup_control_word"] != 0x027f
        || reference["startup_precision"] != "Double53"
        || reference["entry_address"] != 0x9c8680
    {
        return Err("direction reference binding differs from its native capture".into());
    }
    let pins = reference["code_pins"]
        .as_array()
        .ok_or("direction code windows missing")?;
    if pins.len() != 3 {
        return Err("direction producer code windows incomplete".into());
    }
    for pin in pins {
        let raw = pin["bytes_hex"]
            .as_str()
            .ok_or("direction code bytes missing")?;
        if raw.len() % 2 != 0 {
            return Err("direction code byte encoding differs".into());
        }
        let bytes = (0..raw.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>>>()?;
        if Some(bytes.len() as u64) != pin["length"].as_u64() || pin["sha256"] != sha256(&bytes) {
            return Err("direction native code window digest differs".into());
        }
    }
    if NATIVE_TABLE.len() != 2048 * 8 || TABLES.len() != 3 * 2048 * 8 {
        return Err("direction native table lengths differ".into());
    }
    let selected = DirectionTable::original(precision);
    for (index, raw) in NATIVE_TABLE.chunks_exact(8).enumerate() {
        let expected = [
            u32::from_le_bytes(raw[..4].try_into().expect("four bytes")),
            u32::from_le_bytes(raw[4..].try_into().expect("four bytes")),
        ];
        if selected.at(index) != Some(expected) {
            return Err(format!(
                "actual native direction vector{index} differs from selected embedded table"
            ));
        }
    }
    Ok(precision)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_native_startup_cw_and_all_2048_vectors_choose_pc53() {
        assert_eq!(verify_reference().unwrap(), Precision::Double53);
        let mut differing24 = 0;
        for index in 0..2048 {
            let actual = DirectionTable::original(Precision::Double53).at(index);
            assert_eq!(
                actual,
                DirectionTable::original(Precision::Extended64).at(index)
            );
            differing24 +=
                usize::from(actual != DirectionTable::original(Precision::Single24).at(index));
        }
        assert_eq!(differing24, 433);
        assert_eq!(DirectionTable::original(Precision::Double53).at(2048), None);
    }
    #[test]
    fn same_table_at_pc64_is_not_authority_to_change_captured_startup_pc() {
        let original: Value = serde_json::from_slice(CAPTURE).unwrap();
        let mut control = original.clone();
        control["captures"][0]["x87_control"] = 0x037f.into();
        assert!(captured_precision(&control).is_err());
        let mut thread = original.clone();
        thread["captures"][1]["thread_id"] = 1.into();
        assert!(captured_precision(&thread).is_err());
        let mut partial = original.clone();
        partial["completed_direction_return"] = false.into();
        assert!(captured_precision(&partial).is_err());
        let mut count = original;
        count["captures"][1]["directions"]["count"] = 4096.into();
        assert!(captured_precision(&count).is_err());
    }
}
