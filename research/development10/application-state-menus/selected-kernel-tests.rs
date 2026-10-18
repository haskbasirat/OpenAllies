type Result<T>=std::result::Result<T,Error>;
#[derive(Debug)]enum Error{Unsupported(&'static str)}
pub struct LaunchState {
    /// +0 (4 bits, 0x411FA0(16)).
    pub kind: u32,
    pub flag_04: bool,
    pub text: Vec<u16>,
    pub field_0c: u32,
    pub field_10: u32,
    pub flag_14: bool,
    pub flag_15: bool,
    pub flag_16: bool,
    /// +0x18 (4 bits, 0x411FA0(16)).
    pub field_18: u32,
    pub field_1c: u32,
    pub flag_20: bool,
    /// +0x24 (3 bits, 0x411FA0(7)).
    pub field_24: u32,
}
pub fn constructor_launch_state() -> LaunchState {
    LaunchState {
        kind: 0,
        flag_04: false,
        text: Vec::new(),
        field_0c: u32::MAX,
        field_10: 1,
        flag_14: false,
        flag_15: false,
        flag_16: false,
        field_18: 0,
        field_1c: 0,
        flag_20: false,
        field_24: 0,
    }
}
fn ordinary_launch(current: u32) -> Result<LaunchState> {
    if current == 7 {
        return Err(Error::Unsupported(
            "68F629 multiplayer selected-map confirmation",
        ));
    }
    let mut state = constructor_launch_state();
    //Actual40F7D0→7841E0 then784130(4,1). No784160 mode-reset prefix.
    state.kind = 6;
    state.field_18 = 4;
    state.field_1c = 1;
    Ok(state)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    #[test]
    fn native_ordinary_confirmation_preserves_ctor_fields_and_rejects_multiplayer() {
        let f: Value = serde_json::from_str(include_str!(
            "native-selected-confirm.json"
        ))
        .unwrap();
        let mut n = 0;
        for row in f["rows"].as_array().unwrap() {
            if row["op"] != "confirm" {
                continue;
            }
            let current = row["current"].as_u64().unwrap() as u32;
            if current == 7 {
                assert!(ordinary_launch(current).is_err());
                continue;
            }
            let s = ordinary_launch(current).unwrap();
            assert_eq!(
                json!({"kind":s.kind,"flag04":u8::from(s.flag_04),"text_length":s.text.len(),"field0c":s.field_0c,"field10":s.field_10,"flag14":u8::from(s.flag_14),"flag15":u8::from(s.flag_15),"flag16":u8::from(s.flag_16),"field18":s.field_18,"field1c":s.field_1c,"flag20":u8::from(s.flag_20),"field24":s.field_24}),
                row["output"]["launch"]
            );
            assert_eq!(row["output"]["game_type"], 2);
            n += 1;
        }
        assert_eq!(n, 9);
    }
}
