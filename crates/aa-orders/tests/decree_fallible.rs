use aa_orders::commands::{
    ActorOutcome, ActorSlotReference, TryActorCommandContext, try_process_actor,
};
use serde_json::Value;
fn fixture() -> Value {
    serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/skirmish-20261016-player-orders/decree/native-decree.json"),
        )
        .unwrap(),
    )
    .unwrap()
}
struct Host<'a> {
    input: &'a Value,
    events: Vec<&'static str>,
}
impl Host<'_> {
    fn event(&mut self, name: &'static str) -> Result<(), String> {
        self.events.push(name);
        if self.input["stop"].as_str() == Some(name) {
            Err(name.into())
        } else {
            Ok(())
        }
    }
}
impl TryActorCommandContext for Host<'_> {
    type Actor = ();
    type Command = ();
    type Group = ();
    type Error = String;
    fn resolve_slot(&mut self, _: ActorSlotReference) -> Result<Option<()>, String> {
        Ok((self.input["present"] == true).then_some(()))
    }
    fn permit(&mut self, _: &(), _: &mut ()) -> Result<bool, String> {
        self.event("permit")?;
        let bits = self.input["actions"].as_u64().unwrap();
        let shift = self.input["kind"].as_u64().unwrap() as u8;
        Ok(shift < 64 && bits & (1u64 << shift) != 0)
    }
    fn has_valid_link(&mut self, _: &()) -> Result<bool, String> {
        Ok(self.input["group"] == "valid")
    }
    fn linked_group(&mut self, _: &()) -> Result<(), String> {
        self.event("linked_group")
    }
    fn prepare_group(&mut self, _: ()) -> Result<(), String> {
        self.event("prepare_group")
    }
    fn deliver(&mut self, _: &(), _: &mut ()) -> Result<(), String> {
        self.event("deliver")
    }
}
#[test]
fn actor_decree_required_stops_and_native_group_reread_order() {
    let mut checked = 0;
    for row in fixture()["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "actor")
    {
        let mut host = Host {
            input: &row["input"],
            events: Vec::new(),
        };
        let result = try_process_actor(ActorSlotReference(2), &mut (), &mut host);
        assert_eq!(
            result.as_ref().err().map_or("complete", String::as_str),
            row["output"]["status"],
            "{row}"
        );
        assert_eq!(
            serde_json::json!(host.events),
            row["output"]["events"],
            "{row}"
        );
        if let Ok(value) = result {
            let expected = if row["input"]["present"] != true {
                ActorOutcome::ActorNotFound
            } else if row["output"]["allowed"] == false {
                ActorOutcome::PermissionDenied
            } else {
                ActorOutcome::Delivered
            };
            assert_eq!(value, expected, "{row}");
        }
        checked += 1;
    }
    assert_eq!(checked, 1152);
}

#[derive(Debug)]
struct SelectedError(String);
impl From<aa_orders::Error> for SelectedError {
    fn from(e: aa_orders::Error) -> Self {
        Self(e.to_string())
    }
}
struct SelectionHost<'a> {
    input: &'a Value,
    events: Vec<&'static str>,
}
impl SelectionHost<'_> {
    fn event(&mut self, name: &'static str) -> Result<(), SelectedError> {
        self.events.push(name);
        if name == "default" || self.input["stop"].as_str() == Some(name) {
            Err(SelectedError(name.into()))
        } else {
            Ok(())
        }
    }
}
impl aa_orders::commands::TrySelectedCommandContext for SelectionHost<'_> {
    type Selection = ();
    type Command = ();
    type Error = SelectedError;
    fn current_selection(&mut self) -> Result<Option<()>, SelectedError> {
        Ok(Some(()))
    }
    fn is_empty(&mut self, _: &()) -> Result<bool, SelectedError> {
        Ok(self.input["empty"] == true)
    }
    fn inspect_empty(&mut self, _: &(), _: &mut ()) -> Result<(), SelectedError> {
        self.event("diagnostic")
    }
    fn inspect_selected(&mut self, _: &(), _: &mut ()) -> Result<(), SelectedError> {
        if self.input["explicit"] != true {
            self.event("default")?;
        }
        self.event("diagnostic")
    }
    fn deliver_selection(&mut self, _: &(), _: &mut ()) -> Result<(), SelectedError> {
        self.event("delivery")
    }
}
#[test]
fn selected_decree_empty_and_nonempty_diagnostics_remain_required_before_delivery() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-player-orders/decree/native-selected-decree.json");
    let doc: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut count = 0;
    for row in doc["rows"].as_array().unwrap() {
        let mut host = SelectionHost {
            input: &row["input"],
            events: Vec::new(),
        };
        let result = aa_orders::commands::try_process_selected(&mut (), &mut host);
        assert_eq!(
            result.as_ref().err().map_or("complete", |e| e.0.as_str()),
            row["output"]["status"]
        );
        assert_eq!(serde_json::json!(host.events), row["output"]["events"]);
        if let Ok(out) = result {
            assert_eq!(
                out,
                if row["input"]["empty"] == true {
                    aa_orders::commands::SelectedOutcome::EmptySelection
                } else {
                    aa_orders::commands::SelectedOutcome::DeliveredToSelection
                }
            );
        }
        count += 1;
    }
    assert_eq!(count, 72);
}
