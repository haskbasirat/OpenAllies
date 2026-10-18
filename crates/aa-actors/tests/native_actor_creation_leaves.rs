//! Replays research/phase8-actor-creation/native-component-leaves.json: every
//! component virtual (+4 InitCreate, +8 InitExisting, +C PostCreate, +14,
//! +20 requirement) of the 4CF480 classes, run natively on fixture
//! components. aa_actors::component_virtuals must make the same calls with
//! the same arguments, return the same value and leave the same component
//! words, actor state, zone vector and RNG.
use aa_actors::actor_creation::{Error, Result};
use aa_actors::component_virtuals::{self, ComponentWorld, LeafParams, leaves};
use aa_actors::components::{ComponentSlot, StateBits};
use aa_actors::init_create::FactoryKind;
use aa_numeric::{Binary, Precision};
use aa_simulation::NativeRng;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const FIXTURE: &str = include_str!("../../../research/phase8-actor-creation/native-component-leaves.json");

fn n(v: &Value) -> u32 {
    v.as_u64().unwrap_or_else(|| panic!("number expected: {v}")) as u32
}
fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}
fn kind_of(v: &Value) -> FactoryKind {
    match v {
        Value::String(s) => match s.as_str() {
            "4org" => FactoryKind::OrganizationMind,
            "4air" => FactoryKind::AircraftMind,
            _ => FactoryKind::ElementMind,
        },
        v => FactoryKind::Standard(ComponentSlot::from_index(n(v) as usize).unwrap()),
    }
}
const SIZES: [u32; 31] = [
    0x44, 0x74, 0x30, 0x8c, 0, 100, 0xdc, 0xdc, 0x34, 0x54, 0x94, 0x20, 0x24, 0x38, 0x88, 0x2c, 0x20, 0x10, 0x10,
    0x18, 0x34, 0x18, 0x18, 0xb4, 0x60, 0x24, 0x14, 0x38, 0x10, 0x24, 0x2c,
];
fn size_of(kind: FactoryKind) -> u32 {
    match kind {
        FactoryKind::OrganizationMind => 0x2c4,
        FactoryKind::AircraftMind => 0x1d8,
        FactoryKind::ElementMind => 0x1bc,
        FactoryKind::Standard(s) => SIZES[s.index()],
    }
}
/// Classes without an Updatable at +8 (Experience; Transport stores only +0).
fn has_updatable(kind: FactoryKind) -> bool {
    !matches!(kind, FactoryKind::Standard(s) if matches!(s.index(), 17 | 24))
        && !matches!(kind, FactoryKind::OrganizationMind | FactoryKind::AircraftMind | FactoryKind::ElementMind)
}

