//! Replays research/phase8-vworld-update/native-vworld-update.json (original
//! and v14 instructions under Unicorn, PC24/53/64) against this module.
use super::*;
use serde_json::{Value, json};

fn pc(v: &Value) -> Precision {
    match v.as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("{other}"),
    }
}
fn w(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn fixture() -> Value {
    tests_fixture::load()
}

#[test]
fn native_view_time_43ab30() {
    let f = fixture();
    let rows = f["view_time_43ab30"].as_array().unwrap();
    assert_eq!(rows.len(), 432);
    for row in rows {
        let c = &row["input"];
        let mut words = [0xa5a5_a5a5, 0x5a5a_5a5a, w(&c["old8c"]), 0x3c3c_3c3c];
        view_time_43ab30(&mut words, w(&c["t"]), w(&c["e0"]), w(&c["e4"]), pc(&c["precision"])).unwrap();
        assert_eq!(json!(words), row["output"]["words"], "{row}");
        assert_eq!(row["output"]["trace"], json!([["76B120", 0]]));
    }
}

struct Sim<'a> {
    c: &'a Value,
    current: u32,
    trace: Vec<Value>,
}
impl SimTime for Sim<'_> {
    fn session_f4(&mut self) -> Result<bool> {
        Ok(w(&self.c["session_f4"]) != 0)
    }
    fn flags_b396c4(&mut self) -> Result<u32> {
        Ok(w(&self.c["b396c4"]))
    }
    fn single_player_auto_pause(&mut self) -> Result<bool> {
        Ok(w(&self.c["auto_pause"]) != 0)
    }
    fn paused_28(&mut self) -> Result<u8> {
        Ok(w(&self.c["paused"]) as u8)
    }
    fn frozen_29(&mut self) -> Result<u8> {
        Ok(w(&self.c["frozen"]) as u8)
    }
    fn render_dump_avi(&mut self) -> Result<bool> {
        Ok(w(&self.c["avi"]) != 0)
    }
    fn avi_time_76b080(&mut self) -> Result<Binary> {
        self.trace.push(json!(["76B080"]));
        b(w(&self.c["avi_time"]))
    }
    fn time_words(&mut self) -> Result<[u32; 4]> {
        Ok(std::array::from_fn(|i| w(&self.c["times"][i])))
    }
    fn admin_clock_74b180(&mut self, argument: u32) -> Result<Binary> {
        self.trace.push(json!(["74B180", argument]));
        b(w(&self.c["admin"]))
    }
    fn current_34(&mut self) -> Result<u32> {
        Ok(self.current)
    }
    fn store_current_34(&mut self, bits: u32) -> Result<()> {
        self.current = bits;
        Ok(())
    }
    fn world_e0(&mut self) -> Result<u32> {
        Ok(w(&self.c["e0"]))
    }
    fn world_e4(&mut self) -> Result<u32> {
        Ok(w(&self.c["e4"]))
    }
}

#[test]
fn native_sim_render_time_76b120() {
    let f = fixture();
    let rows = f["sim_time_76b120"].as_array().unwrap();
    assert_eq!(rows.len(), 1842);
    for row in rows {
        let c = &row["input"];
        let mut sim = Sim { c, current: w(&c["current"]), trace: Vec::new() };
        let result = sim_time_76b120(&mut sim, w(&c["argument"]), pc(&c["precision"])).unwrap();
        let o = &row["output"];
        assert_eq!(result, w(&o["result"]), "{row}");
        assert_eq!(sim.current, w(&o["current"]), "{row}");
        assert_eq!(json!(sim.trace), o["trace"], "{row}");
    }
}

