use aa_runtime::film_parameter_extras::{self as core, Fields, Register, Result, Target};
use aa_triggers::decode::{Parameter, Span, Value as Stored};
use serde_json::{Value, json};
fn words(v: &Value) -> Vec<u16> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_u64().unwrap() as u16)
        .collect()
}
fn prefix(s: &[u16]) -> &[u16] {
    &s[..s.iter().position(|&u| u == 0).unwrap_or(s.len())]
}
struct Parameters {
    ids: Vec<Vec<u16>>,
    entries: Vec<Vec<Value>>,
}
impl Target for Parameters {
    fn find_50dc00(&self, name: &[u16]) -> Option<usize> {
        self.ids.iter().position(|id| prefix(id) == prefix(name))
    }
    fn append_510150(&mut self, index: usize, key: Vec<u16>, text: Vec<u16>) -> Result<()> {
        self.entries[index].push(json!({"key":key,"text":text}));
        Ok(())
    }
}
struct Input {
    source: Value,
    events: Vec<String>,
    stop: Option<usize>,
}
impl Input {
    fn event(&mut self, n: &str) -> Result<()> {
        if self.stop == Some(self.events.len()) {
            return Err(n.into());
        }
        self.events.push(n.into());
        Ok(())
    }
    fn action(&self, a: (usize, usize)) -> &Value {
        &self.source["triggers"][a.0]["actions"][a.1]
    }
}
impl Fields for Input {
    type Trigger = usize;
    type Action = (usize, usize);
    type Actor = usize;
    type Objective = usize;
    fn first_trigger(&mut self) -> Result<Option<usize>> {
        self.event("triggers")?;
        Ok((!self.source["triggers"].as_array().unwrap().is_empty()).then_some(0))
    }
    fn next_trigger(&mut self, t: usize) -> Result<Option<usize>> {
        self.event("next_trigger")?;
        Ok((t + 1 < self.source["triggers"].as_array().unwrap().len()).then_some(t + 1))
    }
    fn enabled(&mut self, t: usize) -> Result<u8> {
        self.event("enabled")?;
        Ok(self.source["triggers"][t]["enabled"].as_u64().unwrap() as u8)
    }
    fn conditions_present(&mut self, t: usize) -> Result<bool> {
        self.event("conditions")?;
        Ok(self.source["triggers"][t]["conditions"].as_bool().unwrap())
    }
    fn first_action(&mut self, t: usize) -> Result<Option<(usize, usize)>> {
        self.event("actions")?;
        Ok((!self.source["triggers"][t]["actions"]
            .as_array()
            .unwrap()
            .is_empty())
        .then_some((t, 0)))
    }
    fn next_action(&mut self, a: (usize, usize)) -> Result<Option<(usize, usize)>> {
        self.event("next_action")?;
        Ok((a.1 + 1
            < self.source["triggers"][a.0]["actions"]
                .as_array()
                .unwrap()
                .len())
        .then_some((a.0, a.1 + 1)))
    }
    fn opcode(&mut self, a: (usize, usize)) -> Result<u32> {
        self.event("opcode")?;
        Ok(self.action(a)["opcode"].as_u64().unwrap() as u32)
    }
    fn convert_5422f0(&mut self, a: (usize, usize), r: &mut [Register; 2]) -> Result<()> {
        self.event("convert")?;
        let params: Vec<_> = self.action(a)["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                let storage = p["storage"].as_u64().unwrap() as u32;
                let v = &p["value"];
                let value = match storage {
                    1 | 2 => Stored::Word(v.as_u64().unwrap() as u32),
                    5 => Stored::Position(
                        v.as_array()
                            .unwrap()
                            .iter()
                            .map(|n| n.as_u64().unwrap() as u32)
                            .collect::<Vec<_>>()
                            .try_into()
                            .unwrap(),
                    ),
                    6 => Stored::Text(words(v)),
                    7 => Stored::Localized {
                        key: words(&v[0]),
                        inline: Some(words(&v[1])),
                    },
                    _ => Stored::Empty,
                };
                Parameter {
                    source: Span {
                        offset: 0,
                        length: 0,
                    },
                    argument: String::new(),
                    item: String::new(),
                    item_index: p["item"].as_u64().unwrap() as u32,
                    storage,
                    flag: 0,
                    value,
                }
            })
            .collect();
        core::convert(&params, r)
    }
    fn kingdom_54dac0(&mut self, r: &Register) -> Result<Option<usize>> {
        self.event("kingdom")?;
        let actors = self.source["actors"].as_array().unwrap();
        Ok(match r.item {
            18 => actors.iter().position(|a| {
                a["kingdom"].as_bool().unwrap() && prefix(&words(&a["name"])) == prefix(&r.text)
            }),
            19 | 23 | 24 => {
                let raw = match r.item {
                    23 => self.source["condition"].as_u64().unwrap() as u32,
                    24 => self.source["event"].as_u64().unwrap() as u32,
                    _ => r.word,
                };
                actors
                    .iter()
                    .position(|a| a["raw"].as_u64() == Some(u64::from(raw)))
            }
            _ => None,
        })
    }
    fn objective_54da70(&mut self, r: &Register) -> Result<Option<usize>> {
        self.event("objective")?;
        Ok(self.source["objectives"]
            .as_array()
            .unwrap()
            .iter()
            .position(|o| prefix(&words(&o["id"])) == prefix(&r.text)))
    }
    fn name_18(&mut self, a: &usize) -> Result<Vec<u16>> {
        self.event("name")?;
        Ok(words(&self.source["actors"][*a]["name"]))
    }
    fn localized_49ad00(&mut self, o: &usize) -> Result<(Vec<u16>, Vec<u16>)> {
        self.event("localized")?;
        let obj = &self.source["objectives"][*o];
        Ok((words(&obj["id"]), words(&obj["text"])))
    }
}
#[test]
fn original_v14_all_pc_trigger_objective_scratch_reuse_and_partial_prefixes() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-extras.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    for row in cases {
        let source = row["input"].clone();
        let ids: Vec<_> = source["parameter_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(words)
            .collect();
        let mut target = Parameters {
            entries: ids.iter().map(|_| Vec::new()).collect(),
            ids,
        };
        let stop = source["stop"].as_u64().map(|n| n as usize);
        let mut input = Input {
            source,
            events: Vec::new(),
            stop,
        };
        let result = core::capture_fields(&mut target, &mut input);
        let stopped = result.err().map(|e| {
            if e.contains("null KingdomParameter") {
                "destination".into()
            } else {
                e
            }
        });
        assert_eq!(
            json!({"events":input.events,"stopped":stopped,"entries":target.entries}),
            row["output"],
            "{}",
            row["pc"]
        );
    }
}
