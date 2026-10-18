//! Replays research/phase8-movement/native-company-think.json: original x86
//! regiment move Think 5276B0 with 51EA20, 526940, 51E9B0/51D840/51D7F0,
//! 52A1F0, 525740, 523130/523240/5233F0, 52CBD0, 52D230 and 409230 under
//! PC24/53/64. Every recorded service call, its argument bits and the paths
//! sent to members must match.
use aa_navigation::company_move::{
    self, CompanyMoveState, CompanyThinkWorld, CompanyWorld, Extents, FormationKind, Member, SolveRequest, Target,
    ThinkSettings,
};
use aa_navigation::{Error, OwnedPath, PathNode, Result};
use aa_numeric::Precision;
use serde_json::{Value, json};

fn u(v: &Value) -> u32 {
    v.as_u64().expect("u32") as u32
}
fn pair(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}
fn nodes(v: &Value) -> Vec<PathNode> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|n| PathNode { words: n.as_array().unwrap().iter().map(u).collect::<Vec<_>>().try_into().unwrap() })
        .collect()
}

struct World {
    case: Value,
    responses: Vec<Value>,
    events: Vec<Value>,
    slots: Vec<[u32; 2]>,
    pending: Vec<u32>,
}
impl World {
    fn record(&mut self, name: &str, bits: &[u32]) {
        self.events.push(json!({"name": name, "bits": bits}));
    }
    fn response(&mut self, name: &str) -> Value {
        assert!(!self.responses.is_empty(), "missing response {name}");
        let r = self.responses.remove(0);
        assert_eq!(r["name"], name);
        r
    }
    fn flag(&mut self, name: &str) -> bool {
        self.response(name)["value"].as_bool().unwrap()
    }
    fn organization(&self) -> &Value {
        &self.case["organization"]
    }
    fn member(&self, index: usize) -> Option<Member> {
        let m = &self.organization()["members"][index];
        if m.is_null() {
            return None;
        }
        Some(Member {
            index,
            is_leader: self.organization()["leader"].as_u64() == Some(index as u64),
            attached: u(&m["attached"]) != 0,
            slot: u(&m["slot"]) as i32,
            position_bits: pair(&m["position"]),
            in_this_organization: m["in_org"].as_bool().unwrap(),
        })
    }
    fn group_valid(&self) -> bool {
        self.case["actor"]["group_valid"].as_bool().unwrap()
    }
    fn path_event(&mut self, name: &str, member: Member, path: &OwnedPath, speed: u32, aux: [u32; 2]) {
        let mut bits = vec![member.index as u32, speed, aux[0], aux[1]];
        for n in &path.nodes {
            bits.extend_from_slice(&n.words);
        }
        bits.push(path.kind1_count as u32);
        bits.push(path.kind2_count as u32);
        self.record(name, &bits);
    }
}
impl CompanyWorld for World {
    fn can_activate(&mut self, _: u32, _: &mut CompanyMoveState) -> Result<bool> {
        unreachable!()
    }
    fn activate(&mut self, _: u32, _: &mut CompanyMoveState) -> Result<bool> {
        unreachable!()
    }
    fn group_offset(&mut self, _: &mut CompanyMoveState) -> Result<Option<[u32; 2]>> {
        if !self.group_valid() {
            return Ok(None);
        }
        self.record("GroupActive", &[]);
        if !self.flag("GroupActive") {
            return Ok(None);
        }
        self.record("GroupOffset", &[]);
        Ok(Some(pair(&self.response("GroupOffset")["offset"])))
    }
    fn actor_position(&mut self, _: u32) -> Result<Option<[u32; 2]>> {
        unreachable!()
    }
    fn set_mind_auxiliary(&mut self, _: [u32; 2]) -> Result<()> {
        unreachable!()
    }
    fn organization_speed_cap(&mut self) -> Result<u32> {
        unreachable!()
    }
    fn member_slots(&mut self) -> Result<Vec<usize>> {
        unreachable!()
    }
    fn member_accepts_speed(&mut self, _: usize) -> Result<bool> {
        unreachable!()
    }
    fn member_set_speed(&mut self, _: usize, _: u32) -> Result<()> {
        unreachable!()
    }
    fn world_time(&mut self) -> Result<u32> {
        Ok(u(&self.case["world_time"]))
    }
    fn schedule(&mut self, delay_bits: u32) -> Result<()> {
        self.record("Schedule", &[delay_bits, 0]);
        Ok(())
    }
    fn move_failed(&mut self) -> Result<()> {
        self.record("MoveFailed", &[]);
        Ok(())
    }
}
impl CompanyThinkWorld for World {
    fn think_base(&mut self, _: &mut CompanyMoveState) -> Result<()> {
        self.record("ThinkBase", &[]);
        Ok(())
    }
    fn is_current(&mut self, _: &mut CompanyMoveState) -> Result<bool> {
        self.record("IsCurrent", &[]);
        Ok(self.flag("IsCurrent"))
    }
    fn mind_virtual_b8(&mut self) -> Result<()> {
        self.record("MindB8", &[]);
        Ok(())
    }
    fn target(&mut self, id: u32) -> Result<Option<Target>> {
        if id == 0x10001 {
            return Ok(Some(Target { position_bits: pair(&self.case["actor"]["position"]), is_self: true }));
        }
        let t = &self.case["target"];
        if t.is_null() || !t["valid"].as_bool().unwrap() || u(&t["id"]) != id {
            return Ok(None);
        }
        Ok(Some(Target { position_bits: pair(&t["position"]), is_self: false }))
    }
    fn target_lost(&mut self, id: u32) -> Result<()> {
        self.record("TargetLost", &[id]);
        Ok(())
    }
    fn finished(&mut self) -> Result<()> {
        self.record("Finished", &[]);
        Ok(())
    }
    fn mind_virtual_f4(&mut self, id: u32) -> Result<()> {
        self.record("MindF4", &[id]);
        Ok(())
    }
    fn group_formation_active(&mut self) -> Result<bool> {
        if !self.group_valid() {
            return Ok(false);
        }
        self.record("GroupActive", &[]);
        Ok(self.flag("GroupActive"))
    }
    fn target_in_range(&mut self, id: u32) -> Result<bool> {
        self.record("TargetInRange", &[id]);
        Ok(self.flag("TargetInRange"))
    }
    fn engage_target(&mut self, _: u32) -> Result<bool> {
        self.record("TargetV108", &[]);
        if u(&self.case["target"]["word_d0"]) == 0 {
            return Ok(false);
        }
        self.record("ActorV138", &[0x7777]);
        Ok(self.flag("ActorV138"))
    }
    fn own_position(&mut self) -> Result<[u32; 2]> {
        Ok(pair(&self.case["actor"]["position"]))
    }
    fn path_region_ok(&mut self, _: &mut CompanyMoveState, position: [u32; 2]) -> Result<bool> {
        self.record("PathRegionOk", &position);
        Ok(self.flag("PathRegionOk"))
    }
    fn leader_moving(&mut self) -> Result<Option<bool>> {
        if self.organization()["leader"].is_null() {
            return Ok(None);
        }
        self.record("LeaderMoving", &[]);
        Ok(Some(self.flag("LeaderMoving")))
    }
    fn formation_word(&mut self) -> Result<u32> {
        Ok(u32::from(!self.organization()["formation"].is_null()))
    }
    fn pop_pending_member(&mut self) -> Result<Option<Option<Member>>> {
        if self.pending.is_empty() {
            return Ok(None);
        }
        let id = self.pending.remove(0);
        let index = id.wrapping_sub(0x20010) as usize;
        let member = (index < 16).then(|| self.member(index)).flatten();
        let alive = member.filter(|_| u(&self.organization()["members"][index]["flags"]) & 1 == 0);
        Ok(Some(alive))
    }
    fn group_handle_valid(&mut self) -> Result<bool> {
        self.record("GroupHandleValid", &[]);
        Ok(self.flag("GroupHandleValid"))
    }
    fn prepare_leader_query(&mut self) -> Result<()> {
        if self.organization()["leader"].is_null() {
            return Err(Error::World("0x554350 null leader".into()));
        }
        self.record("PrepareLeaderQuery", &[]);
        Ok(())
    }
    fn leader_position(&mut self) -> Result<[u32; 2]> {
        let index = self.organization()["leader"].as_u64().unwrap() as usize;
        Ok(self.member(index).unwrap().position_bits)
    }
    fn mind_virtual_98(&mut self) -> Result<u32> {
        self.record("MindV98", &[]);
        Ok(u(&self.response("MindV98")["value"]))
    }
    fn state_220_current(&mut self) -> Result<bool> {
        self.record("State220Current", &[]);
        Ok(self.flag("State220Current"))
    }
    fn owner_flag_76b9a0(&mut self) -> Result<bool> {
        self.record("OwnerV108", &[]);
        if u(&self.response("OwnerV108")["value"]) == 0 {
            return Ok(false);
        }
        self.record("OwnerV108", &[]);
        self.response("OwnerV108");
        self.record("Owner76b9a0", &[]);
        Ok(self.flag("Owner76b9a0"))
    }
    fn solve(&mut self, path: &mut OwnedPath, request: SolveRequest) -> Result<bool> {
        self.record("Solve", &[request.from_bits[0], request.from_bits[1], request.to_bits[0], request.to_bits[1],
            request.ignore_target.unwrap_or(0), request.query_word28, 0x2000_0000, request.mode]);
        let r = self.response("Solve");
        if r["value"].as_bool().unwrap() {
            path.nodes = nodes(&r["nodes"]);
            path.kind1_count = r["kind1_count"].as_i64().unwrap() as i32;
            path.kind2_count = r["kind2_count"].as_i64().unwrap() as i32;
        }
        Ok(r["value"].as_bool().unwrap())
    }
    fn clear_owner_state_16(&mut self) -> Result<()> {
        if u(&self.case["actor"]["property_flags"]) & 0x40_0000 != 0 {
            self.record("ActorState", &[0x16, 0]);
        }
        Ok(())
    }
    fn set_organization_facing(&mut self, facing_bits: [u32; 2]) -> Result<()> {
        self.record("SetOrganizationFacing", &facing_bits);
        Ok(())
    }
    fn compute_slot_positions(&mut self) -> Result<()> {
        self.record("ComputeSlots", &[]);
        let r = self.response("ComputeSlots");
        self.slots = r["slots"].as_array().unwrap().iter().map(pair).collect();
        Ok(())
    }
    fn formation_kind(&mut self) -> Result<FormationKind> {
        let f = &self.organization()["formation"];
        if f.is_null() {
            return Ok(FormationKind::None);
        }
        Ok(if u(&f[0]) != 0 {
            FormationKind::Oriented
        } else if u(&f[1]) != 0 {
            FormationKind::Scatter
        } else {
            FormationKind::Plain
        })
    }
    fn next_member(&mut self, after: Option<usize>) -> Result<Option<Member>> {
        let count = self.organization()["members"].as_array().unwrap().len();
        let start = after.map_or(0, |i| i + 1);
        Ok((start..count).find_map(|i| self.member(i)))
    }
    fn slot_offset(&mut self, slot: i32) -> Result<[u32; 2]> {
        Ok(self.slots[slot as usize])
    }
    fn own_facing(&mut self) -> Result<[u32; 2]> {
        Ok(pair(&self.case["actor"]["facing"]))
    }
    fn member_move_path(&mut self, member: Member, path: &OwnedPath, speed: u32, aux: [u32; 2]) -> Result<()> {
        self.path_event("MemberMovePath", member, path, speed, aux);
        Ok(())
    }
    fn member_formation_path(&mut self, member: Member, path: &OwnedPath, speed: u32, aux: [u32; 2]) -> Result<()> {
        self.path_event("MemberFormationPath", member, path, speed, aux);
        Ok(())
    }
    fn member_order(&mut self, member: Member, point: [u32; 2], speed: u32, aux: [u32; 2]) -> Result<()> {
        self.record("MemberOrder", &[point[0], point[1], speed, aux[0], aux[1]]);
        self.record("IssueOrder", &[member.index as u32]);
        Ok(())
    }
    fn place_near(&mut self, _: Member, point: &mut [u32; 2], direction: [u32; 2]) -> Result<()> {
        self.record("PlaceNear", &[point[0], point[1], direction[0], direction[1], 3, 0]);
        *point = pair(&self.response("PlaceNear")["point"]);
        Ok(())
    }
    fn next_random(&mut self) -> Result<u32> {
        let v = u(&self.response("Random")["value"]);
        self.record("Random", &[v]);
        Ok(v)
    }
}