/// One test VWorld: trace entries named as the oracle's seams.
struct Lab<'a> {
    c: &'a Value,
    fields: Fields<String>,
    trace: Vec<Value>,
    views: std::collections::BTreeMap<String, Value>,
    rng: Vec<u32>,
    minimap: &'static str,
    requests: bool,
    commands: i32,
}
impl<'a> Lab<'a> {
    fn new(c: &'a Value) -> Self {
        Self { c, fields: Fields::from_native_constructor(), trace: Vec::new(), views: Default::default(),
            rng: Vec::new(), minimap: "minimap", requests: false, commands: 0 }
    }
    fn push(&mut self, v: Value) -> Result<()> {
        self.trace.push(v);
        Ok(())
    }
}
impl Services for Lab<'_> {
    type View = String;
    fn fields(&mut self) -> Result<&mut Fields<String>> {
        Ok(&mut self.fields)
    }
    fn widget_base_update_5eec70(&mut self) -> Result<()> {
        self.push(json!(["5EEC70", "vworld"]))
    }
    fn vworld_ad(&mut self) -> Result<bool> {
        Ok(w(&self.c["ad"]) != 0)
    }
    fn construct_minimap_4341a0(&mut self) -> Result<()> {
        self.minimap = "new140";
        self.push(json!(["6F7810", MINIMAP_140_BYTES, 0]))?;
        self.push(json!(["4341A0", "new140"]))
    }
    fn look_at_timer_construct_59aa70(&mut self) -> Result<()> {
        self.push(json!(["59AA70", "0xb39f68"]))
    }
    fn look_at_timer_restart_59ab60(&mut self) -> Result<()> {
        self.push(json!(["59AB60", "0xb39f68"]))
    }
    fn look_at_timer_exit_96cab0(&mut self) -> Result<()> {
        self.push(json!(["96CAB0", "0x9ece40"]))
    }
    fn look_at_timer_milliseconds_59aba0(&mut self) -> Result<u32> {
        self.push(json!(["59ABA0", "0xb39f68"]))?;
        Ok(w(&self.c["milliseconds"]))
    }
    fn random_b39714_85d840(&mut self) -> Result<u32> {
        let v = self.rng.remove(0);
        self.push(json!(["85D840", v]))?;
        Ok(v)
    }
    fn map_extent_30(&mut self) -> Result<[u32; 2]> {
        Ok([w(&self.c["extent"][0]), w(&self.c["extent"][1])])
    }
    fn camera_look_at_6ff130(&mut self, x: u32, y: u32) -> Result<()> {
        self.push(json!(["6FF130", "camera", x, y]))
    }
    fn camera_update_701590(&mut self) -> Result<()> {
        self.push(json!(["701590", "camera"]))
    }
    fn visual_map_b36518(&mut self) -> Result<bool> {
        Ok(w(&self.c["visual_map"]) != 0)
    }
    fn uninitializing_ae(&mut self) -> Result<bool> {
        Ok(w(&self.c["uninit_ae"]) != 0)
    }
    fn deferred_flags_54(&mut self, view: &String) -> Result<u32> {
        Ok(w(&self.views[view]["flags"]))
    }
    fn deferred_presenting_bc(&mut self, view: &String) -> Result<bool> {
        self.push(json!(["vtBC", view]))?;
        Ok(w(&self.views[view]["vbc"]) != 0)
    }
    fn deferred_count_19c(&mut self, view: &String) -> Result<i32> {
        Ok(w(&self.views[view]["count"]) as i32)
    }
    fn destroy_view_43cf00(&mut self, view: String) -> Result<()> {
        self.push(json!(["43CF00", view]))
    }
    fn cursor_world_point_43c4b0(&mut self, normal: &mut [u32; 3]) -> Result<[u32; 3]> {
        *normal = [0x11, 0x22, 0x33];
        self.push(json!(["43C4B0", "vworld"]))?;
        Ok(std::array::from_fn(|i| w(&self.c["point"][i])))
    }
    fn ui_world_state_43a1c0(&mut self) -> Result<()> {
        self.push(json!(["43A1C0", "13c"]))
    }
    fn terrain_map_71fac0(&mut self) -> Result<()> {
        let visual = if w(&self.c["visual_map"]) != 0 { "visual" } else { "0x0" };
        self.push(json!(["71FAC0", visual]))
    }
    fn minimap_4361b0(&mut self) -> Result<()> {
        let minimap = self.minimap;
        self.push(json!(["4361B0", minimap]))
    }
    fn world_valid(&mut self) -> Result<WorldValid> {
        if self.c.get("b3cc94").is_none() {
            //43FF60 rows: the oracle seams 43EC60 itself; its first reached
            //read marks the call position.
            self.push(json!(["43EC60", "vworld"]))?;
            return Ok(WorldValid { revision_matches_b8: true, initialized_ad: true, world_d8: 0 });
        }
        Ok(WorldValid { revision_matches_b8: self.c["vworld_b8"] == self.c["b3cc94"], initialized_ad: w(&self.c["ad"]) != 0,
            world_d8: w(&self.c["world_d8"]) })
    }
    fn assert_855d30(&mut self, expression: &str, file: &str, line: u32) -> Result<u8> {
        self.push(json!(["855D30", expression, file, line]))?;
        Ok(w(&self.c["assert_result"]) as u8)
    }
    fn world_messages_50c750(&mut self) -> Result<()> {
        if self.c.get("b3cc94").is_none() { return Ok(()); }
        self.push(json!(["50C750", "manager"]))
    }
    fn scene_light_43a400(&mut self) -> Result<()> {
        if self.c.get("b3cc94").is_none() { return Ok(()); }
        self.push(json!(["43A400", "108"]))
    }
    fn heightfield_rects_4318e0(&mut self) -> Result<()> {
        if self.c.get("b3cc94").is_none() { return Ok(()); }
        self.push(json!(["4318E0", "bc"]))
    }
    fn terrain_visibility_4312f0(&mut self) -> Result<()> {
        if self.c.get("b3cc94").is_none() { return Ok(()); }
        self.push(json!(["4312F0", "94"]))
    }
    fn terrain_map_725340(&mut self) -> Result<()> {
        if self.c.get("b3cc94").is_none() { return Ok(()); }
        self.push(json!(["725340", "visual"]))
    }
    fn view_requests_208(&mut self) -> Result<()> {
        if self.requests { self.push(json!("208")) } else { Ok(()) }
    }
    fn fx_command_count_e8(&mut self) -> Result<i32> {
        Ok(self.commands)
    }
    fn fx_commands_e0(&mut self, count: i32) -> Result<()> {
        assert_eq!(count, self.commands);
        self.push(json!("E0"))
    }
}

