//!776B20 ordinary existing-map parameter suffix over the retained Creator.
//! Player lookup follows the actual BC order. The native CString length test
//! at+58 uses the stored length, while ID equality terminates at the first NUL.
use crate::{catalog::Catalog, definitions::Bank, resource_creator::Creator};
use aa_mapworld::header::KingdomParameter;

pub trait Players {
    type Player;
    fn first_kingdom_match(&mut self, name: &[u16]) -> Result<Option<Self::Player>, String>;
    fn application_state_e4(&mut self) -> Result<u32, String>;
    fn ai_byte_0c(&mut self, player: &Self::Player) -> Result<u8, String>;
}

///Ordinary gameType2 has no preceding gameType3 count calculation: local1C
///starts zero. Retain its actual signed wrapping stores and branch order,
///rather than introduce a capacity limit or optimize away native fields.
fn ordinary(records: &mut [KingdomParameter], players: &mut impl Players) -> Result<(), String> {
    let mut remaining = 0i32;
    for record in records.iter_mut().rev() {
        let found = players.first_kingdom_match(&record.id)?;
        let unclaimed = found.is_none() && record.byte_29 == 0 && record.string_58.is_empty();
        let use_unassigned = if players.application_state_e4()? == 3 {
            remaining > 0 && record.byte_10 == 0 && record.byte_28 != 0
        } else {
            unclaimed
        };
        if use_unassigned {
            remaining = remaining.wrapping_sub(1);
            record.byte_31 = 1;
            //Native does not touch32 on this branch, including noncanonical
            //old byte values. No implicit zero/default assignment is allowed.
        } else {
            record.byte_31 = 0;
            record.byte_32 = match found {
                Some(player) => players.ai_byte_0c(&player)?,
                None => 1,
            };
        }
    }
    Ok(())
}

///This is a reached slice, not a new Creator constructor or whole launch.
///The caller performs the real Source+50 and seed stores before entering it.
pub fn ordinary_skirmish(
    catalog: &Catalog<'_>,
    bank: &Bank,
    creator: &mut Creator,
    players: &mut impl Players,
) -> Result<(), String> {
    creator.validate(catalog, bank)?;
    let (parameters, _) = creator.human_assignment_fields_mut();
    ordinary(&mut parameters.kingdoms, players)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    fn text(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }
    struct Source<'a> {
        row: &'a Value,
        calls: Vec<String>,
        fail_ai: bool,
    }
    impl Players for Source<'_> {
        type Player = usize;
        fn first_kingdom_match(&mut self, name: &[u16]) -> Result<Option<usize>, String> {
            let name = String::from_utf16(name).unwrap();
            self.calls.push(format!("find:{name}"));
            Ok(self.row["players"]
                .as_array()
                .unwrap()
                .iter()
                .position(|p| p["kingdom"] == name))
        }
        fn application_state_e4(&mut self) -> Result<u32, String> {
            self.calls.push("E4".into());
            Ok(self.row["state_e4"].as_u64().unwrap() as u32)
        }
        fn ai_byte_0c(&mut self, p: &usize) -> Result<u8, String> {
            self.calls.push("AI".into());
            if self.fail_ai {
                Err("actual AI getter failed".into())
            } else {
                Ok(self.row["players"][*p]["ai"].as_u64().unwrap() as u8)
            }
        }
    }
    fn records(row: &Value) -> Vec<KingdomParameter> {
        row["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| KingdomParameter {
                id: text(r["id"].as_str().unwrap()),
                byte_10: r["independent"].as_u64().unwrap() as u8,
                nation: vec![],
                faction: vec![],
                color: vec![],
                handicap: vec![],
                display_key: vec![],
                string_0c: Some(vec![]),
                string_2c: text(r["profile"].as_str().unwrap()),
                byte_29: r["enabled"].as_u64().unwrap() as u8,
                byte_28: 1,
                string_14: vec![],
                localized_entries: vec![],
                definitions: vec![],
                string_58: text(r["attached"].as_str().unwrap()),
                properties: vec![],
                byte_31: r["initial31"].as_u64().unwrap() as u8,
                byte_32: r["initial32"].as_u64().unwrap() as u8,
                resources: vec![],
            })
            .collect()
    }
    #[test]
    fn ordinary_suffix_replays_all_96_original_v14_rows() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-local-bootstrap/finalize/native-finalize.json"
        ))
        .unwrap();
        assert_eq!(fixture["passed"], true);
        assert_eq!(fixture["cases"].as_array().unwrap().len(), 96);
        for row in fixture["cases"].as_array().unwrap() {
            let mut records = records(row);
            let mut source = Source {
                row,
                calls: vec![],
                fail_ai: false,
            };
            ordinary(&mut records, &mut source).unwrap();
            for (record, expected) in records.iter().zip(row["after"].as_array().unwrap()) {
                assert_eq!(record.byte_31, expected["byte31"].as_u64().unwrap() as u8);
                assert_eq!(record.byte_32, expected["byte32"].as_u64().unwrap() as u8);
            }
            assert_eq!(source.calls.first().unwrap(), "find:Independent");
        }
    }
    #[test]
    fn actual_ai_failure_retains_prior_reverse_records_and_current31_store() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-local-bootstrap/finalize/native-finalize.json"
        ))
        .unwrap();
        let row = &fixture["cases"][0];
        let mut records = records(row);
        let mut source = Source {
            row,
            calls: vec![],
            fail_ai: true,
        };
        assert!(ordinary(&mut records, &mut source).is_err());
        assert_eq!(records[0].byte_31, 128);
        assert_eq!(records[1].byte_31, 0);
        assert_eq!(records[1].byte_32, 255);
        assert_eq!(records[2].byte_31, 1);
        assert_eq!(records[3].byte_32, 1);
        assert_eq!(records[4].byte_31, 1);
    }
}