struct Fixture {
    case: Value,
    res: Value,
    kind: FactoryKind,
    calls: Vec<Value>,
    rng: NativeRng,
    comp: BTreeMap<u32, u32>,
    state: StateBits,
    zones: Vec<bool>,
}
impl Fixture {
    fn new(case: &Value) -> Self {
        let kind = kind_of(&case["kind"]);
        let mut comp = BTreeMap::new();
        for off in (0..size_of(kind)).step_by(4) {
            comp.insert(off, 0xa5a5_a5a5u32);
        }
        comp.insert(0, 0); // vtable (not compared)
        comp.insert(4, 0); // actor
        if has_updatable(kind) {
            comp.insert(8, 0);
            comp.insert(0xc, 0);
        }
        let mut defaults = BTreeMap::new();
        if kind == FactoryKind::Standard(ComponentSlot::Denizen) {
            defaults.insert("0x18".to_owned(), json!(0));
            defaults.insert("0x38".to_owned(), json!(0));
        }
        if kind == FactoryKind::Standard(ComponentSlot::Organization) {
            defaults.insert("0x2c".to_owned(), json!(0));
            defaults.insert("0x70".to_owned(), json!(0));
        }
        let mut fields: BTreeMap<String, Value> = defaults;
        if let Some(c) = case["comp"].as_object() {
            for (k, v) in c {
                fields.insert(k.clone(), v.clone());
            }
        }
        for (k, v) in &fields {
            let off = hex(k);
            match v {
                Value::Object(o) => {
                    let w = comp.entry(off & !3).or_insert(0xa5a5_a5a5);
                    let shift = (off & 3) * 8;
                    *w = (*w & !(0xff << shift)) | (n(&o["byte"]) << shift);
                }
                // Lists (Denizen +18): a pointer marker.
                Value::Array(_) => {
                    comp.insert(off, 0x1157);
                }
                v => {
                    comp.insert(off, n(v));
                }
            }
        }
        let a = &case["actor"];
        Self {
            res: case["results"].clone(),
            kind,
            calls: Vec::new(),
            rng: NativeRng::new(case.get("rng").map_or(0x2468ace, n)),
            comp,
            state: StateBits { low: a.get("low").map_or(0, n), high: a.get("high").map_or(0, n) },
            zones: Vec::new(),
            case: case.clone(),
        }
    }
    fn def(&self, off: u32) -> Option<&Value> {
        self.case["def"].get(format!("{off:#x}"))
    }
    fn mine(&self, slot: ComponentSlot) -> Result<()> {
        if slot == component_virtuals::slot_of(self.kind) {
            Ok(())
        } else {
            Err(Error::World(format!("field of another component {slot:?}")))
        }
    }
    fn push(&mut self, v: Value) {
        self.calls.push(v);
    }
}
type A = ();

