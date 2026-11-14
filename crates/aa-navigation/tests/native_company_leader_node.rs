//! Replays research/phase8-movement/native-company-leader-node.json
//! (original 51F7A0 with 51E8D0, 52CDB0, 51D7D0 and 52A1F0 under Unicorn,
//! at PC24/PC53/PC64) against aa_navigation::company_leader_node.
use aa_navigation::{
    OwnedPath, PathNode, Result,
    company_leader_node::{LeaderNodeWorld, leader_node_reached},
};
use aa_numeric::Precision;
use serde_json::{Value, json};

const LEADER: u32 = 7;
const OTHER: u32 = 8;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-movement/native-company-leader-node.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn pair(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}

struct Host {
    c: Value,
    events: Vec<Value>,
}
impl LeaderNodeWorld for Host {
    fn state_is_current(&mut self, offset: u32) -> Result<bool> {
        let r = self.c["current"][offset.to_string()].as_bool().unwrap();
        self.events
            .push(json!({"event": "current", "offset": offset, "result": r}));
        Ok(r)
    }
    fn leader(&mut self) -> Result<u32> {
        Ok(LEADER)
    }
    fn leader_path_nodes(&mut self, leader: u32) -> Result<i32> {
        assert_eq!(leader, LEADER);
        Ok(self.c["leader_nodes"].as_array().unwrap().len() as i32)
    }
    fn face(&mut self, point: [u32; 2]) -> Result<()> {
        self.events.push(json!({"event": "face", "point": point}));
        Ok(())
    }
    fn move_status(&mut self) -> Result<u32> {
        Ok(u(&self.c["move_status"]))
    }
    fn move_goal(&mut self) -> Result<[u32; 2]> {
        Ok(pair(&self.c["goal"]))
    }
    fn organization_position(&mut self) -> Result<[u32; 2]> {
        Ok(pair(&self.c["org_pos"]))
    }
    fn mind_virtual_144(&mut self) -> Result<bool> {
        let r = self.c["v144"].as_bool().unwrap();
        self.events
            .push(json!({"event": "mind_virtual_144", "result": r}));
        Ok(r)
    }
}

fn node(v: &Value) -> PathNode {
    let mut words = [0; 8];
    for (k, w) in v.as_array().unwrap().iter().enumerate() {
        words[k] = u(w);
    }
    PathNode { words }
}

#[test]
fn native_company_leader_node_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 1080);
    assert_eq!(f["coverage"]["51F7A0"], json!([83, 83]));
    assert_eq!(f["coverage"]["51E8D0"], json!([10, 10]));
    assert_eq!(f["coverage"]["52CDB0"], json!([31, 31]));
    assert_eq!(f["coverage"]["52A1F0"], json!([28, 28]));
    let mut arrivals = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut path = OwnedPath {
            nodes: c["nodes"].as_array().unwrap().iter().map(node).collect(),
            kind1_count: c["kind1"].as_i64().unwrap() as i32,
            kind2_count: c["kind2"].as_i64().unwrap() as i32,
            dirty: false,
            word18: u(&c["word18"]),
        };
        let mut host = Host {
            c: c.clone(),
            events: vec![],
        };
        let leader = if c["arg_is_leader"].as_bool().unwrap() {
            LEADER
        } else {
            OTHER
        };
        let r = leader_node_reached(leader, &mut path, &mut host, p).unwrap();
        assert_eq!(json!(host.events), o["events"], "{name}");
        assert_eq!(json!(u8::from(r)), o["result"], "{name}");
        let nodes: Vec<Value> = path.nodes.iter().map(|n| json!(n.words)).collect();
        assert_eq!(json!(nodes), o["nodes"], "{name}");
        assert_eq!(json!(path.kind1_count), o["kind1"], "{name}");
        assert_eq!(json!(path.kind2_count), o["kind2"], "{name}");
        assert_eq!(json!(u8::from(path.dirty)), o["dirty"], "{name}");
        assert_eq!(json!(path.word18), o["word18"], "{name}");
        assert_eq!(o["tail_is_last"], json!(true), "{name}");
        if host.events.iter().any(|e| e["event"] == "mind_virtual_144") {
            arrivals += 1;
        }
    }
    assert_eq!(arrivals, 174);
}
