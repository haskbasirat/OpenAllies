use super::*;
use serde_json::Value;
#[test]
fn actual_tile_bounds_and_concrete_mind_target_getters_match_native() {
    let fixture: Value = serde_json::from_slice(include_bytes!("../../../research/skirmish-20261017-stored-member-command/native-member-command-fields.json")).unwrap();
    let mut target_rows = 0;
    let mut tile_rows = 0;
    for row in fixture["rows"].as_array().unwrap() {
        let input = &row["input"];
        if input["op"] == "target" {
            target_rows += 1;
            let kind = match input["kind"].as_str().unwrap() {
                "element" => Kind::Element,
                "regiment" => Kind::Regiment,
                "aircraft" => Kind::Aircraft,
                _ => panic!(),
            };
            let offsets = match kind {
                Kind::Element => [0x90, 0x94],
                Kind::Regiment => [0x7c, 0x80],
                Kind::Aircraft => [0x4c, 0x50],
                Kind::Base => unreachable!(),
            };
            let mut reads = vec![];
            let actual = target(kind, |offset| {
                reads.push(offset);
                Ok(if offset == 0x54 && kind == Kind::Aircraft {
                    input["z"].as_u64().unwrap() as u32
                } else {
                    input["values"][usize::from(offset == offsets[1])]
                        .as_u64()
                        .unwrap() as u32
                })
            })
            .unwrap();
            assert_eq!(serde_json::json!(actual), row["output"], "{row}");
            let mut expected = offsets.to_vec();
            if kind == Kind::Aircraft {
                expected.push(0x54);
            }
            assert_eq!(reads, expected);
        } else {
            tile_rows += 1;
            let geometry = MapGeometry {
                shift: input["shift"].as_u64().unwrap() as u32,
                tile_size_bits: ((1u32 << input["shift"].as_u64().unwrap()) as f32).to_bits(),
                extent_bits: serde_json::from_value(input["extent"].clone()).unwrap(),
                width: input["width"].as_i64().unwrap() as i32,
                height: input["height"].as_i64().unwrap() as i32,
            };
            let actual = tile(
                &geometry,
                serde_json::from_value(input["point"].clone()).unwrap(),
            )
            .unwrap();
            assert_eq!(serde_json::json!(actual), row["output"], "{row}");
        }
    }
    assert_eq!((target_rows, tile_rows), (126, 261));
}
#[test]
fn base_purecall_and_unwritten_current_target_fields_do_not_create_zero_positions() {
    let mut reads = vec![];
    assert!(
        target(Kind::Base, |offset| {
            reads.push(offset);
            Ok(0)
        })
        .unwrap_err()
        .contains("purecall")
    );
    assert!(reads.is_empty());
    let mut reads = vec![];
    assert!(
        target(Kind::Aircraft, |offset| {
            reads.push(offset);
            if offset == 0x54 {
                Err("current field absent".into())
            } else {
                Ok(offset)
            }
        })
        .is_err()
    );
    assert_eq!(reads, [0x4c, 0x50, 0x54]);
}

#[test]
fn aircraft_x_reuses_x87_copy_while_mov_fields_keep_raw_bits() {
    //Hardware x87-copy contract, separate from the finite Unicorn rows.
    let bits = 0xff80_0011;
    for kind in [Kind::Element, Kind::Regiment, Kind::Aircraft] {
        let actual = target(kind, |_| Ok(bits)).unwrap();
        assert_eq!(
            actual,
            [
                if kind == Kind::Aircraft {
                    aa_mind::cai_thinks::x87_copy(bits)
                } else {
                    bits
                },
                bits
            ]
        );
    }
}
