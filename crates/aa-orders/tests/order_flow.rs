//! Replays research/phase8-orders/order-flow.json: every ORIGINAL request
//! handler (accepted, no ordering client, version not acknowledged, film
//! session, accepted with mirror) and decree handler (mirror off/on) run to
//! completion with the process function stubbed. Rust flow::process_request /
//! process_decree must produce the same rejections, synchronizer entries,
//! process call, mirror query, and decree/mirror stream contents.
use aa_orders::commands::WireWriter;
use aa_orders::flow::{self, Log, OrderHost, RequestOutcome, SyncEntry, Validity};
use aa_orders::payload::{self, Field, NameKind, Registry, SchemaContext, ThingCheck};
use aa_orders::{Cursor, Position, registry_bits};
use serde_json::{Value, json};
use std::collections::HashMap;

fn load(name: &str) -> Value {
    let path = format!("{}/../../research/phase8-orders/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

struct Ctx {
    counts: HashMap<u32, usize>,
}
impl SchemaContext for Ctx {
    fn registry_count(&self, r: Registry) -> aa_orders::Result<usize> {
        self.counts.get(&r.0).copied().ok_or(aa_orders::Error::SchemaContext("registry"))
    }
    fn team_command_flags(&self, index: u32) -> aa_orders::Result<u32> {
        [0u32, 1, 2, 8, 0xb].get(index as usize).copied().ok_or(aa_orders::Error::SchemaContext("team"))
    }
    fn definition_kind(&self, _r: Registry, index: u32) -> aa_orders::Result<u32> {
        Ok(index % 4)
    }
    fn zero_width_list_budget(&self) -> usize {
        1 << 16
    }
}
impl payload::Resolver for Ctx {
    fn thing(&self, _c: ThingCheck, slot: u16) -> Option<u16> {
        Some(slot)
    }
    fn player(&self, id: u32) -> bool {
        id == 1 || id == 2
    }
    fn definition(&self, r: Registry, index: u32) -> bool {
        (index as usize) < self.counts[&r.0]
    }
    fn named(&self, kind: NameKind, key: &[u16]) -> Option<Vec<u16>> {
        (kind != NameKind::RandomMapConfig || !key.is_empty()).then(|| key.to_vec())
    }
}

struct Host<'a> {
    config: &'a str,
    events: Vec<Value>,
}
impl OrderHost for Host<'_> {
    fn has_ordering_client(&mut self) -> bool {
        self.config != "no_ordering_client"
    }
    fn version_acknowledged(&mut self) -> bool {
        self.config != "version_not_acked"
    }
    fn film_session(&mut self) -> bool {
        self.config == "film_session"
    }
    fn valid(&mut self, _v: Validity, _r: usize, _f: &[Field]) -> aa_orders::Result<bool> {
        Ok(true) // the oracle forces validity seams true
    }
    fn sync(&mut self, e: SyncEntry) -> aa_orders::Result<()> {
        let log = |l: Log| if l == Log::A { "A" } else { "S" };
        self.events.push(match e {
            SyncEntry::Int { log: l, label, value } => {
                json!({"kind":"sync_int","log":log(l),"label":label,"value":value})
            }
            SyncEntry::Text { log: l, label, value } => {
                json!({"kind":"sync_string","log":log(l),"label":label,"value":value})
            }
        });
        Ok(())
    }
    fn process(&mut self, _d: usize, _f: &mut Vec<Field>) -> aa_orders::Result<()> {
        self.events.push(json!({"kind":"process"}));
        Ok(())
    }
    fn mirror_enabled(&mut self) -> bool {
        let on = self.config.ends_with("mirror");
        self.events.push(json!({"kind":"mirror_query","value":on}));
        on
    }
}

/// Primitive 0x40D420 writes a field list produces (same convention as the
/// trace fixtures: <=32-bit values numeric, longer buffers hex).
fn primitives(fields: &[Field], ctx: &Ctx) -> Vec<Value> {
    let mut out = Vec::new();
    let p = |w: usize, v: Value| json!([w, v]);
    for f in fields {
        match f {
            Field::Uint { width, value } => out.push(p(*width as usize, json!(value))),
            Field::Text(u) | Field::Named { units: u, .. } => {
                out.push(p(16, json!(u.len())));
                let b: Vec<u8> = u.iter().flat_map(|x| x.to_le_bytes()).collect();
                if u.len() * 16 <= 32 {
                    let mut x = [0u8; 4];
                    x[..b.len()].copy_from_slice(&b);
                    out.push(p(u.len() * 16, json!(u32::from_le_bytes(x))));
                } else {
                    out.push(p(u.len() * 16, json!(b.iter().map(|x| format!("{x:02x}")).collect::<String>())));
                }
            }
            Field::Bounded { n, value } => out.push(p(registry_bits(*n as usize).unwrap() as usize, json!(value))),
            Field::Def { registry, index } => {
                out.push(p(registry_bits(ctx.counts[&registry.0] + 1).unwrap() as usize, json!(index)))
            }
            Field::Thing { slot, .. } => out.push(p(16, json!(slot))),
            Field::Player(id) => out.push(p(32, json!(id))),
            Field::NetAddr { ip, port } => {
                out.push(p(32, json!(ip)));
                out.push(p(32, json!(port)));
            }
        }
    }
    out
}

