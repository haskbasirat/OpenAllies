//! Fixture-only native command checks. Registry contents are supplied samples;
//! successful decode/encode does not authorize or execute a gameplay command.
use aa_orders::{Cursor, Position, commands::*};
use anyhow::{Context, Result};
use serde_json::{Value, json};

pub fn check_orders(document: &Value) -> Result<Vec<Value>> {
    let samples = document["samples"]
        .as_array()
        .context("orders samples missing")?;
    let mut rows = Vec::new();
    for sample in samples {
        let input = &sample["fixture"];
        let raw: Vec<u8> = serde_json::from_value(input["raw"].clone())?;
        let start: Position = serde_json::from_value(input["start"].clone())?;
        let end: Position = serde_json::from_value(input["end"].clone())?;
        let counts: RegistrySizes = serde_json::from_value(input["counts"].clone())?;
        let expected_command: CommandWire = serde_json::from_value(input["command"].clone())?;
        let mut cursor = Cursor::with_bounds(&raw, start, end.absolute()?)?;
        let decoded = CommandWire::decode(
            &mut cursor,
            counts,
            DecodeBudget {
                max_parameter_entries: 64,
            },
        )?;
        let mut writer = WireWriter::new(start)?;
        decoded.encode(&mut writer, counts)?;
        let actual = json!({"command": decoded, "read_end": cursor.position(),
            "write_end": writer.position(), "bytes": writer.bytes()});
        let expected = json!({"command": expected_command, "read_end": end,
            "write_end": end, "bytes": raw});
        rows.push(
            json!({"name": sample["name"], "source_index": sample["source_index"],
            "passed": actual == expected, "actual": actual, "expected": expected}),
        );
    }
    Ok(rows)
}
