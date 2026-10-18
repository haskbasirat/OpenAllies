use aa_mind::{
    MindId, Result, StateId,
    element_command::{self, Actor, Command, Element, World},
};
use aa_navigation::Activation;
use aa_numeric::Precision;
use serde_json::{Value, json};
struct Host {
    input: Value,
    events: Vec<Value>,
    accept_count: usize,
}
impl Host {
    fn event(&mut self, name: &str, args: &[u32]) {
        let mut v = vec![json!(name)];
        v.extend(args.iter().map(|n| json!(n)));
        self.events.push(Value::Array(v));
        if self.input["mutations"]
            .as_array()
            .unwrap()
            .first()
            .is_some_and(|m| m["event"] == name)
        {
            let m = self.input["mutations"].as_array_mut().unwrap().remove(0);
            for section in ["actor", "command", "element"] {
                if let Some(changes) = m[section].as_object() {
                    for (k, v) in changes {
                        self.input[section][k] = v.clone();
                    }
                }
            }
        }
    }
    fn outcome(&self, name: &str) -> bool {
        self.input[name].as_bool().unwrap()
    }
}
impl World for Host {
    fn command(&self, _: u32) -> Result<Command> {
        Ok(serde_json::from_value(self.input["command"].clone()).unwrap())
    }
    fn element(&self, _: MindId) -> Result<Element> {
        Ok(serde_json::from_value(self.input["element"].clone()).unwrap())
    }
    fn actor(&self, _: u32) -> Result<Actor> {
        Ok(serde_json::from_value(self.input["actor"].clone()).unwrap())
    }
    fn resolve(&self, id: u32) -> Result<Option<u32>> {
        Ok(self.input["registry"][id.to_string()]
            .as_u64()
            .map(|n| n as u32))
    }
    fn active(&self, _: StateId) -> Result<bool> {
        Ok(self.outcome("active"))
    }
    fn schedule(&mut self, m: MindId, d: u32, v: u32) -> Result<()> {
        self.event("schedule", &[m.0, d, v]);
        Ok(())
    }
    fn indirect_fire(&mut self, p: u32) -> Result<bool> {
        self.event("indirect", &[p]);
        Ok(self.outcome("indirect"))
    }
    fn target_accepts(&mut self, t: u32, a: u32) -> Result<bool> {
        self.event("accept", &[t, a]);
        let values = self.input["accept"].as_array().unwrap();
        let v = values[self.accept_count.min(values.len() - 1)]
            .as_bool()
            .unwrap();
        self.accept_count += 1;
        Ok(v)
    }
    fn activate_sleep(&mut self, s: StateId, m: u32) -> Result<bool> {
        self.event("sleep", &[s.0, m]);
        Ok(self.outcome("sleep_result"))
    }
    fn activate_move(&mut self, s: StateId, r: Activation) -> Result<bool> {
        self.event(
            "move",
            &[
                s.0,
                r.activation_mode,
                r.destination_bits[0],
                r.destination_bits[1],
                r.movement_mode,
                r.requested_speed_bits,
                r.auxiliary_bits[0],
                r.auxiliary_bits[1],
            ],
        );
        Ok(self.outcome("move_result"))
    }
    fn activate_kill(&mut self, s: StateId, m: u32, t: u32) -> Result<bool> {
        self.event("kill", &[s.0, m, t]);
        Ok(self.outcome("kill_result"))
    }
    fn activate_bombard(&mut self, s: StateId, m: u32, t: u32) -> Result<bool> {
        self.event("bombard", &[s.0, m, t]);
        Ok(self.outcome("bombard_result"))
    }
    fn activate_follow(
        &mut self,
        s: StateId,
        m: u32,
        t: u32,
        mode: u32,
        speed: u32,
        aux: [u32; 2],
    ) -> Result<bool> {
        self.event("follow", &[s.0, m, t, mode, speed, aux[0], aux[1]]);
        Ok(self.outcome("follow_result"))
    }
}
#[test]
fn native_contextual_move_point_actor_branches_and_cached_parameters() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-mind/native-element-command.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 123);
    for row in cases {
        let mut host = Host {
            input: row["input"].clone(),
            events: vec![],
            accept_count: 0,
        };
        let precision: Precision = serde_json::from_value(host.input["precision"].clone()).unwrap();
        let accepted = element_command::move_command(MindId(1), 1, &mut host, precision).unwrap();
        assert_eq!(
            json!({"accepted":accepted,"events":host.events}),
            row["output"],
            "{} {:?}",
            host.input["name"],
            precision
        );
        assert!(host.input["mutations"].as_array().unwrap().is_empty());
    }
}
