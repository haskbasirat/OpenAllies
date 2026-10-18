//!766560 over a scoped, actual BC roster. Native76CE10 searches first-match
//!case-sensitive names; no snapshot list or implicit authorization is created.
use crate::{Error, Result};
pub trait World {
    type Player: PartialEq;
    fn first_named(&mut self, name: &[u16]) -> Result<Option<Self::Player>>;
    fn duplicate_name(&mut self, original: &[u16], number: u32) -> Result<Vec<u16>>;
    fn store_name_7664f0(&mut self, player: &Self::Player, name: Vec<u16>) -> Result<()>;
}
pub fn set<W: World>(w: &mut W, player: &W::Player, name: &[u16]) -> Result<()> {
    let mut candidate = name.to_vec();
    let mut n = 1u32;
    loop {
        match w.first_named(&candidate)? {
            Some(found) if &found != player => {
                candidate = w.duplicate_name(name, n)?;
                n = n.wrapping_add(1);
            }
            _ => break,
        }
    }
    w.store_name_7664f0(player, candidate)
}

///The verified VC7.1 wide printf domain used by the shipped duplicate-name
///text: string then signed32 integer; literal%% consumes no argument. Other
///conversions require their actual CRT type/width and are explicitly refused.
pub fn format(format: &[u16], name: &[u16], number: u32) -> Result<Vec<u16>> {
    let mut output = Vec::new();
    let mut argument = 0;
    let mut index = 0;
    while index < format.len() {
        let unit = format[index];
        if unit == 0 {
            break;
        }
        if unit != u16::from(b'%') {
            output.push(unit);
            index += 1;
            continue;
        }
        match format.get(index + 1).copied() {
            Some(v) if v == u16::from(b'%') => output.push(v),
            Some(v) if v == u16::from(b's') && argument == 0 => {
                output.extend(name.iter().copied().take_while(|&v| v != 0));
                argument = 1;
            }
            Some(v) if v == u16::from(b'd') && argument == 1 => {
                output.extend((number as i32).to_string().encode_utf16());
                argument = 2;
            }
            _ => {
                return Err(Error::Unsupported(
                    "766560 current duplicate-name format outside verified %s/%d/%% domain",
                ));
            }
        }
        index += 2;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_vc71_duplicate_name_format_replay() {
        let doc:serde_json::Value=serde_json::from_str(include_str!(
            "../../../../research/skirmish-20261016-local-bootstrap/implicit-ai/native-name-format.json"
        )).unwrap();
        assert_eq!(doc["passed"], true);
        for row in doc["cases"].as_array().unwrap() {
            let units = |v: &serde_json::Value| {
                v.as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u16)
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                format(
                    &units(&row["format"]),
                    &units(&row["name"]),
                    row["number"].as_u64().unwrap() as u32
                )
                .unwrap(),
                units(&row["output"])
            );
        }
    }
}
