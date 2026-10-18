//! The native initialized FX unset-position sentinel atB28114/18/1C.
//!
//!Both baseline PEs store binary32(-100.0) for all three components. The34
//!direct references are source reads or pass its address to read-only point
//!readers428170/428430/42A710->429080. Constructors428620/4286E0 copy these
//!actual bits. This is an immutable engine constant, not a zero/default world
//!position, a PointShape reference, or user-configurable source data.
static UNSET_POSITION: [u32; 3] = [0xc2c8_0000; 3];

/// Read the same initialized native constant only when its caller reaches
///the native constructor's global-position read. No new mutable owner exists.
pub fn unset_position() -> [u32; 3] {
    UNSET_POSITION
}

#[cfg(test)]
mod tests {
    #[test]
    fn initialized_constant_matches_every_native_constructor_and_point_reader() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-fx-static-position/native-fx-static-position.json"
        ))
        .unwrap();
        let expected = serde_json::json!(super::unset_position());
        assert_eq!(expected, fixture["initialized_data"]["bits"]);
        assert_eq!(fixture["direct_reference_census"].as_array().unwrap().len(), 34);
        for row in fixture["rows"].as_array().unwrap() {
            assert_eq!(expected, row["output"]["globals"]);
            if row["input"]["kind"] == "constructor" {
                assert_eq!(expected, row["output"]["target_position"]);
            }
        }
    }
}