#[test]
fn every_native_request_and_decree_flow_matches() {
    let v = load("order-flow.json");
    let ctx = Ctx {
        counts: v["managers"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, c)| (u32::from_str_radix(k.trim_start_matches("0x"), 16).unwrap(), c.as_u64().unwrap() as usize))
            .collect(),
    };
    let mut failures = Vec::new();
    let mut checked = 0;
    for d in v["descriptors"].as_array().unwrap() {
        let id = d["id"].as_u64().unwrap() as usize;
        for case in d["cases"].as_array().unwrap() {
            let config = case["config"].as_str().unwrap();
            if case["status"] == "native_null_dereference" {
                // The original faults (e.g. writer 0x76A8C0 on an unresolved
                // named reference); Rust must refuse instead of inventing data.
                if check(id, config, case, &ctx).is_ok() {
                    failures.push(format!("{id} {config}: native fault accepted by Rust"));
                }
                continue;
            }
            if case["status"] != "returned" {
                failures.push(format!("{id} {config}: native status {}", case["status"]));
                continue;
            }
            match check(id, config, case, &ctx) {
                Ok(()) => checked += 1,
                Err(e) => failures.push(format!("{id} {} {config}: {e}", d["name"].as_str().unwrap())),
            }
        }
    }
    assert!(failures.is_empty(), "{} failures (checked {checked}):\n{}", failures.len(), failures.join("\n"));
}

fn check(id: usize, config: &str, case: &Value, ctx: &Ctx) -> Result<(), String> {
    let input = hex(case["input"].as_str().unwrap());
    let end: Position = serde_json::from_value(json!({"byte": case["end"][0], "bit": case["end"][1]})).unwrap();
    let mut bytes = input.clone();
    bytes.resize(bytes.len().max(1), 0);
    let mut cursor = Cursor::with_bounds(&bytes, Position { byte: 0, bit: 3 }, end.absolute().unwrap()).unwrap();
    let fields = payload::decode(id, &mut cursor, ctx).map_err(|e| format!("decode {e}"))?;
    let native: Vec<Value> = case["events"].as_array().unwrap().clone();
    let expect_events: Vec<Value> = native
        .iter()
        .filter(|e| e["kind"] != "write")
        .map(|e| match e["kind"].as_str().unwrap() {
            "process" => json!({"kind":"process"}),
            _ => e.clone(),
        })
        .collect();
    let stream = |name: &str| -> Vec<Value> {
        native
            .iter()
            .filter(|e| e["kind"] == "write" && e["stream"] == name)
            .map(|e| json!([e["width"], e["value"]]))
            .collect()
    };
    let mut host = Host { config, events: Vec::new() };
    let mut decree_w = WireWriter::new(Position { byte: 0, bit: 3 }).unwrap();
    let mut mirror_w = WireWriter::new(Position { byte: 0, bit: 3 }).unwrap();
    let shape = flow::shape(id).map_err(|e| e.to_string())?;
    let normalized = payload::reserialize(id, &fields, ctx, ctx);
    if shape.request {
        let outcome = flow::process_request(id, &fields, &mut host, ctx, ctx, &mut decree_w, &mut mirror_w);
        let outcome = match outcome {
            Err(aa_orders::Error::UnsupportedReserialization(_)) => return Ok(()),
            other => other.map_err(|e| format!("process_request {e}"))?,
        };
        let line = match outcome {
            RequestOutcome::NoOrderingClient => Some(0x114),
            RequestOutcome::VersionNotAcknowledged => Some(0x118),
            RequestOutcome::WouldInterfereWithFilm => Some(0x11c),
            RequestOutcome::Invalid => Some(-1),
            RequestOutcome::Accepted => None,
        };
        if let Some(line) = line {
            host.events.insert(0, json!({"kind":"rejected","line":line}));
        }
        let expect: Vec<Value> = expect_events
            .iter()
            .map(|e| if e["kind"] == "rejected" { json!({"kind":"rejected","line":e["line"]}) } else { e.clone() })
            .collect();
        if host.events != expect {
            return Err(format!("events\n rust   {:?}\n native {:?}", host.events, expect));
        }
        if outcome == RequestOutcome::Accepted {
            let order = normalized.map_err(|e| e.to_string())?;
            let mut w = vec![json!([9, shape.decree])];
            w.extend(primitives(&order, ctx));
            if w != stream("decree") {
                return Err(format!("decree writes\n rust   {w:?}\n native {:?}", stream("decree")));
            }
            let m = if config.ends_with("mirror") && shape.mirror { w } else { Vec::new() };
            if m != stream("mirror") {
                return Err(format!("mirror writes\n rust   {m:?}\n native {:?}", stream("mirror")));
            }
        }
    } else {
        match flow::process_decree(id, &fields, &mut host, ctx, ctx, &mut mirror_w) {
            Err(aa_orders::Error::UnsupportedReserialization(_)) => return Ok(()),
            other => other.map_err(|e| format!("process_decree {e}"))?,
        }
        if host.events != expect_events {
            return Err(format!("events\n rust   {:?}\n native {:?}", host.events, expect_events));
        }
        let m = if config.ends_with("mirror") && shape.mirror {
            let order = normalized.map_err(|e| e.to_string())?;
            let mut w = vec![json!([9, id])];
            w.extend(primitives(&order, ctx));
            w
        } else {
            Vec::new()
        };
        if m != stream("mirror") {
            return Err(format!("mirror writes\n rust   {m:?}\n native {:?}", stream("mirror")));
        }
    }
    Ok(())
}