impl ComponentWorld for Fixture {
    type Actor = A;
    fn world_version(&self) -> u32 {
        self.case.get("version").map_or(0x1a, n)
    }
    fn rng(&mut self) -> &mut NativeRng {
        &mut self.rng
    }
    fn begin_time_bits(&self) -> u32 {
        self.case.get("time").map_or(0x4216_0000, n)
    }
    fn actor_flags(&self, _: A) -> Result<u32> {
        Ok(self.case["actor"].get("flags").map_or(0x0080_0000, n))
    }
    fn state_bits(&self, _: A) -> Result<StateBits> {
        Ok(self.state)
    }
    fn has_component(&self, _: A, slot: ComponentSlot) -> Result<bool> {
        if slot == component_virtuals::slot_of(self.kind) {
            return Ok(true);
        }
        Ok(self.case["actor"]["components"]
            .as_array()
            .is_some_and(|c| c.iter().any(|v| n(v) as usize == slot.index())))
    }
    fn position_xy(&self, _: A) -> Result<[u32; 2]> {
        let a = &self.case["actor"];
        Ok([a.get("x").map_or(0x4206_0000, n), a.get("y").map_or(0xc14c_0000, n)])
    }
    fn owner(&self, _: A) -> Result<u32> {
        Ok(if self.case["actor"].get("owner").is_some() { 0x7000_0001 } else { 0 })
    }
    fn component_word(&self, _: A, slot: ComponentSlot, offset: u32) -> Result<u32> {
        self.mine(slot)?;
        self.comp.get(&offset).copied().ok_or(Error::World(format!("component word {offset:#x}")))
    }
    fn set_component_word(&mut self, _: A, slot: ComponentSlot, offset: u32, value: u32) -> Result<()> {
        self.mine(slot)?;
        self.comp.insert(offset, value);
        Ok(())
    }
    fn component_byte(&self, a: A, slot: ComponentSlot, offset: u32) -> Result<u8> {
        Ok((self.component_word(a, slot, offset & !3)? >> ((offset & 3) * 8)) as u8)
    }
    fn set_component_byte(&mut self, a: A, slot: ComponentSlot, offset: u32, value: u8) -> Result<()> {
        let w = self.component_word(a, slot, offset & !3)?;
        let shift = (offset & 3) * 8;
        self.set_component_word(a, slot, offset & !3, (w & !(0xff << shift)) | (u32::from(value) << shift))
    }
    fn definition_word(&self, _: A, offset: u32) -> Result<u32> {
        Ok(self.def(offset).and_then(|v| v.as_u64()).map_or(0, |v| v as u32))
    }
    fn definition_byte(&self, a: A, offset: u32) -> Result<u8> {
        Ok(self.definition_word(a, offset)? as u8)
    }
    fn definition_list(&self, _: A, offset: u32) -> Result<Vec<u32>> {
        Ok(self.def(offset).map_or(Vec::new(), |v| v[2].as_array().unwrap().iter().map(n).collect()))
    }
    fn recruit_targets(&self, _: A) -> Result<Vec<bool>> {
        Ok(self.case["def"]
            .get("0x4ec_targets")
            .map_or(Vec::new(), |v| v.as_array().unwrap().iter().map(|x| n(x) != 0).collect()))
    }
    fn unit_supply_enabled(&self) -> Result<bool> {
        Ok(self.case.get("unit_supply").map_or(1, n) != 0)
    }
    fn scheduled(&self, _: A, slot: ComponentSlot) -> Result<bool> {
        self.mine(slot)?;
        Ok(self.comp[&0xc] != 0)
    }
    fn add_periodic(&mut self, _: A, _: ComponentSlot, queue: u8, data: u32) -> Result<()> {
        self.push(json!(["periodic", queue, data]));
        Ok(())
    }
    fn add_after(&mut self, _: A, _: ComponentSlot, delay: u32, data: u32) -> Result<()> {
        self.push(json!(["after", delay, data]));
        Ok(())
    }
    fn has_scheduled(&mut self, _: A, _: ComponentSlot, queue: u8, data: u32) -> Result<bool> {
        self.push(json!(["has", queue, data]));
        Ok(self.res.get("50B220").map_or(0, n) != 0)
    }
    fn create_zone(&mut self, _: A, index: u32) -> Result<bool> {
        if self.zones.is_empty() {
            self.zones = vec![false; 9];
        }
        if !self.zones[index as usize] {
            self.push(json!(["zone ctor", 1u32 << index]));
            self.zones[index as usize] = true;
        }
        Ok(true)
    }
    fn zone_present(&self, _: A, index: u32) -> Result<bool> {
        Ok(self.zones.get(index as usize).copied().unwrap_or(false))
    }
    fn move_zone(&mut self, _: A, index: u32, xy: [u32; 2], radius: u32) -> Result<()> {
        self.push(json!(["515F40", index, [xy[0], xy[1], radius]]));
        Ok(())
    }
    fn zone_flag(&mut self, _: A, index: u32, bit: u32, on: u32) -> Result<()> {
        self.push(json!(["515940", index, [bit, on]]));
        Ok(())
    }
    fn thing_manager_74_bits(&self) -> Result<u32> {
        Ok(self.case.get("thing_manager_74").map_or(0x3fc0_0000, n))
    }
    fn tile_size_bits(&self) -> Result<u32> {
        Ok(self.case.get("tile_size").map_or(0x4100_0000, n))
    }
    fn set_state(&mut self, _: A, state: u32, enabled: bool) -> Result<()> {
        self.push(json!(["4D64F0", state, u32::from(enabled)]));
        Ok(())
    }
    fn query_4d1050(&mut self, _: A) -> Result<bool> {
        self.push(json!(["4D1050"]));
        Ok(self.res.get("4D1050").map_or(0, n) != 0)
    }
    fn refresh_4d4cf0(&mut self, _: A) -> Result<()> {
        self.push(json!(["4D4CF0"]));
        Ok(())
    }
    fn modifier_context(&mut self, _: A) -> Result<u32> {
        self.push(json!(["4CF110"]));
        Ok(self.res.get("4CF110").map_or(0x4cf110, n))
    }
    fn log_settlement_walls(&mut self, _: A) -> Result<()> {
        self.push(json!(["log", 0x68]));
        Ok(())
    }
    fn body_set_health(&mut self, _: A, r: u32, t: u32, no: u32, s: u32) -> Result<()> {
        self.push(json!(["561E50", r, t, no, s]));
        Ok(())
    }
    fn construction_565070(&mut self, _: A) -> Result<Option<u32>> {
        self.push(json!(["565070"]));
        Ok(self.res.get("565070").map(n).filter(|v| *v != 0))
    }
    fn body_561d40(&mut self, _: A, context: u32, value: u32) -> Result<()> {
        self.push(json!(["561D40", context, value]));
        Ok(())
    }
    fn properties_581420(&mut self, _: A) -> Result<()> {
        self.push(json!(["581420"]));
        Ok(())
    }
    fn properties_add_initial(&mut self, _: A, p: u32) -> Result<()> {
        self.push(json!(["props add", p]));
        Ok(())
    }
    fn denizen_slot_count(&self, _: A) -> Result<usize> {
        Ok(self.case["comp"].get("0x18").map_or(0, |v| v[2].as_array().unwrap().len()))
    }
    fn denizen_target_health(&mut self, _: A, index: usize) -> Result<Binary> {
        let target = n(&self.case["comp"]["0x18"][2][index]);
        self.push(json!(["45D730", target]));
        Ok(Binary::from_f32_bits(self.res.get("target_health").map_or(0x40d0_0000, n)).unwrap())
    }
    fn set_denizen_slot_health(&mut self, _: A, index: usize, bits: u32) -> Result<()> {
        self.push(json!(["slot health", index, bits]));
        Ok(())
    }
    fn settlement_create_walls(&mut self, _: A) -> Result<bool> {
        self.push(json!(["586BC0"]));
        Ok(self.res.get("586BC0").map_or(1, n) != 0)
    }
    fn settlement_584cf0(&mut self, _: A) -> Result<()> {
        self.push(json!(["584CF0"]));
        Ok(())
    }
    fn supply_58ae10(&mut self, _: A) -> Result<()> {
        self.push(json!(["58AE10"]));
        Ok(())
    }
    fn owner_hq_table(&self, _: u32) -> Result<bool> {
        Ok(self.case["actor"]["owner"].get("hq_slots").is_some())
    }
    fn allocate_hq_index(&mut self, _: u32) -> Result<i32> {
        self.push(json!(["4E2640"]));
        Ok(self.res.get("hq_index").map_or(3, n) as i32)
    }
    fn hq_no_badge_message(&mut self, _: A, _: u32) -> Result<()> {
        self.push(json!(["hq message"]));
        Ok(())
    }
    fn performer_gate_by_rank(&mut self, _: A) -> Result<()> {
        self.push(json!(["57DCF0"]));
        Ok(())
    }
    fn performer_refresh(&mut self, _: A) -> Result<()> {
        self.push(json!(["57E430"]));
        Ok(())
    }
    fn organization_set_definition_layout(&mut self, _: A) -> Result<()> {
        self.push(json!(["57C450"]));
        Ok(())
    }
    fn organization_57b390(&mut self, _: A) -> Result<()> {
        self.push(json!(["57B390"]));
        Ok(())
    }
    fn organization_70_flag(&self, _: A) -> Result<u8> {
        Ok(0)
    }
    fn organization_577f30(&mut self, _: A, flag: u8) -> Result<()> {
        self.push(json!(["577F30", flag]));
        Ok(())
    }
    fn organization_members(&self, _: A) -> Result<Vec<A>> {
        Ok(if self.case.get("members").is_some() { vec![(), ()] } else { vec![] })
    }
    fn element_5709d0(&mut self, _: A) -> Result<()> {
        self.push(json!(["5709D0"]));
        Ok(())
    }
    fn element_range(&mut self, _: A, context: u32) -> Result<Binary> {
        self.push(json!(["45D5B0", context]));
        Ok(Binary::from_f32_bits(self.res.get("element_range").map_or(0x40d0_0000, n)).unwrap())
    }
    fn organization_range(&mut self, _: A) -> Result<Binary> {
        self.push(json!(["45D4C0"]));
        Ok(Binary::from_f32_bits(self.res.get("organization_range").map_or(0x40d0_0000, n)).unwrap())
    }
    fn defender_range(&mut self, _: A) -> Result<Binary> {
        self.push(json!(["45D6C0"]));
        Ok(Binary::from_f32_bits(self.res.get("defender_range").map_or(0x40d0_0000, n)).unwrap())
    }
    fn experience_rank(&mut self, _: A, value: u32) -> Result<u32> {
        self.push(json!(["45DB90", value]));
        Ok(self.res.get("45DB90").map_or(2, n))
    }
    fn experience_set_rank(&mut self, _: A, rank: u32) -> Result<()> {
        self.push(json!(["5716B0", rank]));
        Ok(())
    }
    fn mover_24(&self, _: A) -> Result<u32> {
        Ok(self.case["actor"].get("mover_24").map_or(0x3e80_0000, n))
    }
    fn mover_576b20(&mut self, _: A, v: u32, s: u32) -> Result<()> {
        self.push(json!(["576B20", v, s]));
        Ok(())
    }
    fn mind_init_existing(&mut self, _: A, _: FactoryKind) -> Result<bool> {
        self.push(json!(["mind"]));
        Ok(self.res.get("mind").map_or(1, n) != 0)
    }
    fn wall_grid_init(&mut self, _: A) -> Result<()> {
        self.push(json!(["wall grid"]));
        Ok(())
    }
    fn gate_zone_radius(&mut self) -> Result<u32> {
        if self.case.get("gate_latch").map_or(1, n) & 1 == 0 {
            self.push(json!(["4BE0D0"]));
        }
        self.push(json!(["4BE0F0"]));
        Ok(self.res.get("gate_radius").map_or(0x4140_0000, n))
    }
    fn detection_activate(&mut self, _: A) -> Result<()> {
        Err(Error::Unsupported("56EC90 is aa_zones::component::activate (not in this fixture)"))
    }
    fn city_create_with_field(&mut self, _: A, w: u32) -> Result<()> {
        self.push(json!(["5644F0", w]));
        Ok(())
    }
}

