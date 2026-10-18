//! Replays research/phase8-sai/native-base-priority.json (original goal
//! vtable +78 bodies under Unicorn) against aa_mind::sai::base_priority.
//! Every virtual call and seam is answered from the recorded events in
//! lockstep; object fields come from the case.
use aa_mind::{
    Result,
    sai::base_priority::{
        self as bp, BasePriorityWorld, ClassGoal, GeneralPowerInstance, Obj, PlayerContext,
    },
    sai::base_priority_misc::{self as misc, ClassPriorityWorld},
    sai::base_priority_more::{self as more, DestroyStatics, MorePriorityWorld},
    sai::manager::SaiRng,
    sai::ego::Ego,
    sai::native_hash::NativeHash,
    x87::binary_from_x87,
};
use aa_numeric::{Binary, Precision, crt_pow::CrtEnvironment};
use serde_json::{Map, Value, json};
use std::collections::VecDeque;

struct Host {
    c: Value,
    expected: VecDeque<Value>,
    case: usize,
    rng: SaiRng,
}

fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn id(o: Option<Obj>) -> u32 {
    o.unwrap_or(0)
}
fn opt(v: &Value) -> Option<Obj> {
    let x = u(v);
    (x != 0).then_some(x)
}
fn x87(v: &Value) -> Binary {
    binary_from_x87(
        v["significand"].as_u64().unwrap(),
        v["sign_exponent"].as_u64().unwrap() as u16,
    )
    .unwrap()
}

impl Host {
    fn obj(&self, o: Obj) -> &Value {
        let v = &self.c["objects"][o.to_string()];
        assert!(!v.is_null(), "case {}: unknown object {o}", self.case);
        v
    }
    fn next(&mut self, op: &str, args: Value) -> Value {
        let e = self
            .expected
            .pop_front()
            .unwrap_or_else(|| panic!("case {}: unexpected {op} {args}", self.case));
        let mut exp: Map<String, Value> = e.as_object().unwrap().clone();
        let result = exp.remove("result").unwrap_or(Value::Null);
        let mut act: Map<String, Value> = args.as_object().unwrap().clone();
        act.insert("op".into(), json!(op));
        assert_eq!(Value::Object(exp), Value::Object(act), "case {}", self.case);
        result
    }
    fn gps(&self, key: &str) -> Vec<GeneralPowerInstance> {
        self.c[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| GeneralPowerInstance {
                caster: u(&g["caster"]),
                side: u(&g["side"]),
                kind: u(&g["kind"]),
                bonus: u(&g["bonus"]),
                target: u(&g["target"]),
                targets_actor: g["targets_actor"].as_bool().unwrap(),
                owner: u(&g["owner"]),
                point: [u(&g["point"][0]), u(&g["point"][1])],
                has_point: g["has_point"].as_bool().unwrap(),
                additive: g["additive"].as_bool().unwrap(),
            })
            .collect()
    }
}

