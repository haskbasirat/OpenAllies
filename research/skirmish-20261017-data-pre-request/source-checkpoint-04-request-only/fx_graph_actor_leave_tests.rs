//! Controlled loaded-node inputs; actual native parent/constructors are the
//! oracle. This does not label laboratory source nodes initialized actors.
use super::*;
use serde_json::{Value, json};
struct StopSource {
    ids: Option<String>,
    reads: Vec<String>,
}
impl Inputs for StopSource {
    type Source = ();
    fn text(&mut self, _: &(), name: &str) -> Result<String> {
        assert_eq!(name, "vfx_ids");
        self.reads.push(name.into());
        self.ids.clone().ok_or("required_ids".into())
    }
    fn provenance(&mut self, _: &()) -> Result<Vec<Provenance>> {
        unreachable!("derived-only native scope")
    }
    fn children(&mut self, _: &(), _: &str) -> Result<Vec<()>> {
        unreachable!("completed common loader")
    }
    fn integer(&mut self, _: &(), _: &str) -> Result<i32> {
        unreachable!()
    }
    fn scalar(&mut self, _: &(), _: &str) -> Result<u32> {
        unreachable!()
    }
    fn flag(&mut self, _: &(), _: &str) -> Result<bool> {
        unreachable!()
    }
    fn point(&mut self, _: &(), _: &str) -> Result<[u32; 3]> {
        unreachable!()
    }
    fn target_index(&mut self, _: &str) -> Result<u32> {
        unreachable!()
    }
    fn find(&mut self, _: &str, _: &str) -> Result<Option<Reference>> {
        unreachable!("same local node directory")
    }
    fn enum_value(&mut self, _: &str, _: &str) -> Result<i32> {
        unreachable!()
    }
}
#[test]
fn original_v14_stop_vfx_load_retains_target_before_attached_error() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-stored-state-effects/native-stop-vfx-load.json"
    ))
    .unwrap();
    assert_eq!(doc["rows"].as_array().unwrap().len(), 27);
    for row in doc["rows"].as_array().unwrap() {
        let input = &row["input"];
        let mut draft = Draft::new();
        for attached in input["targets"].as_array().unwrap() {
            let mut node = Node::create_vfx(Vec::new());
            node.bytes
                .insert(0x65, u8::from(attached.as_bool().unwrap()));
            draft.nodes.push(node);
            draft.kinds.push(FxcKind::CreateVfx);
        }
        for (name, index) in input["mapping"].as_object().unwrap() {
            draft
                .first_vfx
                .insert(name.clone(), NodeId(index.as_u64().unwrap() as usize));
        }
        let id = NodeId(draft.nodes.len());
        draft.nodes.push(construct_stop_vfx(Vec::new()));
        draft.kinds.push(FxcKind::StopVfx);
        let mut source = StopSource {
            ids: input["ids"].as_str().map(str::to_owned),
            reads: Vec::new(),
        };
        let result = load_stop_vfx(&mut source, &(), &mut draft, id);
        let outcome = match result {
            Ok(_) => "ok",
            Err(e) if e == "required_ids" => "required_ids",
            Err(e) if e.contains("42C974") => "unknown_vfx",
            Err(e) if e.contains("42C98C") => "unattached",
            Err(e) => panic!("unexpected reached error:{e}"),
        };
        let actual = json!({"outcome":outcome,"target":draft.nodes[id.0].node_links.get(&0x3c).map(|n|n.0),"complete":draft.nodes[id.0].complete});
        let mut expected = row["output"].clone();
        expected.as_object_mut().unwrap().remove("events");
        assert_eq!(actual, expected, "{} / {}", input["name"], row["precision"]);
        assert_eq!(source.reads, ["vfx_ids"]);
    }
}
#[test]
fn original_v14_actor_leave_recursive_stop_synthesis_preserves_links_and_order() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-stored-state-effects/native-actor-leave-stops.json"
    ))
    .unwrap();
    assert_eq!(doc["rows"].as_array().unwrap().len(), 366);
    for row in doc["rows"].as_array().unwrap() {
        let input = &row["input"];
        let inputs = input["nodes"].as_array().unwrap();
        let mut draft = Draft::new();
        for (i, n) in inputs.iter().enumerate() {
            let kind = match n["kind"].as_str().unwrap() {
                "create" => FxcKind::CreateVfx,
                "play" => FxcKind::PlaySfx,
                _ => FxcKind::StopVfx,
            };
            let mut node = match kind {
                FxcKind::CreateVfx => Node::create_vfx(Vec::new()),
                FxcKind::PlaySfx => sound::construct_play_sfx(Vec::new()),
                FxcKind::StopVfx => construct_stop_vfx(Vec::new()),
                FxcKind::RequestOnly { .. } => panic!("full ActorLeave fixture never admits request-only subtype fields"),
            };
            for (offset, value) in [
                (0x18, 0x100 + i as u32),
                (0x1c, 0x200 + i as u32),
                (0x24, 3 + i as u32),
                (0x28, 0x3e800000),
                (0x2c, 0xbf800000),
                (0x30, n["trigger"].as_u64().unwrap() as u32),
                (0x34, n["state"].as_u64().unwrap() as u32),
            ] {
                node.words.insert(offset, value);
            }
            if kind == FxcKind::CreateVfx {
                node.bytes
                    .insert(0x64, u8::from(n["duration"].as_bool().unwrap()));
                node.bytes
                    .insert(0x65, u8::from(n["attached"].as_bool().unwrap()));
            }
            node.immediate = n["immediate"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| NodeId(v.as_u64().unwrap() as usize))
                .collect();
            node.chain = n["chain"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| NodeId(v.as_u64().unwrap() as usize))
                .collect();
            node.complete = true;
            draft.nodes.push(node);
            draft.kinds.push(kind);
        }
        draft.roots = input["leave"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| NodeId(v.as_u64().unwrap() as usize))
            .collect();
        let roots = input["roots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| NodeId(v.as_u64().unwrap() as usize))
            .collect::<Vec<_>>();
        append_synthetic_stops(&roots, &mut draft, &mut |_, id| {
            Ok(inputs[id.0]["permanent"].as_bool().unwrap()
                && !inputs[id.0]["duration"].as_bool().unwrap())
        })
        .unwrap();
        let generated=draft.nodes.iter().skip(inputs.len()).map(|n|{
   let words=[0x18,0x1c,0x24,0x28,0x2c,0x30,0x34].into_iter().map(|o|(o.to_string(),json!(n.word(o).unwrap()))).collect::<serde_json::Map<_,_>>();
   json!({"words":words,"target":n.node_links.get(&0x3c).map(|n|n.0),"terrain":0,"flag":n.byte(0x38).unwrap(),"immediate":n.immediate.len(),"chain":n.chain.len()})
  }).collect::<Vec<_>>();
        let backlinks = draft
            .nodes
            .iter()
            .zip(inputs)
            .map(|(n, i)| {
                if i["kind"] == "create" {
                    n.node_links.get(&0x68).map(|n| n.0)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            json!({"roots":draft.roots.iter().map(|n|n.0).collect::<Vec<_>>(),"generated":generated,"backlinks":backlinks}),
            row["output"],
            "{input}"
        );
    }
}