#[test]
fn native_deferred_view_prune_43da00() {
    let f = fixture();
    let rows = f["deferred_43da00"].as_array().unwrap();
    assert_eq!(rows.len(), 42);
    for row in rows {
        let c = &row["input"];
        let mut lab = Lab::new(c);
        let views = c["views"].as_array().unwrap();
        for (i, v) in views.iter().enumerate().rev() {
            lab.views.insert(format!("v{i}"), v.clone());
            lab.fields.defer_destroy_43d190(format!("v{i}")).unwrap();
        }
        prune_43da00(&mut lab).unwrap();
        //Native frees each node right after its 43CF00; the Rust list removes it.
        let native: Vec<Value> = row["output"]["trace"].as_array().unwrap().iter().filter(|e| e[0] != "6F7430").cloned().collect();
        let frees: Vec<Value> = row["output"]["trace"].as_array().unwrap().iter().filter(|e| e[0] == "6F7430")
            .map(|e| json!(format!("v{}", &e[1].as_str().unwrap()[1..]))).collect();
        let destroyed: Vec<Value> = native.iter().filter(|e| e[0] == "43CF00").map(|e| e[1].clone()).collect();
        assert_eq!(frees, destroyed);
        assert_eq!(json!(lab.trace), json!(native), "{row}");
        assert_eq!(json!(lab.fields.deferred_98()), row["output"]["remaining"], "{row}");
    }
}