impl BasePriorityWorld for Host {
    fn resolve(&mut self, gtid: u32) -> Result<Option<Obj>> {
        let entry = &self.c["registry"][(gtid & 0xffff).to_string()];
        if entry.is_null() {
            return Ok(None);
        }
        let (serial, obj) = (u(&entry[0]), u(&entry[1]));
        Ok((serial == gtid >> 16 && obj != 0).then_some(obj))
    }
    fn flags(&mut self, obj: Obj) -> Result<u32> {
        Ok(u(&self.obj(obj)["flags"]))
    }
    fn object_gtid(&mut self, obj: Obj) -> Result<u32> {
        Ok(u(&self.obj(obj)["gtid"]))
    }
    fn sai_general_powers(&mut self) -> Result<Vec<GeneralPowerInstance>> {
        Ok(self.gps("sai_gps"))
    }
    fn player_general_powers(&mut self) -> Result<Vec<GeneralPowerInstance>> {
        Ok(self.gps("player_gps"))
    }
    fn game_time(&mut self) -> Result<u32> {
        Ok(u(&self.c["time"]))
    }
    fn map_ready(&mut self) -> Result<bool> {
        Ok(self.c["map_ready"].as_bool().unwrap())
    }
    fn position(&mut self, obj: Obj) -> Result<[u32; 2]> {
        let p = &self.obj(obj)["pos"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn flags_100(&mut self, obj: Obj) -> Result<u32> {
        Ok(u(&self.obj(obj)["f100"]))
    }
    fn flags_104(&mut self, obj: Obj) -> Result<u32> {
        Ok(u(&self.obj(obj)["f104"]))
    }
    fn has_118(&mut self, obj: Obj) -> Result<bool> {
        Ok(u(&self.obj(obj)["w118"]) != 0)
    }
    fn component_7c(&mut self, obj: Obj) -> Result<Option<Obj>> {
        Ok(opt(&self.obj(obj)["c7c"]))
    }
    fn component_80_10(&mut self, obj: Obj) -> Result<Option<Obj>> {
        let v = &self.obj(obj)["c80"];
        Ok(if v.is_null() { None } else { opt(v) })
    }
    fn definition_2f4(&mut self, obj: Obj) -> Result<bool> {
        Ok(self.obj(obj)["def2f4"].as_bool().unwrap())
    }
    fn component_5c_60(&mut self, component: Obj) -> Result<[u32; 2]> {
        let c = self.obj(component);
        Ok([u(&c["c5c"]), u(&c["c60"])])
    }
    fn record_mask(&mut self, record: Obj) -> Result<u64> {
        Ok(self.obj(record)["mask"].as_u64().unwrap())
    }
    fn record_object(&mut self, record: Obj) -> Result<u32> {
        Ok(u(&self.obj(record)["object"]))
    }
    fn record_route_state(&mut self, record: Obj) -> Result<Option<u32>> {
        let v = &self.obj(record)["route"];
        Ok(if v.is_null() { None } else { Some(u(v)) })
    }
    fn entry_state(&mut self, region: Obj) -> Result<u32> {
        Ok(u(&self.obj(region)["entry_state"]))
    }
    fn goal_expired(&mut self, _goal: u32) -> Result<bool> {
        Ok(self.next("goal_expired", json!({})).as_bool().unwrap())
    }
    fn goal_virtual_18(&mut self, _goal: u32, flag: bool) -> Result<()> {
        self.next("goal_virtual_18", json!({"flag": u8::from(flag)}));
        Ok(())
    }
    fn goal_location(&mut self, _goal: u32) -> Result<Option<Obj>> {
        Ok(opt(&self.next("goal_location", json!({}))))
    }
    fn virtual_58(&mut self, obj: Obj, other: Option<Obj>, flag: bool) -> Result<bool> {
        let r = self.next(
            "virtual_58",
            json!({"obj": obj, "other": id(other), "flag": u8::from(flag)}),
        );
        Ok(r.as_bool().unwrap())
    }
    fn virtual_70(&mut self, obj: Obj, other: Option<Obj>, flag: bool) -> Result<bool> {
        let r = self.next(
            "virtual_70",
            json!({"obj": obj, "other": id(other), "flag": u8::from(flag)}),
        );
        Ok(r.as_bool().unwrap())
    }
    fn virtual_100(&mut self, obj: Obj) -> Result<Option<Obj>> {
        Ok(opt(&self.next("virtual_100", json!({"obj": obj}))))
    }
    fn virtual_108(&mut self, obj: Obj) -> Result<Option<Obj>> {
        Ok(opt(&self.next("virtual_108", json!({"obj": obj}))))
    }
    fn virtual_138(&mut self, obj: Obj, other: Option<Obj>) -> Result<bool> {
        let r = self.next("virtual_138", json!({"obj": obj, "other": id(other)}));
        Ok(r.as_bool().unwrap())
    }
    fn virtual_140(&mut self, obj: Obj, other: Option<Obj>) -> Result<bool> {
        let r = self.next("virtual_140", json!({"obj": obj, "other": id(other)}));
        Ok(r.as_bool().unwrap())
    }
    fn record_dead(&mut self, record: Obj) -> Result<bool> {
        Ok(self
            .next("record_dead", json!({"record": record}))
            .as_bool()
            .unwrap())
    }
    fn unit_check(&mut self, obj: Obj) -> Result<bool> {
        Ok(self
            .next("unit_check", json!({"obj": obj}))
            .as_bool()
            .unwrap())
    }
    fn seen_record(&mut self, obj: Option<Obj>, kingdom: Option<Obj>) -> Result<Option<Obj>> {
        Ok(opt(&self.next(
            "seen_record",
            json!({"obj": id(obj), "kingdom": id(kingdom)}),
        )))
    }
    fn region_at(&mut self, point: [u32; 2]) -> Result<Option<Obj>> {
        Ok(opt(&self.next("region_at", json!({"point": point}))))
    }
    fn region_index(&mut self, region: Obj) -> Result<u32> {
        Ok(u(&self.obj(region)["index"]))
    }
    fn route_hops(&mut self, start: u32, goal: u32) -> Result<Option<i32>> {
        let r = self.next("route_hops", json!({"start": start, "goal": goal}));
        Ok(r.as_i64().map(|h| h as i32))
    }
    fn health(&mut self, obj: Obj, flag: bool) -> Result<Binary> {
        let r = self.next("health", json!({"obj": obj, "flag": u8::from(flag)}));
        Ok(x87(&r))
    }
    fn attack_value(&mut self, target: Option<Obj>) -> Result<Binary> {
        Ok(x87(&self.next("attack_value", json!({"target": id(target)}))))
    }
    fn threat_value(&mut self, actor: Option<Obj>, flag: bool) -> Result<Binary> {
        Ok(x87(&self.next(
            "threat_value",
            json!({"actor": id(actor), "flag": u8::from(flag)}),
        )))
    }
    fn repair_value(&mut self, actor: Option<Obj>) -> Result<Binary> {
        Ok(x87(&self.next("repair_value", json!({"actor": id(actor)}))))
    }
    fn region_threat(&mut self, region: Obj) -> Result<(Binary, Option<Obj>)> {
        let r = self.next("region_threat", json!({"region": region}));
        Ok((x87(&r[0]), opt(&r[1])))
    }
}

fn table(v: &Value) -> NativeHash<u32> {
    let mut t = NativeHash::new(u(&v["n"]));
    for (i, chain) in v["buckets"].as_array().unwrap().iter().enumerate() {
        for node in chain.as_array().unwrap() {
            t.buckets[i].push((u(&node[0]), u(&node[1])));
            t.count += 1;
        }
    }
    t
}

impl ClassPriorityWorld for Host {
    fn first_word(&mut self, obj: Obj) -> Result<u32> {
        Ok(u(&self.obj(obj)["key"]))
    }
    fn component_40(&mut self, component: Obj) -> Result<Option<Obj>> {
        Ok(opt(&self.obj(component)["c40"]))
    }
    fn word_84(&mut self, obj: Obj) -> Result<u32> {
        Ok(u(&self.obj(obj)["w84"]))
    }
    fn word_98(&mut self, obj: Obj) -> Result<u32> {
        Ok(if self.obj(obj)["org"].is_null() { 0 } else { 100_000 + obj })
    }
    fn word_128(&mut self, obj: Obj) -> Result<u32> {
        Ok(u(&self.obj(obj)["w128"]))
    }
    fn definition_548(&mut self, obj: Obj) -> Result<u32> {
        Ok(u(&self.obj(obj)["def548"]))
    }
    fn definition_348(&mut self, obj: Obj) -> Result<bool> {
        Ok(self.obj(obj)["def348"].as_bool().unwrap())
    }
    fn definition(&mut self, obj: Obj) -> Result<Obj> {
        Ok(u(&self.obj(obj)["definition"]))
    }
    fn component_d0(&mut self, component: Obj) -> Result<u32> {
        Ok(u(&self.obj(component)["cd0"]))
    }
    fn component_d8(&mut self, obj: Obj) -> Result<Obj> {
        Ok(u(&self.obj(obj)["cd8"]))
    }
    fn sai_record(&mut self, obj: Obj) -> Result<Option<Obj>> {
        Ok(opt(&self.obj(obj)["record"]))
    }
    fn record_table(&mut self, record: Obj) -> Result<Option<NativeHash<u32>>> {
        let v = &self.obj(record)["table"];
        Ok(if v.is_null() { None } else { Some(table(v)) })
    }
    fn hq_slots(&mut self, player: Obj) -> Result<Vec<Option<Obj>>> {
        Ok(self.obj(player)["hqs"].as_array().unwrap().iter().map(opt).collect())
    }
    fn map_extent(&mut self) -> Result<[u32; 2]> {
        Ok([u(&self.c["extent"][0]), u(&self.c["extent"][1])])
    }
    fn tile_zone(&mut self, point: [u32; 2], kingdom: Option<Obj>) -> Result<bool> {
        let r = self.next(
            "tile_zone",
            json!({"point": point, "kingdom": id(kingdom), "flag": 0}),
        );
        Ok(r.as_bool().unwrap())
    }
    fn sai_query_79e590(&mut self, kingdom: Option<Obj>) -> Result<bool> {
        let r = self.next("sai_query_79e590", json!({"kingdom": id(kingdom)}));
        Ok(r.as_bool().unwrap())
    }
    fn hq_accepts(&mut self, component: Obj, actor: Option<Obj>) -> Result<bool> {
        let r = self.next(
            "hq_accepts",
            json!({"component": component, "actor": id(actor)}),
        );
        Ok(r.as_bool().unwrap())
    }
    fn value_4d01e0(&mut self, obj: Obj, flag: bool) -> Result<Binary> {
        Ok(x87(&self.next(
            "value_4d01e0",
            json!({"obj": obj, "flag": u8::from(flag)}),
        )))
    }
    fn give_resources_value(
        &mut self,
        kingdom: Option<Obj>,
        actor: Option<Obj>,
        amount: u32,
    ) -> Result<Binary> {
        Ok(x87(&self.next(
            "give_resources_value",
            json!({"kingdom": id(kingdom), "actor": id(actor), "amount": amount}),
        )))
    }
    fn give_actor_value(
        &mut self,
        kingdom: Option<Obj>,
        recipient: Option<Obj>,
        given: Option<Obj>,
        player: Obj,
    ) -> Result<Binary> {
        Ok(x87(&self.next(
            "give_actor_value",
            json!({"kingdom": id(kingdom), "recipient": id(recipient), "given": id(given), "player": player}),
        )))
    }
    fn definition_value(&mut self, definition: Obj, player: Obj, flag: bool) -> Result<Binary> {
        Ok(x87(&self.next(
            "definition_value",
            json!({"definition": definition, "player": player, "flag": u8::from(flag)}),
        )))
    }
}

impl MorePriorityWorld for Host {
    fn exploration_setting(&mut self) -> Result<u32> {
        Ok(u(&self.c["explore_setting"]))
    }
    fn primary_island_regions(&mut self) -> Result<Vec<Option<Obj>>> {
        Ok(self.c["islands"][0].as_array().unwrap().iter().map(opt).collect())
    }
    fn region_exploration(&mut self, region: Obj, _kingdom: Option<Obj>) -> Result<Option<(i32, i32)>> {
        let r = self.obj(region);
        Ok((!r["explored"].is_null()).then(|| (u(&r["explored"]) as i32, u(&r["area"]) as i32)))
    }
    fn organization_14(&mut self, obj: Obj) -> Result<Option<Option<Obj>>> {
        let o = &self.obj(obj)["org"];
        Ok((!o.is_null()).then(|| opt(o)))
    }
    fn component_9c(&mut self, obj: Obj) -> Result<Obj> {
        Ok(200_000 + obj)
    }
    fn record_table_object(&mut self, record: Obj) -> Result<Option<Obj>> {
        Ok((!self.obj(record)["table"].is_null()).then_some(300_000 + record))
    }
    fn record_byte_28(&mut self, record: Obj) -> Result<bool> {
        Ok(self.obj(record)["b28"].as_bool().unwrap())
    }
    fn entry_word(&mut self, entry: Obj, offset: u32) -> Result<u32> {
        Ok(u(&self.obj(entry)[format!("w{offset:x}")]))
    }
    fn economy_arrays(&mut self, kingdom: Obj) -> Result<[Vec<u32>; 3]> {
        let e = &self.obj(kingdom)["economy"];
        let v = |i: usize| e[i].as_array().unwrap().iter().map(u).collect::<Vec<_>>();
        Ok([v(0), v(1), v(2)])
    }
    fn definition_534(&mut self, obj: Obj) -> Result<u32> {
        Ok(u(&self.obj(obj)["def534"]))
    }
    fn island_regions(&mut self, region: Obj) -> Result<Vec<Option<Obj>>> {
        let island = u(&self.obj(region)["island"]) as usize;
        Ok(self.c["islands"][island].as_array().unwrap().iter().map(opt).collect())
    }
    fn sai_random(&mut self) -> Result<u32> {
        Ok(self.rng.next_u32())
    }
    fn virtual_34(&mut self, obj: Obj, kind: u32) -> Result<bool> {
        Ok(self.next("virtual_34", json!({"obj": obj, "kind": kind})).as_bool().unwrap())
    }
    fn record_region(&mut self, record: Obj) -> Result<Obj> {
        Ok(u(&self.next("record_region", json!({"record": record}))))
    }
    fn can_build(&mut self, component: Obj, definition: u32) -> Result<bool> {
        let r = self.next(
            "can_build",
            json!({"component": component, "definition": definition,
                   "point": [0xbf80_0000u32, 0xbf80_0000u32], "rest": [0, 0, 0]}),
        );
        Ok(r.as_bool().unwrap())
    }
    fn build_site(&mut self, definition: u32, kingdom: Option<Obj>, builder: Option<Obj>) -> Result<Option<[u32; 2]>> {
        let r = self.next(
            "build_site",
            json!({"definition": definition, "kingdom": id(kingdom), "builder": id(builder)}),
        );
        Ok((!r.is_null()).then(|| [u(&r[0]), u(&r[1])]))
    }
    fn unpack_site(&mut self, layout: u32, kingdom: Option<Obj>, region: Obj) -> Result<Option<[u32; 2]>> {
        let r = self.next(
            "unpack_site",
            json!({"layout": layout, "kingdom": id(kingdom), "region": region}),
        );
        Ok((!r.is_null()).then(|| [u(&r[0]), u(&r[1])]))
    }
    fn can_upgrade(&mut self, component: Obj, definition: u32) -> Result<bool> {
        let r = self.next("can_upgrade", json!({"component": component, "definition": definition}));
        Ok(r.as_bool().unwrap())
    }
    fn regions_within(&mut self, region: Obj, depth: u32) -> Result<Vec<Obj>> {
        let r = self.next("regions_within", json!({"region": region, "depth": depth}));
        Ok(r.as_array().unwrap().iter().map(u).collect())
    }
    fn build_value(&mut self, definition: u32, point: [u32; 2]) -> Result<Binary> {
        Ok(x87(&self.next("build_value", json!({"definition": definition, "point": point}))))
    }
    fn upgrade_value(&mut self, actor: Option<Obj>, definition: u32) -> Result<Binary> {
        Ok(x87(&self.next("upgrade_value", json!({"actor": id(actor), "definition": definition}))))
    }
    fn template_value(&mut self, table: Obj, record: Obj, definition: u32) -> Result<Binary> {
        Ok(x87(&self.next(
            "template_value",
            json!({"table": table, "record": record, "definition": definition}),
        )))
    }
    fn template_missing(&mut self, table: Obj, record: Obj) -> Result<Option<Obj>> {
        Ok(opt(&self.next("template_missing", json!({"table": table, "record": record}))))
    }
    fn template_has(&mut self, table: Obj, definition: Obj) -> Result<bool> {
        let r = self.next("template_has", json!({"table": table, "definition": definition}));
        Ok(r.as_bool().unwrap())
    }
    fn destroy_value(&mut self, target: Option<Obj>, differences: &[u32], economy: &[u32]) -> Result<Binary> {
        Ok(x87(&self.next(
            "destroy_value",
            json!({"target": id(target), "differences": differences, "economy": economy}),
        )))
    }
    fn destroy_record_value(&mut self, record: Option<Obj>) -> Result<Binary> {
        Ok(x87(&self.next("destroy_record_value", json!({"record": id(record)}))))
    }
    fn unpack_value(&mut self, record: Obj, region: Obj, point: [u32; 2], same: bool) -> Result<Binary> {
        Ok(x87(&self.next(
            "unpack_value",
            json!({"record": record, "region": region, "point": point, "same": u8::from(same)}),
        )))
    }
}

fn ego_of(c: &Value) -> Ego {
    let mut e = Ego::new("case");
    e.image = vec![0; e.image.len()];
    for (off, v) in c["ego"].as_object().unwrap() {
        e.set_u32(off.parse().unwrap(), u(v));
    }
    e.set_u8(0x7a4, c["ego_7a4"].as_u64().unwrap() as u8);
    e.table_830 = table(&c["ego_table_830"]);
    e
}

#[test]
fn native_base_priorities_match_original() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-sai/native-base-priority.json"
    );
    let f: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert!(cases.len() >= 19 * 100, "{}", cases.len());
    let mut faults = 0;
    let mut nans = 0;
    for (i, row) in cases.iter().enumerate() {
        let c = &row["input"];
        let o = &row["output"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let ego = ego_of(c);
        let ctx = PlayerContext {
            player: u(&c["player"]["id"]),
            actor: opt(&c["player"]["actor"]),
            ego: &ego,
            crt: CrtEnvironment {
                x87_control: match p {
                    Precision::Single24 => 0x007f,
                    Precision::Double53 => 0x027f,
                    Precision::Extended64 => 0x037f,
                },
                ..CrtEnvironment::SSE2_DOUBLE53
            },
        };
        let mut goal = ClassGoal {
            id: 1,
            priority: u(&c["goal"]["priority"]),
            words: c["goal"]["words"].as_array().unwrap().iter().map(u).collect(),
        };
        let mut host = Host {
            c: c.clone(),
            expected: o["events"].as_array().unwrap().iter().cloned().collect(),
            case: i,
            rng: SaiRng { state: u(&c["rng"]) },
        };
        let mut statics = (!c["statics"].is_null()).then(|| DestroyStatics {
            differences: c["statics"][0].as_array().unwrap().iter().map(u).collect(),
            economy: c["statics"][1].as_array().unwrap().iter().map(u).collect(),
        });
        let result = match c["class"].as_str().unwrap() {
            "ATTACK_REGION" => bp::attack_region(&mut host, &mut goal, &ctx, p),
            "ATTACK_ACTOR" => bp::attack_actor(&mut host, &mut goal, &ctx, p),
            "DEFEND_REGION" => bp::defend_region(&mut host, &mut goal, &ctx, p),
            "DEFEND_ACTOR" => bp::defend_actor(&mut host, &mut goal, &ctx, p),
            "AIR_ATTACK" => bp::air_attack(&mut host, &mut goal, &ctx, p),
            "RECOVER" => bp::recover(&mut host, &mut goal, &ctx, p),
            "REPAIR" => bp::repair(&mut host, &mut goal, &ctx, p),
            "BOMBARD" => bp::bombard(&mut host, &mut goal, &ctx, p),
            "PACK" => misc::pack(&mut host, &mut goal, &ctx),
            "RECRUIT" => misc::recruit(&mut goal),
            "GIVE_RESOURCES" => misc::give_resources(&mut host, &mut goal, &ctx),
            "GIVE_ACTOR" => misc::give_actor(&mut host, &mut goal, &ctx),
            "RESEARCH" => misc::research(&mut host, &mut goal, &ctx, p),
            "ATTACH" => misc::attach(&mut host, &mut goal, &ctx, p),
            "EXPLORE" => more::explore(&mut host, &mut goal, &ctx, p),
            "BUILD" => more::build(&mut host, &mut goal, &ctx, p),
            "UPGRADE" => more::upgrade(&mut host, &mut goal, &ctx, p),
            "DESTROY" => more::destroy(&mut host, &mut goal, &ctx, &mut statics, p),
            "UNPACK" => more::unpack(&mut host, &mut goal, &ctx),
            other => panic!("class {other}"),
        };
        let nan = f32::from_bits(u(&o["priority"])).is_nan();
        if nan && !o["fault"].as_bool().unwrap() {
            // Native continued with a NaN priority (EXPLORE on an empty
            // island: 0/0); Rust refuses nonfinite arithmetic at that point.
            assert!(result.is_err(), "case {i}: native NaN priority, Rust returned Ok");
            nans += 1;
            continue;
        }
        assert!(host.expected.is_empty(), "case {i}: unconsumed events {:?}", host.expected);
        if o["fault"].as_bool().unwrap() {
            faults += 1;
            assert!(result.is_err(), "case {i}: native faults, Rust returned Ok");
        } else {
            result.unwrap_or_else(|e| panic!("case {i}: {e}"));
        }
        assert_eq!(host.rng.state, u(&o["rng"]), "case {i}: SAI RNG");
        if c["class"] == "DESTROY" {
            let got = statics.as_ref().map(|s| json!([s.differences, s.economy]));
            assert_eq!(got.unwrap_or(Value::Null), o["statics"], "case {i}: statics");
        }
        assert_eq!(goal.priority, u(&o["priority"]), "case {i}: +38");
        let words: Vec<u32> = o["words"].as_array().unwrap().iter().map(u).collect();
        assert_eq!(goal.words, words, "case {i}: class words");
    }
    assert!(faults < cases.len() / 4, "{faults}");
    assert!(nans < 100, "{nans}");
}
