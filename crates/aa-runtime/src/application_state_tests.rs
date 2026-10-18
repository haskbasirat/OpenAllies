use super::*;
use serde_json::{json, Value};
struct Node(u32); //Deliberately no Clone/Copy requirement.
struct Arg(u32); //The kernel must forward these same borrowed allocations.
struct Lab<'a> {
    input: &'a Value,
    events: Vec<Value>,
    arg8: *const Arg,
    arg_c: *const Arg,
}
impl Lab<'_> {
    fn mutate(&self, application: &mut State, label: &str) {
        if let Some(v) = self.input["mutate"][label].as_array() {
            application.current_10c = v[0].as_u64().unwrap() as u32;
            application.previous_110 = v[1].as_u64().unwrap() as u32;
        }
    }
    fn stop(&self, label: &str) -> Result<()> {
        if self.input["stop"] == label {
            Err(format!("required_{label}"))
        } else {
            Ok(())
        }
    }
}
impl Services for Lab<'_> {
    type Node = Node;
    type Arg8 = Arg;
    type ArgC = Arg;
    fn diagnostic(&mut self, app: &mut State, from: u32, to: u32) -> Result<()> {
        self.events.push(json!([
            "diagnostic",
            from,
            to,
            app.current(),
            app.previous()
        ]));
        self.mutate(app, "diagnostic");
        self.stop("diagnostic")
    }
    fn state_node(&mut self, index: u32) -> Result<Option<Node>> {
        self.events.push(json!(["node", index]));
        Ok((!self.input["null_nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_u64() == Some(index.into())))
        .then_some(Node(index)))
    }
    fn leave(&mut self, app: &mut State, node: Node) -> Result<()> {
        self.events
            .push(json!(["leave", node.0, app.current(), app.previous()]));
        self.mutate(app, "leave");
        self.stop("leave")
    }
    fn enter(&mut self, app: &mut State, node: Node, arg8: &Arg, arg_c: &Arg) -> Result<()> {
        assert_eq!(arg8 as *const Arg, self.arg8);
        assert_eq!(arg_c as *const Arg, self.arg_c);
        self.events.push(json!([
            "enter",
            node.0,
            arg8.0,
            arg_c.0,
            app.current(),
            app.previous()
        ]));
        self.mutate(app, "enter");
        self.stop("enter")
    }
}
#[test]
fn original_application_state_fields_order_rereads_and_failures_all198_rows() {
    let native: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-application-state/native-application-state.json"
    ))
    .unwrap();
    assert_eq!(native["rows"].as_array().unwrap().len(), 198);
    for row in native["rows"].as_array().unwrap() {
        let input = &row["input"];
        let mut app = State {
            current_10c: input["current"].as_u64().unwrap() as u32,
            previous_110: input["previous"].as_u64().unwrap() as u32,
        };
        let arg8 = Arg(input["arg8"].as_u64().unwrap() as u32);
        let arg_c = Arg(input["arg_c"].as_u64().unwrap() as u32);
        let mut services = Lab {
            input,
            events: vec![],
            arg8: &arg8,
            arg_c: &arg_c,
        };
        let result = app.set_state(
            input["requested"].as_u64().unwrap() as u32,
            &arg8,
            &arg_c,
            &mut services,
        );
        let status = match result {
            Ok(()) => "ok".into(),
            Err(e) if e.contains("40CE5A") => "null_leave".into(),
            Err(e) if e.contains("40CE84") => "null_enter".into(),
            Err(e) => e,
        };
        assert_eq!(
            json!({"current":app.current(),"previous":app.previous(),"events":services.events,"status":status}),
            row["output"],
            "{} / {input}",
            row["precision"]
        );
    }
    for row in native["constructor_rows"].as_array().unwrap() {
        let app = State::from_constructor();
        assert_eq!(
            json!({"current":app.current(),"previous":app.previous()}),
            row["output"]
        );
    }
}

#[test]
fn real_reentrant_setter_updates_same_owner_without_raw_setter() {
    struct Basic;
    impl Services for Basic {
        type Node = Node;
        type Arg8 = Arg;
        type ArgC = Arg;
        fn diagnostic(&mut self, _: &mut State, _: u32, _: u32) -> Result<()> {
            Ok(())
        }
        fn state_node(&mut self, index: u32) -> Result<Option<Node>> {
            Ok(Some(Node(index)))
        }
        fn leave(&mut self, _: &mut State, _: Node) -> Result<()> {
            Ok(())
        }
        fn enter(&mut self, _: &mut State, _: Node, _: &Arg, _: &Arg) -> Result<()> {
            Ok(())
        }
    }
    struct Reenter<'a> {
        arg8: &'a Arg,
        arg_c: &'a Arg,
        seen: Vec<(u32, u32, u32)>,
    }
    impl Services for Reenter<'_> {
        type Node = Node;
        type Arg8 = Arg;
        type ArgC = Arg;
        fn diagnostic(&mut self, _: &mut State, _: u32, _: u32) -> Result<()> {
            Ok(())
        }
        fn state_node(&mut self, index: u32) -> Result<Option<Node>> {
            Ok(Some(Node(index)))
        }
        fn leave(&mut self, app: &mut State, node: Node) -> Result<()> {
            self.seen.push((node.0, app.current(), app.previous()));
            app.set_state(3, self.arg8, self.arg_c, &mut Basic)
        }
        fn enter(&mut self, app: &mut State, node: Node, _: &Arg, _: &Arg) -> Result<()> {
            self.seen.push((node.0, app.current(), app.previous()));
            Ok(())
        }
    }
    let (arg8, arg_c) = (Arg(17), Arg(19));
    let mut app = State::from_constructor();
    app.set_state(2, &arg8, &arg_c, &mut Basic).unwrap();
    let mut services = Reenter {
        arg8: &arg8,
        arg_c: &arg_c,
        seen: vec![],
    };
    app.set_state(6, &arg8, &arg_c, &mut services).unwrap();
    assert_eq!(services.seen, vec![(2, 2, 0), (6, 6, 3)]);
    assert_eq!((app.current(), app.previous()), (6, 3));
}