#[test]
fn native_regiment_move_think_matches_original_x86() {
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-movement/native-company-think.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    let cases = doc["cases"].as_array().unwrap();
    let mut faults = 0;
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let context = format!("{name} {p:?}");
        let s = &case["state"];
        let has_formation = !case["organization"]["formation"].is_null();
        let mut state = CompanyMoveState {
            status: 2,
            target_id: u(&s["target_id"]),
            path_destination_bits: pair(&s["path_destination_bits"]),
            destination_bits: pair(&s["destination_bits"]),
            auxiliary_bits: pair(&s["auxiliary_bits"]),
            speed_bits: u(&s["speed_bits"]),
            tries: s["tries"].as_i64().unwrap() as i32,
            path: OwnedPath {
                nodes: nodes(&s["path"]["nodes"]),
                kind1_count: s["path"]["kind1_count"].as_i64().unwrap() as i32,
                kind2_count: s["path"]["kind2_count"].as_i64().unwrap() as i32,
                dirty: false,
                word18: 0,
            },
            formation_word: u32::from(u(&s["formation_word"]) == 1 && has_formation),
            start_time_bits: u(&s["start_time_bits"]),
        };
        let mut world = World {
            case: case.clone(),
            responses: case["responses"].as_array().unwrap().clone(),
            events: vec![],
            slots: case["organization"]["slots"].as_array().unwrap().iter().map(pair).collect(),
            pending: case["pending"].as_array().unwrap().iter().map(u).collect(),
        };
        let settings = ThinkSettings {
            group_speed_minimum_bits: u(&case["group_speed_minimum"]),
            min_seconds_before_fail_bits: u(&case["min_seconds"]),
            extents: Extents { extent_bits: [256f32.to_bits(), 192f32.to_bits()] },
        };
        let result = company_move::think(&mut state, settings, p, &mut world);
        let out = &case["output"];
        if out.get("null_dereference").is_some() {
            assert!(result.is_err(), "{context}: native null read must be an error");
            assert_eq!(Value::Array(world.events), out["events"], "{context} events before fault");
            faults += 1;
            continue;
        }
        result.unwrap();
        assert_eq!(Value::Array(world.events.clone()), out["events"], "{context} events");
        assert!(world.responses.is_empty(), "{context} unused responses");
        let e = &out["state"];
        assert_eq!(state.target_id, u(&e["target_id"]), "{context}");
        assert_eq!(state.path_destination_bits, pair(&e["path_destination_bits"]), "{context} goal");
        assert_eq!(state.destination_bits, pair(&e["destination_bits"]), "{context}");
        assert_eq!(state.speed_bits, u(&e["speed_bits"]), "{context}");
        assert_eq!(state.tries as i64, e["tries"].as_i64().unwrap(), "{context} tries");
        let path: Vec<Vec<u32>> = state.path.nodes.iter().map(|n| n.words.to_vec()).collect();
        assert_eq!(json!(path), e["path"]["nodes"], "{context} path");
        assert_eq!(state.path.kind1_count as i64, e["path"]["kind1_count"].as_i64().unwrap(), "{context}");
        assert_eq!(state.path.kind2_count as i64, e["path"]["kind2_count"].as_i64().unwrap(), "{context}");
        let current = state.formation_word == u32::from(has_formation);
        assert_eq!(current, e["formation_word_is_current"].as_bool().unwrap(), "{context} formation");
        assert_eq!(world.pending.is_empty(), !out["pending_left"].as_bool().unwrap(), "{context} pending");
    }
    assert!(faults > 0);
    assert_eq!(cases.len(), doc["case_count"].as_u64().unwrap() as usize);
}
