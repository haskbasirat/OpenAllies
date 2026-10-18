use aa_runtime::film_kingdom_parameters::{self as native, Fields, Result, Table};
use serde_json::{Value, json};
struct Input {
    slots: Vec<Value>,
    resources: usize,
    events: Vec<String>,
    stop: Option<usize>,
    globals: Vec<(Vec<u16>, Vec<u16>)>,
}
fn words(v: &Value) -> Vec<u16> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u16)
        .collect()
}
impl Input {
    fn new(v: &Value) -> Self {
        let slots = v["slots"].as_array().unwrap().clone();
        let mut globals = Vec::new();
        for r in slots.iter().filter(|r| !r.is_null()) {
            let key = words(&r["key"]);
            if !globals.iter().any(|(k, _)| *k == key) {
                globals.push((key, vec![111, 108, 100]));
            }
        }
        Self {
            slots,
            resources: v["resources"].as_u64().unwrap() as usize,
            events: Vec::new(),
            stop: v["stop"].as_u64().map(|n| n as usize),
            globals,
        }
    }
    fn event(&mut self, n: &str) -> Result<()> {
        if self.stop == Some(self.events.len()) {
            return Err(n.into());
        }
        self.events.push(n.into());
        Ok(())
    }
    fn read(&mut self, k: usize, n: &str) -> Result<Vec<u16>> {
        self.event(n)?;
        Ok(words(&self.slots[k][n]))
    }
    fn scalar(&mut self, k: usize, n: &str) -> Result<u32> {
        self.event(n)?;
        Ok(self.slots[k][n].as_u64().unwrap() as u32)
    }
    fn definition(&mut self, k: usize, n: &str) -> Result<Option<u32>> {
        self.event(n)?;
        Ok(self.slots[k][n].as_u64().map(|n| n as u32))
    }
    fn from(&self, from: usize) -> Option<usize> {
        (from..self.slots.len()).find(|&i| !self.slots[i].is_null())
    }
}
impl Fields for Input {
    type Kingdom = usize;
    type Team = usize;
    type Definition = u32;
    fn first(&mut self) -> Result<Option<usize>> {
        self.event("slots")?;
        Ok(self.from(0))
    }
    fn next(&mut self, k: usize) -> Result<Option<usize>> {
        self.event("next")?;
        Ok(self.from(k + 1))
    }
    fn resource_count(&mut self) -> Result<usize> {
        self.event("resources")?;
        Ok(self.resources)
    }
    fn id_18(&mut self, k: usize) -> Result<Vec<u16>> {
        self.read(k, "id")
    }
    fn key_194(&mut self, k: usize) -> Result<Vec<u16>> {
        self.read(k, "key")
    }
    fn text_198(&mut self, k: usize) -> Result<Vec<u16>> {
        self.read(k, "text")
    }
    fn name_24(&mut self, k: usize) -> Result<Vec<u16>> {
        self.event("name")?;
        Ok(words(&self.slots[k]["text"]))
    }
    fn set_name_49ae00(
        &mut self,
        key: &[u16],
        text: &mut Vec<u16>,
        supplied: &[u16],
    ) -> Result<()> {
        self.event("set_name")?;
        *text = supplied.to_vec();
        if !key.is_empty() {
            self.event("lookup")?;
            self.globals.iter_mut().find(|(k, _)| k == key).unwrap().1 = supplied.to_vec();
        }
        Ok(())
    }
    fn glyph_4e93c0(&mut self, k: usize) -> Result<Vec<u16>> {
        self.read(k, "glyph")
    }
    fn word_190(&mut self, k: usize) -> Result<u32> {
        self.scalar(k, "minor")
    }
    fn team_1ec(&mut self, k: usize) -> Result<Option<usize>> {
        self.event("team")?;
        Ok((!self.slots[k]["team"].is_null()).then_some(k))
    }
    fn team_name_18(&mut self, k: usize) -> Result<Vec<u16>> {
        self.event("team_name")?;
        Ok(words(&self.slots[k]["team"]))
    }
    fn nation_234(&mut self, k: usize) -> Result<Option<u32>> {
        self.definition(k, "nation")
    }
    fn faction_230_4(&mut self, k: usize) -> Result<Option<u32>> {
        self.definition(k, "faction")
    }
    fn color_1e8(&mut self, k: usize) -> Result<Option<u32>> {
        self.definition(k, "color")
    }
    fn handicap_244(&mut self, k: usize) -> Result<Option<u32>> {
        self.definition(k, "handicap")
    }
    fn byte_23b(&mut self, k: usize) -> Result<u8> {
        self.scalar(k, "allow").map(|v| v as u8)
    }
    fn byte_23c(&mut self, k: usize) -> Result<u8> {
        self.scalar(k, "eligible").map(|v| v as u8)
    }
    fn word_19c(&mut self, k: usize) -> Result<u32> {
        self.scalar(k, "mode")
    }
    fn string_240(&mut self, k: usize) -> Result<Vec<u16>> {
        self.read(k, "profile")
    }
    fn byte_23d(&mut self, k: usize) -> Result<u8> {
        self.scalar(k, "byte32").map(|v| v as u8)
    }
}
#[test]
fn original_v14_all_pc_current_kingdom_copy_and_text_mutation_prefixes() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-kingdoms.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 156);
    for case in cases {
        let mut input = Input::new(&case["input"]);
        let mut target = Table::from_native_constructor();
        let mut seed = Input::new(
            &json!({"slots":[{"id":[1],"key":[],"text":[],"glyph":[],"profile":[],"minor":1,"mode":1,"allow":1,"eligible":0,"byte32":1,"team":null,"nation":null,"faction":null,"color":null,"handicap":null}],"resources":1}),
        );
        native::capture_fields(&mut target, &mut seed).unwrap();
        assert_eq!(target.len(), 1);
        let outcome = native::capture_fields(&mut target, &mut input);
        let rows:Vec<_>=target.iter().map(|r|json!({"id":r.id,"key":r.key,"text":r.text,"glyph":r.glyph,"profile":r.profile,"team":r.team,
   "nation":r.nation,"faction":r.faction,"color":r.color,"handicap":r.handicap,"minor":r.minor,"eligible":r.eligible,"allow":r.allow,
   "byte30":r.byte30,"byte31":r.byte31,"byte32":r.byte32,"byte68":r.byte68,"resource_bits":r.resource_bits,"entry_count":r.localized_entries.len(),
   "definition_count":r.definitions.len(),"property_count":r.properties.len(),"string58":r.string58})).collect();
        assert_eq!(json!(rows), case["output"]["kingdoms"], "{}", case["pc"]);
        assert_eq!(
            json!(input.events),
            case["output"]["events"],
            "{}",
            case["pc"]
        );
        assert_eq!(json!(outcome.as_ref().err()), case["output"]["stopped"]);
        assert_eq!(
            json!(outcome.is_ok()),
            case["output"]["completed_kingdom_phase"]
        );
        let globals: Vec<_> = input
            .globals
            .iter()
            .map(|(key, text)| json!({"key":key,"text":text}))
            .collect();
        assert_eq!(json!(globals), case["output"]["globals"]);
    }
}
