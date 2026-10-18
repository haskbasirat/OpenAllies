use aa_runtime::film_parameters::{self, Result, State, Table, TeamFields};
use serde_json::{Value, json};
use std::cell::RefCell;
struct Input {
    slots: Vec<Option<usize>>,
    rows: Vec<Value>,
    events: RefCell<Vec<String>>,
    stop: Option<usize>,
}
impl Input {
    fn event(&self, name: &str) -> Result<()> {
        if self.stop == Some(self.events.borrow().len()) {
            return Err(name.into());
        }
        self.events.borrow_mut().push(name.into());
        Ok(())
    }
    fn words(&self, team: usize, key: &str) -> Vec<u16> {
        self.rows[team][key]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u16)
            .collect()
    }
}
impl TeamFields for Input {
    type Team = usize;
    fn slots(&self) -> Result<&[Option<usize>]> {
        self.event("slots")?;
        Ok(&self.slots)
    }
    fn name_18(&self, team: usize) -> Result<Vec<u16>> {
        self.event("id")?;
        Ok(self.words(team, "id"))
    }
    fn key_3c(&self, team: usize) -> Result<Vec<u16>> {
        self.event("key")?;
        Ok(self.words(team, "key"))
    }
    fn text_40(&self, team: usize) -> Result<Vec<u16>> {
        self.event("text")?;
        Ok(self.words(team, "text"))
    }
}
#[test]
fn original_v14_all_pc_full_team_phase_preserves_current_localized_pair() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-teams.json"
    ))
    .unwrap();
    let cases = data["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 36);
    for row in cases {
        let rows = row["input"]["slots"].as_array().unwrap().clone();
        let input = Input {
            slots: rows
                .iter()
                .enumerate()
                .map(|(i, r)| (!r.is_null()).then_some(i))
                .collect(),
            rows,
            events: RefCell::new(vec![]),
            stop: row["input"]["stop"].as_u64().map(|n| n as usize),
        };
        let state = State::from_native_constructor();
        assert_eq!(state.teams().len(), 0);
        assert_eq!(state.kingdom_count(), 0);
        let mut target = Table::from_native_constructor();
        assert_eq!(target.teams().len(), 0);
        //Seed a previous active Team to verify native count clear at the first
        //read-error boundary. This uses the same production value-copy kernel.
        let seed = Input {
            slots: vec![Some(0)],
            rows: vec![json!({"id":[1],"key":[2],"text":[3]})],
            events: RefCell::new(vec![]),
            stop: None,
        };
        film_parameters::capture_team_fields(&mut target, &seed).unwrap();
        let result = film_parameters::capture_team_fields(&mut target, &input);
        let teams: Vec<_> = target
            .teams()
            .map(|team| {
                let pair = team.display();
                json!({"id":team.id(),"key":pair.key(),"text":pair.text()})
            })
            .collect();
        assert_eq!(json!(teams), row["output"]["teams"]);
        assert_eq!(json!(*input.events.borrow()), row["output"]["events"]);
        assert_eq!(json!(result.as_ref().err()), row["output"]["stopped"]);
        assert_eq!(json!(result.is_ok()), row["output"]["completed_team_phase"]);
        //The original fixture carries a preexisting native Kingdom count7 and
        //stops before its reset. The Rust constructor has zero active Kingdoms;
        //this phase never changes either distinct initial count.
        assert_eq!(row["output"]["kingdom_count"], 7);
        assert_eq!(state.kingdom_count(), 0);
    }
}
