//! Replays research/phase8-restoration/native-spawn-member.json: original
//! 0x4FBF40 at PC24/PC53/PC64 with every callee seamed. Call sequence,
//! arguments (binary32 bits) and result must match.
use aa_actors::world_creation::{Result, SpawnRequest, SpawnWorld, spawn_member};
use aa_numeric::Precision;
use serde_json::{Value, json};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Obj {
    Org,
    Candidate(usize),
    Created(u32),
}
struct Recorder<'a> {
    case: &'a Value,
    calls: Vec<Value>,
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn name(o: Obj) -> Value {
    match o {
        Obj::Org => json!("org"),
        Obj::Candidate(i) => json!(i),
        Obj::Created(_) => unreachable!(),
    }
}
impl SpawnWorld<u32> for Recorder<'_> {
    type Object = Obj;
    type Owner = i64;
    fn game_mode(&self) -> Result<u32> {
        Ok(u(&self.case["game_mode"]))
    }
    fn actor_position(&self, _: Obj) -> Result<[u32; 2]> {
        Ok([u(&self.case["actor_xy"][0]), u(&self.case["actor_xy"][1])])
    }
    fn actor_facing(&self, _: Obj) -> Result<[u32; 2]> {
        Ok([u(&self.case["facing"][0]), u(&self.case["facing"][1])])
    }
    fn actor_gtid(&self, _: Obj) -> Result<u32> {
        Ok(0x30007)
    }
    fn organization_radius(&self, _: Obj) -> Result<u32> {
        Ok(u(&self.case["radius"]))
    }
    fn organization_position(&self, _: Obj) -> Result<[u32; 2]> {
        Ok([u(&self.case["org_xy"][0]), u(&self.case["org_xy"][1])])
    }
    fn slot_offset(&self, _: Obj, slot: i32) -> Result<[u32; 2]> {
        let o = &self.case["offsets"][slot as usize];
        Ok([u(&o[0]), u(&o[1])])
    }
    fn query(&mut self, position: [u32; 2], radius: u32) -> Result<Vec<Obj>> {
        self.calls.push(json!(["query", position[0], position[1], radius]));
        let n = self.case["candidates"].as_array().unwrap().len();
        Ok((0..n).map(Obj::Candidate).collect())
    }
    fn owner(&mut self, object: Obj) -> Result<Option<i64>> {
        self.calls.push(json!(["owner", name(object)]));
        Ok(match object {
            Obj::Org => Some(-1),
            Obj::Candidate(i) => self.case["candidates"][i]["has_owner"]
                .as_bool()
                .unwrap()
                .then_some(i as i64),
            Obj::Created(_) => None,
        })
    }
    fn related(&mut self, owner: i64, other: Option<i64>) -> Result<bool> {
        assert_eq!(other, Some(-1));
        self.calls.push(json!(["related", owner]));
        Ok(self.case["candidates"][owner as usize]["related"].as_bool().unwrap())
    }
    fn definition(&self, object: Obj) -> Result<u32> {
        let Obj::Candidate(i) = object else { panic!() };
        Ok(u(&self.case["candidates"][i]["definition"]))
    }
    fn recruit_structure(&self, definition: &u32) -> Result<bool> {
        Ok(self.case["recruit_flag_defs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| u(d) == *definition))
    }
    fn object_position(&self, object: Obj) -> Result<[u32; 2]> {
        let Obj::Candidate(i) = object else { panic!() };
        let xy = &self.case["candidates"][i]["xy"];
        Ok([u(&xy[0]), u(&xy[1])])
    }
    fn create_at_structure(&mut self, _: &SpawnRequest<u32>, params_40: u32, structure: Obj) -> Result<Option<Obj>> {
        self.calls.push(json!(["create_at_structure", name(structure), params_40]));
        Ok(self.case["structure_creation"].as_bool().unwrap().then_some(Obj::Created(0x6000)))
    }
    fn set_member(&mut self, _: Obj, slot: i32, member: Obj) -> Result<()> {
        let Obj::Created(v) = member else { panic!() };
        self.calls.push(json!(["set_member", slot, v]));
        Ok(())
    }
    fn slot_placement(&mut self, _: Obj, slot: i32, position: [u32; 2], facing: [u32; 2]) -> Result<bool> {
        self.calls.push(json!(["slot_placement", slot, position[0], position[1], facing[0], facing[1]]));
        Ok(self.case["placement_ok"].as_bool().unwrap())
    }
    fn slot_placement_at(&mut self,actor:Obj,slot:i32,position:&mut [u32;2],facing:[u32;2])->Result<bool>{
        let result=self.slot_placement(actor,slot,*position,facing)?;
        if let Some(value)=self.case.get("mutated_xy"){*position=[u(&value[0]),u(&value[1])];}
        Ok(result)
    }
    fn create_member(
        &mut self,
        _: Obj,
        slot: i32,
        position: [u32; 3],
        facing: [u32; 2],
        byte_50: u8,
        byte_51: u8,
    ) -> Result<Option<Obj>> {
        self.calls.push(json!([
            "create_member", slot, position[0], position[1], position[2], facing[0], facing[1], byte_50, byte_51
        ]));
        Ok(self.case["member_creation"].as_bool().unwrap().then_some(Obj::Created(0x7000)))
    }
}

#[test]
fn original_member_placement_mutates_xy_and_preserves_captured_facing(){
    let doc:Value=serde_json::from_str(include_str!("../../../research/development10/organization-spawn/native-member-slot-writeback.json")).unwrap();
    assert_eq!(doc["rows"].as_array().unwrap().len(),108);
    for row in doc["rows"].as_array().unwrap(){let input=&row["input"];
        let pc=serde_json::from_value(input["precision"].clone()).unwrap();
        let request=SpawnRequest{structure_definitions:vec![],word_28:0,byte_50:u(&input["byte_50"])as u8,byte_51:u(&input["byte_51"])as u8};
        let mut world=Recorder{case:input,calls:vec![]};
        let result=spawn_member(&request,Obj::Org,input["slot"].as_i64().unwrap()as i32,&mut world,pc).unwrap();
        let result=match result{Some(Obj::Created(v))=>v,None=>0,_=>unreachable!()};
        assert_eq!(json!(result),row["native"]["result"],"{row}");assert_eq!(json!(world.calls),row["native"]["calls"],"{row}");
    }
}

#[test]
fn spawn_member_matches_native_calls_and_x87_values() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-restoration/native-spawn-member.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let (mut matched, mut refused) = (0, 0);
    for case in doc["cases"].as_array().unwrap() {
        let input = &case["input"];
        let native = &case["native"];
        let pc = match input["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let request = SpawnRequest {
            structure_definitions: input["accepted_defs"].as_array().unwrap().iter().map(u).collect(),
            word_28: u(&input["params_28"]),
            byte_50: u(&input["byte_50"]) as u8,
            byte_51: u(&input["byte_51"]) as u8,
        };
        let mut world = Recorder {
            case: input,
            calls: Vec::new(),
        };
        let slot = input["slot"].as_i64().unwrap() as i32;
        match spawn_member(&request, Obj::Org, slot, &mut world, pc) {
            Ok(result) => {
                let value = match result {
                    Some(Obj::Created(v)) => v,
                    None => 0,
                    other => panic!("{other:?}"),
                };
                assert_eq!(value, u(&native["result"]), "{input}");
                assert_eq!(json!(world.calls), native["calls"], "{input}");
                matched += 1;
            }
            Err(e) => {
                // Only non-finite x87 operands (NaN/infinite inputs or
                // intermediates) are outside the Binary domain.
                assert!(e.to_string().contains("non-finite"), "{input}: {e}");
                refused += 1;
            }
        }
    }
    println!("matched {matched} refused {refused}");
    assert!(matched >= 200, "{matched}");
}