fn normalize(calls: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for c in calls {
        let seam = c["seam"].as_str().unwrap();
        out.push(match seam {
            "50AC60" => json!(["periodic", 1, c["data"]]),
            "50AC00" => json!(["periodic", 0, c["data"]]),
            "50ACC0" => json!(["after", c["delay"], c["data"]]),
            "50B220" => json!(["has", c["queue"], c["data"]]),
            "581420" => json!(["581420"]),
            "5817A0" => json!(["props add", c["args"][0]]),
            "561E50" => json!(["561E50", c["args"][0], c["args"][1], c["args"][2], c["args"][3]]),
            "561D40" => json!(["561D40", c["args"][0], c["args"][1]]),
            "45D730" => json!(["45D730", c["ecx"]]),
            "45D5B0" => json!(["45D5B0", c["arg"]]),
            "577F30" => json!(["577F30", c["args"][0]]),
            "576B20" => json!(["576B20", c["args"][0], c["args"][1]]),
            "45DB90" => json!(["45DB90", c["args"][0]]),
            "5716B0" => json!(["5716B0", c["args"][0]]),
            "4D64F0" => json!(["4D64F0", c["args"][0], c["args"][1]]),
            "zone ctor" => json!(["zone ctor", c["mask"]]),
            "515F40" => json!(["515F40", c["zone"], c["xyz"]]),
            "515940" => json!(["515940", c["zone"], c["args"]]),
            "actor name" | "4B9F40" => continue,
            "40C230" => json!(["hq message"]),
            "5644F0" => json!(["5644F0", c["args"][1]]),
            "mind 529EC0" | "mind 517C60" => json!(["mind"]),
            "wall grid 58CC40" => json!(["wall grid"]),
            "404ED0" => json!(["log", c["line"]]),
            other => json!([other]),
        });
    }
    out
}