#[test]
fn native_vworld_update_43ff60() {
    let f = fixture();
    let rows = f["update_43ff60"].as_array().unwrap();
    assert_eq!(rows.len(), 512);
    let mut replayed = 0;
    for row in rows {
        let c = &row["input"];
        //Allocation null stores +140 = 0 and faults in 4361B0: native only.
        if w(&c["alloc_null"]) != 0 {
            continue;
        }
        let mut lab = Lab::new(c);
        lab.fields.minimap_140 = w(&c["minimap"]) != 0;
        lab.fields.look_at_174 = w(&c["look_at_174"]);
        lab.fields.timer_guard_b39f70 = w(&c["guard_b39f70"]);
        lab.fields.normal_178 = [0x0178_1781, 0x0178_1782, 0x0178_1783];
        lab.fields.cursor_184 = [0x0184_1841, 0x0184_1842, 0x0184_1843];
        lab.rng = c["rng"].as_array().unwrap().iter().map(w).collect();
        let mut refusals = Refusals::strict();
        update_43ff60(&mut lab, &mut refusals, pc(&c["precision"])).unwrap();
        let o = &row["output"];
        //43DA00 is seamed natively; the Rust prune over the empty +98 list
        //calls nothing (its kernel is replayed by deferred_43da00).
        let native: Vec<Value> = o["trace"].as_array().unwrap().iter().filter(|e| e[0] != "43DA00").cloned().collect();
        assert_eq!(json!(lab.trace), json!(native), "{row}");
        let expected_140 = if o["field_140"] == 0 { json!(false) } else { json!(true) };
        assert_eq!(json!(lab.fields.minimap_140), expected_140);
        assert_eq!(lab.fields.timer_guard_b39f70, w(&o["guard_b39f70"]));
        assert_eq!(json!(lab.fields.normal_178), o["normal_178"], "{row}");
        assert_eq!(json!(lab.fields.cursor_184), o["cursor_184"], "{row}");
        assert_eq!(lab.rng.len() as u64, o["rng_left"].as_u64().unwrap());
        replayed += 1;
    }
    assert_eq!(replayed, 511);
}

#[test]
fn native_world_view_43ec60() {
    let f = fixture();
    let rows = f["world_view_43ec60"].as_array().unwrap();
    assert_eq!(rows.len(), 42);
    for row in rows {
        let c = &row["input"];
        let mut lab = Lab::new(c);
        lab.fields.assert_latch_b39f64 = w(&c["latch"]) as u8;
        lab.requests = !c["requests"].as_array().unwrap().is_empty();
        lab.commands = c["commands"].as_array().unwrap().len() as i32;
        let mut refusals = Refusals::strict();
        world_view_43ec60(&mut lab, &mut refusals).unwrap();
        //The +208 segment (vt+74, 45B720, 50C510 free) and the FX loop
        //(4289A0, 429050, 45C7A0, free) are the Services' own steps.
        let mut native = Vec::new();
        for e in row["output"]["trace"].as_array().unwrap() {
            let name = e[0].as_str().unwrap();
            let marker = match name {
                "vt74" | "45B720" => Some("208"),
                "6F7430" if e[1].as_str().unwrap().starts_with('q') => Some("208"),
                "4289A0" | "429050" | "45C7A0" | "6F7430" => Some("E0"),
                _ => None,
            };
            match marker {
                Some(m) => {
                    if native.last() != Some(&json!(m)) {
                        native.push(json!(m));
                    }
                }
                None => native.push(e.clone()),
            }
        }
        assert_eq!(json!(lab.trace), json!(native), "{row}");
        assert_eq!(lab.fields.assert_latch_b39f64 as u64, row["output"]["latch"].as_u64().unwrap(), "{row}");
    }
}

