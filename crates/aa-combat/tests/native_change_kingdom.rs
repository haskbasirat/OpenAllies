//! Replays research/phase8-combat/native-change-kingdom-fixtures.json:
//! original 4D8450 KKC_GActor::ChangeKingdom with recorded seams.
use aa_combat::{
    Result,
    ownership::{self, World},
};
use serde_json::{Value, json};

const ACTOR: u32 = 1;
const OUTER: u32 = 2;
const ORG: u32 = 3;
const MINDOBJ: u32 = 4;
const T15C: u32 = 5;

fn ktag(k: Option<u32>) -> Value {
    match k {
        Some(k) => json!(format!("k{}", k - 100)),
        None => json!(0),
    }
}

struct Host {
    c: Value,
    kingdom: Option<u32>,
    events: Vec<Value>,
}
impl Host {
    fn a(&self) -> &Value {
        &self.c["actor"]
    }
    fn ev(&mut self, name: &str, v: Value) {
        self.events.push(json!({ name: v }));
    }
    fn s(&mut self, name: &str) {
        self.events.push(json!(name));
    }
    fn present(&self, key: &str) -> bool {
        self.a()[key] == true
    }
}
impl World for Host {
    fn ownable(&self, _: u32) -> Result<bool> {
        Ok(self.present("ownable"))
    }
    fn not_ownable(&mut self, _: u32) -> Result<()> {
        self.s("NotOwnable");
        Ok(())
    }
    fn kingdom(&self, a: u32) -> Result<Option<u32>> {
        assert_eq!(a, ACTOR);
        Ok(self.kingdom)
    }
    fn set_kingdom(&mut self, _: u32, k: Option<u32>) -> Result<()> {
        self.kingdom = k;
        Ok(())
    }
    fn flags100(&self, _: u32) -> Result<u32> {
        Ok(self.a()["flags100"].as_u64().unwrap() as u32)
    }
    fn has_wall_link(&self, _: u32) -> Result<bool> {
        Ok(self.present("wall_link"))
    }
    fn change_log(&mut self, _: u32, _: Option<u32>) -> Result<()> {
        self.s("ChangeLog");
        Ok(())
    }
    fn sai_active(&self) -> Result<bool> {
        Ok(self.c["sai"] == true)
    }
    fn sai_change(&mut self, _: u32, old: Option<u32>, new: Option<u32>) -> Result<()> {
        self.ev("SaiChange", json!(["actor", 0x1400, ktag(old), ktag(new)]));
        Ok(())
    }
    fn kingdom_count(&self) -> Result<i32> {
        Ok(self.c["kingdoms"].as_array().unwrap().len() as i32)
    }
    fn kingdom_at(&self, i: i32) -> Result<Option<u32>> {
        Ok((!self.c["kingdoms"][i as usize].is_null()).then_some(100 + i as u32))
    }
    fn visible_to(&mut self, _: u32, k: u32) -> Result<bool> {
        let tag = ktag(Some(k));
        self.ev("Visible", json!([tag.clone(), 1]));
        Ok(self.a()["visible"].as_array().unwrap().contains(&tag))
    }
    fn own_index(&self, k: u32) -> Result<Option<u32>> {
        let v = &self.c["kingdoms"][(k - 100) as usize];
        Ok((v["own_visible"] != true).then(|| v["index"].as_u64().unwrap() as u32))
    }
    fn snapshot_begin(&mut self, _: u32) -> Result<()> {
        self.ev("SnapshotBegin", json!([0, 0, 9]));
        Ok(())
    }
    fn snapshot_end(&mut self, _: u32) -> Result<()> {
        self.ev("SnapshotEnd", Value::Null);
        Ok(())
    }
    fn snapshot_release(&mut self, _: u32) -> Result<()> {
        if self.c["snapshot_list"] == true {
            self.ev("Free", json!("snapshot"));
        }
        Ok(())
    }
    fn has_component(&self, _: u32, offset: u32) -> Result<bool> {
        let key = match offset {
            0x94 => "comp94",
            0x160 => "obj160",
            0xd8 => "compd8",
            0x98 => "wall98",
            0x7c => "orgcomp7c",
            0x88 => "c88",
            0xd4 => "objd4",
            _ => unreachable!(),
        };
        Ok(self.present(key))
    }
    fn research_detach(&mut self, _: u32) -> Result<()> {
        self.ev("Research94Detach", json!("comp94"));
        Ok(())
    }
    fn construction_cancel(&mut self, _: u32) -> Result<()> {
        self.ev("ConstructionCancel", json!("comp9c"));
        Ok(())
    }
    fn upgrade_cancel(&mut self, _: u32) -> Result<()> {
        self.ev("UpgradeCancel", json!("comp9c"));
        Ok(())
    }
    fn object160(&mut self, _: u32) -> Result<()> {
        self.ev("Object160", json!(["obj160", 0x10001, 1]));
        Ok(())
    }
    fn object164_live(&self, _: u32) -> Result<bool> {
        Ok(self.present("obj164"))
    }
    fn object164(&mut self, _: u32) -> Result<()> {
        self.ev("Object164", json!(["obj164", "actor"]));
        Ok(())
    }
    fn find_single(&mut self, k: u32, _: u32) -> Result<Option<u32>> {
        let list = format!("singles:k{}", k - 100);
        let found = self.c["singles"][&list] == true;
        self.ev("FindSingle", json!([list, found]));
        Ok(found.then_some(77))
    }
    fn remove_single(&mut self, k: u32, _: u32) -> Result<()> {
        self.ev(
            "RemoveSingle",
            json!([format!("singles:k{}", k - 100), "single", 1]),
        );
        Ok(())
    }
    fn add_single(&mut self, k: u32, _: u32) -> Result<()> {
        self.ev(
            "AddSingle",
            json!([format!("singles:k{}", k - 100), "single"]),
        );
        Ok(())
    }
    fn free_single(&mut self, _: u32) -> Result<()> {
        self.ev("Free", json!("single"));
        Ok(())
    }
    fn duplicate_single(&mut self, _: u32, _: u32) -> Result<()> {
        self.s("DuplicateSingle");
        Ok(())
    }
    fn kingdom_remove_actor(&mut self, k: u32, _: u32) -> Result<()> {
        self.ev("KingdomRemoveActor", json!([ktag(Some(k)), "actor"]));
        Ok(())
    }
    fn kingdom_add_actor(&mut self, k: u32, _: u32) -> Result<()> {
        self.ev("KingdomAddActor", json!([ktag(Some(k)), "actor"]));
        Ok(())
    }
    fn component_d8(&mut self, _: u32, new: Option<u32>) -> Result<()> {
        self.ev("ComponentD8", ktag(new));
        Ok(())
    }
    fn membership_organization(&self, _: u32) -> Result<Option<Option<u32>>> {
        let m = &self.a()["membership"];
        Ok((!m.is_null()).then(|| (m == true).then_some(ORG)))
    }
    fn membership_release(&mut self, _: u32) -> Result<()> {
        self.ev("MembershipRelease", json!(["membership", 1]));
        Ok(())
    }
    fn request_removal(&mut self, _: u32) -> Result<()> {
        self.ev("RequestRemoval", json!(["actor", 0]));
        Ok(())
    }
    fn manager(&mut self, _: u32, had: bool) -> Result<()> {
        self.ev("Manager", json!([0x1300, "actor", had as u8]));
        Ok(())
    }
    fn outer(&mut self, _: u32) -> Result<Option<u32>> {
        self.s("Outer");
        Ok(self.present("outer").then_some(OUTER))
    }
    fn change_kingdom(&mut self, a: u32, new: Option<u32>) -> Result<()> {
        let name = match a {
            OUTER => "outer",
            ORG => "org",
            MINDOBJ => "mindobj",
            _ => unreachable!(),
        };
        self.ev("ChangeKingdom", json!([name, ktag(new)]));
        Ok(())
    }
    fn wall_kingdom(&mut self, _: u32, new: Option<u32>) -> Result<()> {
        self.ev("WallKingdom", ktag(new));
        Ok(())
    }
    fn organization_kingdom(&mut self, _: u32, new: Option<u32>) -> Result<()> {
        self.ev("OrganizationKingdom", ktag(new));
        Ok(())
    }
    fn mind_object(&self, _: u32) -> Result<Option<Option<u32>>> {
        let m = &self.a()["mind"];
        Ok((!m.is_null()).then(|| (m == true).then_some(MINDOBJ)))
    }
    fn denizen_kingdom(&mut self, _: u32, new: Option<u32>) -> Result<()> {
        self.ev("DenizenKingdom", ktag(new));
        Ok(())
    }
    fn object_d4_kingdom(&mut self, _: u32, new: Option<u32>) -> Result<()> {
        self.ev("ObjectD4Kingdom", ktag(new));
        Ok(())
    }
    fn research_kingdom(&mut self, _: u32, new: Option<u32>) -> Result<()> {
        self.ev("Research94Kingdom", ktag(new));
        Ok(())
    }
    fn notify(&mut self, _: u32) -> Result<()> {
        self.ev("Notify", json!("actor"));
        Ok(())
    }
    fn update_0ea0(&mut self, _: u32) -> Result<()> {
        self.ev("Update0EA0", Value::Null);
        Ok(())
    }
    fn virtual8c(&mut self, _: u32) -> Result<bool> {
        self.s("Virtual8C");
        Ok(self.present("reveal"))
    }
    fn virtual15c(&mut self, _: u32) -> Result<Option<u32>> {
        self.s("Virtual15C");
        Ok(self.present("t15c").then_some(T15C))
    }
    fn reveal(&mut self, _: u32, k: Option<u32>) -> Result<()> {
        self.ev("Reveal", json!([ktag(k), 1]));
        Ok(())
    }
    fn wall_reveal(&mut self, object: u32, k: Option<u32>) -> Result<()> {
        assert_eq!(object, T15C);
        self.ev("WallReveal", json!(["wall:t15c", ktag(k)]));
        Ok(())
    }
}

#[test]
fn original_change_kingdom_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-change-kingdom-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 25);
    for row in cases {
        let c = &row["input"];
        let mut host = Host {
            kingdom: c["actor"]["kingdom"].as_u64().map(|k| 100 + k as u32),
            c: c.clone(),
            events: vec![],
        };
        let new = c["new"].as_u64().map(|k| 100 + k as u32);
        ownership::change_kingdom(ACTOR, new, &mut host).unwrap();
        assert_eq!(json!(host.events), row["output"]["events"], "{}", c["name"]);
        assert_eq!(
            ktag(host.kingdom),
            row["output"]["kingdom"],
            "{}",
            c["name"]
        );
    }
}