fn run(row: &Value) {
    let label = row["label"].as_str().unwrap();
    let case = &row["case"];
    let r = &row["result"];
    assert!(r["outcome"].is_null() && r["unexpected"].as_array().unwrap().is_empty(), "{label}");
    let mut w = Fixture::new(case);
    let kind = w.kind;
    let off = n(&case["virtual"]);
    let l = leaves(kind);
    let leaf = match off {
        4 => l.init_create,
        8 => l.init_existing,
        0xc => l.post_create,
        0x14 => l.vf14,
        _ => l.requirement,
    };
    assert_eq!(format!("{leaf:#x}"), r["leaf"].as_str().unwrap(), "{label}: vtable entry");
    let p = &case["params"];
    let params = LeafParams {
        word_34: p.get("w34").map_or(u32::MAX, n),
        mode: p.get("mode").map_or(0, n),
        word_40: p.get("w40").map_or(0, n),
        word_48: p.get("w48").map_or(0, n),
    };
    let pc = match case.get("pc").and_then(|v| v.as_str()).unwrap_or("Double53") {
        "Single24" => Precision::Single24,
        "Extended64" => Precision::Extended64,
        _ => Precision::Double53,
    };
    let returned: Option<u32> = match off {
        4 => {
            let pp = (p.get("null").is_none()).then_some(&params);
            Some(u32::from(component_virtuals::init_create(kind, (), pp, &mut w).unwrap_or_else(|e| panic!("{label}: {e}"))))
        }
        8 => Some(u32::from(component_virtuals::init_existing(kind, (), &mut w, pc).unwrap_or_else(|e| panic!("{label}: {e}")))),
        0xc => {
            component_virtuals::post_create(kind, (), &params, case.get("flag").map_or(0, n), &mut w)
                .unwrap_or_else(|e| panic!("{label}: {e}"));
            None
        }
        0x14 => {
            component_virtuals::vf14(kind, (), &mut w).unwrap_or_else(|e| panic!("{label}: {e}"));
            None
        }
        _ => Some(
            component_virtuals::requirement(kind, (), &mut w)
                .unwrap()
                .map_or(u32::MAX, |s| s.index() as u32),
        ),
    };
    if let Some(v) = returned {
        assert_eq!(v, n(&r["returned"]), "{label}: returned");
    }
    let expected = normalize(r["calls"].as_array().unwrap());
    // Denizen InitCreate's slot health stores are memory writes natively
    // (checked through the component image instead).
    let actual: Vec<Value> = w.calls.iter().filter(|c| c[0] != "slot health").cloned().collect();
    assert_eq!(expected, actual, "{label}: calls");
    if let Some(slots) = r.get("denizen_slot_health") {
        let stored: Vec<Value> = w.calls.iter().filter(|c| c[0] == "slot health").map(|c| c[2].clone()).collect();
        let native: Vec<Value> = slots.as_array().unwrap().iter().filter(|v| n(v) != 0xa5a5_a5a5).cloned().collect();
        assert_eq!(native, stored, "{label}: Denizen slot health");
    }
    assert_eq!(w.rng.state, n(&r["rng_after"]), "{label}: rng");
    let words = r["comp"].as_array().unwrap();
    for (i, v) in words.iter().enumerate() {
        let off = 4 * i as u32;
        if off < 0x10 && off != 0xc || (off == 0xc && !has_updatable(kind)) && off < 0x10 {
            continue;
        }
        if off == 0xc && has_updatable(kind) {
            continue; // the event-list head (seamed queues do not link nodes)
        }
        if let Some(Value::Array(_)) = case["comp"].get(format!("{off:#x}")) {
            continue; // list head pointer
        }
        if case.get("members").is_some() && off == 0x28 {
            continue;
        }
        assert_eq!(w.comp[&off], n(v), "{label}: component word {off:#x}");
    }
    let za = &r["actor"];
    assert_eq!(w.zones.len() as u32, n(&za["zones"]), "{label}: zone vector size");
}

#[test]
fn native_component_leaves_replay() {
    let doc: Value = serde_json::from_str(FIXTURE).unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() > 200);
    let mut ran = 0;
    for row in cases {
        // Detection +14 is aa_zones::component::activate (aa-zones oracle).
        if row["case"]["kind"] == 22 && n(&row["case"]["virtual"]) == 0x14 {
            continue;
        }
        run(row);
        ran += 1;
    }
    assert!(ran > 200);
}
