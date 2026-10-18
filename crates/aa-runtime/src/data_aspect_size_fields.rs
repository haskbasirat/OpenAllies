//!469CC0 finite binary32 size fields, x87 square/add/FSQRT at source PC.
//!The caller stores58/5C before invoking this same arithmetic for+60.
use aa_numeric::{Binary, Precision};
pub(super) fn radius(size: [u32; 2], pc: Precision) -> std::result::Result<u32, String> {
    let x = Binary::from_f32_bits(size[0]).map_err(|e| e.to_string())?;
    let y = Binary::from_f32_bits(size[1]).map_err(|e| e.to_string())?;
    let xx = x.mul(&x, pc).map_err(|e| e.to_string())?;
    let yy = y.mul(&y, pc).map_err(|e| e.to_string())?;
    xx.add(&yy, pc)
        .and_then(|sum| sum.sqrt(pc))
        .and_then(|v| v.store_f32())
        .map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_banner_normal_size_copy_and_fsqrt_all_pc() {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-data-pre-request/native-banner-aspect.json"
        ))
        .unwrap();
        for row in doc["rows"].as_array().unwrap() {
            let pc = match row["precision"].as_str().unwrap() {
                "Single24" => Precision::Single24,
                "Double53" => Precision::Double53,
                "Extended64" => Precision::Extended64,
                _ => panic!(),
            };
            let size: [u32; 2] = serde_json::from_value(row["size"].clone()).unwrap();
            let actual = if row["output"]["gate"] == 1 {
                radius(size, pc).unwrap()
            } else {
                0
            };
            assert_eq!(actual, row["output"]["radius"], "{row}");
        }
    }
}
