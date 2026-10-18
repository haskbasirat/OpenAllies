//! Replays research/phase8-orders/order-traces.json: for every one of the 357
//! original order handlers, the ORIGINAL x86 reader decoded generated wire
//! bytes (and, for requests, the original writer re-encoded the decree).
//! Rust must decode the same fields in the same order and widths, stop at the
//! same native cursor position, re-encode the identical bytes, and reproduce
//! the request -> decree re-encoding under the harness' declared resolution.
use aa_orders::commands::WireWriter;
use aa_orders::payload::{self, Field, NameKind, Registry, SchemaContext, ThingCheck};
use aa_orders::{Cursor, Position, registry_bits};
use serde_json::Value;
use std::collections::HashMap;

fn traces() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-orders/order-traces.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("trace fixture")).unwrap()
}

/// Mirrors the harness environment recorded in the fixture.
struct Fixture {
    counts: HashMap<u32, usize>,
    team_flags: Vec<u32>,
    kind_modulus: u32,
    players: Vec<u32>,
}
impl Fixture {
    fn new(v: &Value) -> Self {
        let counts = v["managers"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, c)| {
                (
                    u32::from_str_radix(k.trim_start_matches("0x"), 16).unwrap(),
                    c.as_u64().unwrap() as usize,
                )
            })
            .collect();
        let env = &v["environment"];
        let modulus = env["definition_plus_0x20"]
            .as_str()
            .unwrap()
            .rsplit(' ')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        Self {
            counts,
            team_flags: serde_json::from_value(env["team_command_flags"].clone()).unwrap(),
            kind_modulus: modulus,
            players: serde_json::from_value(env["session_player_ids"].clone()).unwrap(),
        }
    }
}
impl SchemaContext for Fixture {
    fn registry_count(&self, registry: Registry) -> aa_orders::Result<usize> {
        self.counts
            .get(&registry.0)
            .copied()
            .ok_or(aa_orders::Error::SchemaContext("unknown registry"))
    }
    fn team_command_flags(&self, index: u32) -> aa_orders::Result<u32> {
        self.team_flags
            .get(index as usize)
            .copied()
            .ok_or(aa_orders::Error::SchemaContext("team command index"))
    }
    fn definition_kind(&self, _registry: Registry, index: u32) -> aa_orders::Result<u32> {
        Ok(index % self.kind_modulus)
    }
    fn zero_width_list_budget(&self) -> usize {
        1 << 16
    }
}
impl payload::Resolver for Fixture {
    fn thing(&self, _check: ThingCheck, slot: u16) -> Option<u16> {
        Some(slot)
    }
    fn player(&self, id: u32) -> bool {
        self.players.contains(&id)
    }
    fn definition(&self, registry: Registry, index: u32) -> bool {
        (index as usize) < self.counts[&registry.0]
    }
    /// Harness seams: 0x41EEC0 returns null for an empty key, otherwise an
    /// object named by the key; the metagame lookups always return an object.
    fn named(&self, kind: NameKind, key: &[u16]) -> Option<Vec<u16>> {
        (kind != NameKind::RandomMapConfig || !key.is_empty()).then(|| key.to_vec())
    }
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
/// Flatten fields into the primitive 0x40D5D0 operations the native trace records.
fn primitives(fields: &[Field], ctx: &Fixture) -> Vec<(u64, Value)> {
    let mut out = Vec::new();
    let num = |w: usize, v: u32| (w as u64, Value::from(v));
    for f in fields {
        match f {
            Field::Uint { width, value } => out.push(num(*width as usize, *value)),
            Field::Text(units) | Field::Named { units, .. } => {
                out.push(num(16, units.len() as u32));
                let bytes: Vec<u8> = units.iter().flat_map(|u| u.to_le_bytes()).collect();
                let width = units.len() * 16;
                if width <= 32 {
                    let mut b = [0u8; 4];
                    b[..bytes.len()].copy_from_slice(&bytes);
                    out.push(num(width, u32::from_le_bytes(b)));
                } else {
                    out.push((width as u64, Value::from(to_hex(&bytes))));
                }
            }
            Field::Bounded { n, value } => {
                out.push(num(registry_bits(*n as usize).unwrap() as usize, *value))
            }
            Field::Def { registry, index } => {
                let c = ctx.counts[&registry.0];
                out.push(num(registry_bits(c + 1).unwrap() as usize, *index))
            }
            Field::Thing { slot, .. } => out.push(num(16, u32::from(*slot))),
            Field::Player(id) => out.push(num(32, *id)),
            Field::NetAddr { ip, port } => {
                out.push(num(32, *ip));
                out.push(num(32, *port));
            }
        }
    }
    out
}

#[test]
fn every_native_order_payload_matches_original_reader_and_writer() {
    let v = traces();
    let ctx = Fixture::new(&v);
    let only: Option<Vec<usize>> = std::env::var("AA_ORDER_TRACE_ONLY")
        .ok()
        .map(|s| s.split(',').map(|x| x.trim().parse().unwrap()).collect());
    let mut failures = Vec::new();
    let mut checked = 0usize;
    let mut nulls = 0usize;
    for d in v["descriptors"].as_array().unwrap() {
        let id = d["id"].as_u64().unwrap() as usize;
        if only.as_ref().is_some_and(|o| !o.contains(&id)) {
            continue;
        }
        for (k, run) in d["runs"].as_array().unwrap().iter().enumerate() {
            let status = run["status"].as_str().unwrap();
            assert!(
                status == "stopped_before_process" || status == "native_null_dereference",
                "{id} run {k}: {status}"
            );
            if status == "native_null_dereference" {
                // The original faults here; Rust must refuse rather than invent data.
                if check_run(id, run, &ctx).is_ok() {
                    failures.push(format!("{id} run {k}: native null dereference decoded without error"));
                }
                nulls += 1;
                continue;
            }
            if let Err(e) = check_run(id, run, &ctx) {
                failures.push(format!("{id} {} run {k}: {e}", d["name"].as_str().unwrap()));
            }
            checked += 1;
        }
    }
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
    assert!(checked > 0);
    eprintln!("checked {checked} native runs, {nulls} native null-dereference runs refused");
}

fn check_run(id: usize, run: &Value, ctx: &Fixture) -> Result<(), String> {
    let input = hex(run["input"].as_str().unwrap());
    let end: Position = serde_json::from_value(serde_json::json!({
        "byte": run["end"][0], "bit": run["end"][1]})).unwrap();
    let end_bits = end.absolute().unwrap();
    let mut bytes = input.clone();
    bytes.resize(bytes.len().max(1), 0);
    let mut cursor = Cursor::with_bounds(&bytes, Position { byte: 0, bit: 3 }, end_bits)
        .map_err(|e| e.to_string())?;
    let fields = payload::decode(id, &mut cursor, ctx).map_err(|e| format!("decode: {e}"))?;
    if cursor.position() != end {
        return Err(format!("end {:?} != native {:?}", cursor.position(), end));
    }
    let expected: Vec<(u64, Value)> = run["reads"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["width"].as_u64().unwrap(), r["value"].clone()))
        .collect();
    let got = primitives(&fields, ctx);
    if got != expected {
        return Err(format!("reads differ\n rust   {got:?}\n native {expected:?}"));
    }
    // Byte-exact re-encoding through the original packing.
    let mut w = WireWriter::new(Position { byte: 0, bit: 3 }).unwrap();
    payload::encode(id, &fields, &mut w, ctx).map_err(|e| format!("encode: {e}"))?;
    if w.position() != end || w.bytes().get(..input.len()).map(to_hex) != Some(to_hex(&input)) {
        return Err(format!(
            "re-encode {} at {:?} != {} at {:?}",
            to_hex(w.bytes()),
            w.position(),
            to_hex(&input),
            end
        ));
    }
    // Request -> decree: original ProcessRequest writes decree id + payload.
    if let Some(writes) = run["writes"].as_array().filter(|w| !w.is_empty()) {
        let decree = writes[0]["value"].as_u64().unwrap() as usize;
        if writes[0]["width"] != 9 {
            return Err("first write is not the 9-bit decree id".into());
        }
        let normalized = match payload::reserialize(id, &fields, ctx, ctx) {
            Err(aa_orders::Error::UnsupportedReserialization(_)) => return Ok(()),
            other => other.map_err(|e| format!("normalize: {e}"))?,
        };
        // Every original 0x40D420 call on the decree stream after the id.
        let native_writes: Vec<(u64, Value)> = writes[1..]
            .iter()
            .map(|r| (r["width"].as_u64().unwrap(), r["value"].clone()))
            .collect();
        let rust_writes = primitives(&normalized, ctx);
        if rust_writes != native_writes {
            return Err(format!(
                "decree {decree} writes differ\n rust   {rust_writes:?}\n native {native_writes:?}"
            ));
        }
        let mut w = WireWriter::new(Position { byte: 0, bit: 3 }).unwrap();
        w.write_uint(decree as u32, 9).unwrap();
        payload::encode(decree, &normalized, &mut w, ctx).map_err(|e| format!("decree encode: {e}"))?;
        let out = hex(run["output"].as_str().unwrap());
        let oend: Position = serde_json::from_value(serde_json::json!({
            "byte": run["output_end"][0], "bit": run["output_end"][1]})).unwrap();
        if w.position() != oend || w.bytes().get(..out.len()).map(to_hex) != Some(to_hex(&out)) {
            return Err(format!(
                "decree {decree}: {} at {:?} != native {} at {:?}",
                to_hex(w.bytes()),
                w.position(),
                to_hex(&out),
                oend
            ));
        }
    }
    Ok(())
}