/// The look-ahead policy records each refusal and continues natively; the
/// product stops at the first one with its native step name.
#[test]
fn refusal_policies() {
    struct Refusing(Fields<u32>, Vec<&'static str>);
    impl Services for Refusing {
        type View = u32;
        fn fields(&mut self) -> Result<&mut Fields<u32>> { Ok(&mut self.0) }
        fn widget_base_update_5eec70(&mut self) -> Result<()> { self.1.push("5EEC70"); Err("no widget owner".into()) }
        fn vworld_ad(&mut self) -> Result<bool> { Ok(true) }
        fn construct_minimap_4341a0(&mut self) -> Result<()> { self.1.push("4341A0"); Err("no minimap owner".into()) }
        fn look_at_timer_construct_59aa70(&mut self) -> Result<()> { unreachable!() }
        fn look_at_timer_restart_59ab60(&mut self) -> Result<()> { unreachable!() }
        fn look_at_timer_exit_96cab0(&mut self) -> Result<()> { unreachable!() }
        fn look_at_timer_milliseconds_59aba0(&mut self) -> Result<u32> { unreachable!() }
        fn random_b39714_85d840(&mut self) -> Result<u32> { unreachable!() }
        fn map_extent_30(&mut self) -> Result<[u32; 2]> { unreachable!() }
        fn camera_look_at_6ff130(&mut self, _: u32, _: u32) -> Result<()> { unreachable!() }
        fn camera_update_701590(&mut self) -> Result<()> { self.1.push("701590"); Err("no camera entry".into()) }
        fn visual_map_b36518(&mut self) -> Result<bool> { Ok(false) }
        fn uninitializing_ae(&mut self) -> Result<bool> { Ok(false) }
        fn deferred_flags_54(&mut self, _: &u32) -> Result<u32> { unreachable!() }
        fn deferred_presenting_bc(&mut self, _: &u32) -> Result<bool> { unreachable!() }
        fn deferred_count_19c(&mut self, _: &u32) -> Result<i32> { unreachable!() }
        fn destroy_view_43cf00(&mut self, _: u32) -> Result<()> { unreachable!() }
        fn cursor_world_point_43c4b0(&mut self, _: &mut [u32; 3]) -> Result<[u32; 3]> { unreachable!() }
        fn ui_world_state_43a1c0(&mut self) -> Result<()> { self.1.push("43A1C0"); Ok(()) }
        fn terrain_map_71fac0(&mut self) -> Result<()> { self.1.push("71FAC0"); Ok(()) }
        fn minimap_4361b0(&mut self) -> Result<()> { self.1.push("4361B0"); Err("no minimap owner".into()) }
        fn world_valid(&mut self) -> Result<WorldValid> { Ok(WorldValid { revision_matches_b8: true, initialized_ad: true, world_d8: 3 }) }
        fn assert_855d30(&mut self, _: &str, _: &str, _: u32) -> Result<u8> { unreachable!() }
        fn world_messages_50c750(&mut self) -> Result<()> { self.1.push("50C750"); Ok(()) }
        fn scene_light_43a400(&mut self) -> Result<()> { self.1.push("43A400"); Err("no light owner".into()) }
        fn heightfield_rects_4318e0(&mut self) -> Result<()> { self.1.push("4318E0"); Ok(()) }
        fn terrain_visibility_4312f0(&mut self) -> Result<()> { self.1.push("4312F0"); Ok(()) }
        fn terrain_map_725340(&mut self) -> Result<()> { self.1.push("725340"); Ok(()) }
        fn view_requests_208(&mut self) -> Result<()> { self.1.push("208"); Ok(()) }
        fn fx_command_count_e8(&mut self) -> Result<i32> { Ok(0) }
        fn fx_commands_e0(&mut self, _: i32) -> Result<()> { unreachable!() }
    }
    let mut s = Refusing(Fields::from_native_constructor(), Vec::new());
    let error = update_43ff60(&mut s, &mut Refusals::strict(), Precision::Double53).unwrap_err();
    assert_eq!(error, "5EEC70 VWorld widget base update: no widget owner");
    assert_eq!(s.1, ["5EEC70"]);
    let mut s = Refusing(Fields::from_native_constructor(), Vec::new());
    let mut refusals = Refusals::look_ahead();
    update_43ff60(&mut s, &mut refusals, Precision::Double53).unwrap();
    assert_eq!(s.1, ["5EEC70", "4341A0", "701590", "50C750", "43A400", "4318E0", "4312F0", "725340", "208", "43A1C0", "71FAC0", "4361B0"]);
    let steps: Vec<_> = refusals.list().iter().map(|r| r.0).collect();
    assert_eq!(steps, ["5EEC70 VWorld widget base update", "43FF98 6F7810+4341A0 (+140)", "4400E7 701590 camera update",
        "43ECF4 43A400 VWorld+108 light", "44016F 4361B0 +140"]);
    //A refused construction leaves +140 null; the next frame retries it.
    assert!(!s.0.minimap_140());
}
