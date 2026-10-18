use super::super::*;
use serde_json::{Value, json};
use std::sync::Arc;
struct Fixture {
    case: Value,
    events: Vec<Value>,
    application: Arc<()>,
    pc: Precision,
}
impl Fixture {
    fn get(&mut self, node: usize, name: &str) -> Result<Value> {
        self.events.push(json!(["get", node, name]));
        self.stop(name)?;
        self.case["nodes"][node]
            .get(name)
            .cloned()
            .ok_or_else(|| format!("fixture missing {name}"))
    }
    fn stop(&self, name: &str) -> Result<()> {
        if self.case["fail"] == name {
            Err(format!("required {name}"))
        } else {
            Ok(())
        }
    }
}
impl Inputs for Fixture {
    type Source = usize;
    fn provenance(&mut self, _: &usize) -> Result<Vec<Provenance>> {
        Ok(vec![])
    }
    fn children(&mut self, n: &usize, name: &str) -> Result<Vec<usize>> {
        self.events.push(json!(["children", n, name]));
        Ok(self.case["nodes"][*n][name]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect())
    }
    fn text(&mut self, n: &usize, name: &str) -> Result<String> {
        Ok(self.get(*n, name)?.as_str().unwrap().into())
    }
    fn integer(&mut self, n: &usize, name: &str) -> Result<i32> {
        Ok(self.get(*n, name)?.as_i64().unwrap() as i32)
    }
    fn scalar(&mut self, n: &usize, name: &str) -> Result<u32> {
        Ok(self.get(*n, name)?.as_u64().unwrap() as u32)
    }
    fn fixed(&mut self, n: &usize, name: &str) -> Result<u32> {
        self.scalar(n, name)
    }
    fn flag(&mut self, n: &usize, name: &str) -> Result<bool> {
        Ok(self.get(*n, name)?.as_bool().unwrap())
    }
    fn point(&mut self, n: &usize, name: &str) -> Result<[u32; 3]> {
        Ok(self
            .get(*n, name)?
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap())
    }
    fn target_index(&mut self, ids: &str) -> Result<u32> {
        self.events.push(json!(["target", ids]));
        match ids {
            "friend" => Ok(0),
            "enemy" => Ok(1),
            "body" => Ok(12),
            _ => Err("unknown target".into()),
        }
    }
    fn find(&mut self, manager: &str, ids: &str) -> Result<Option<Reference>> {
        self.events.push(json!(["find", manager, ids]));
        self.stop(&format!("find:{manager}"))?;
        Ok((!ids.is_empty() && ids != "missing").then(|| Reference {
            id: ids.into(),
            source: vec![],
            ordinal: Some(if ids == "head" { 3 } else { 0 }),
            cutscene: false,
        }))
    }
    fn enum_value(&mut self, domain: &str, ids: &str) -> Result<i32> {
        self.events.push(json!(["enum", domain, ids]));
        self.stop(&format!("enum:{domain}"))?;
        let names: &[&str] = match domain {
            "FXC" => &aa_rules::abilities::FXC_TYPES,
            "EquipmentAction" => &["HIDE", "UNHIDE", "FLIP", "REMOVE", "ADD", "REPLACE"],
            "AT" => &[
                "SELF",
                "TARGET_THING",
                "SOURCE_GROUND",
                "TARGET_GROUND",
                "SOURCE_ABILITY_POSITION",
                "TARGET_HIT_POSITION",
                "MAP_CENTER",
            ],
            "HealthTrigger" => &[
                "ABSOLUTE_BELOW",
                "ABSOLUTE_ABOVE",
                "RELATIVE_DECREASE",
                "RELATIVE_INCREASE",
            ],
            _ => return Err("unknown enum".into()),
        };
        Ok(names
            .iter()
            .position(|n| n.eq_ignore_ascii_case(ids))
            .map(|i| i as i32)
            .unwrap_or(-1))
    }
    fn texture_reference(
        &mut self,
        _: &usize,
        ids: &str,
    ) -> Result<Option<crate::resource_objects::Handle>> {
        self.events.push(json!(["texture_path", ids]));
        self.stop("texture_path")?;
        self.events.push(json!(["texture", ids, 0]));
        self.stop("texture")?;
        if ids == "missing" {
            return Ok(None);
        }
        //Explicit synthetic allocation transport for this native core fixture.
        //No real file/default/depot/source-ready authority is asserted.
        let source =
            crate::resource_objects::SourceFile::from_catalog(&aa_rules::sources::CatalogEntry {
                logical_path: ids.into(),
                size: 0,
                origin: aa_rules::sources::Origin::V14Loose,
            });
        Ok(Some(crate::resource_objects::Handle::new(
            &self.application,
            crate::resource_objects::Kind::Texture,
            ids.encode_utf16().collect(),
            source,
        )))
    }
    fn radar_texture_policy(&mut self, texture: &crate::resource_objects::Handle) -> Result<()> {
        texture.require_application(&self.application)?;
        assert_eq!(texture.kind(), crate::resource_objects::Kind::Texture);
        for name in ["texture_push", "texture_read", "texture_render"] {
            self.events.push(json!([
                name,
                String::from_utf16(texture.logical_ids()).unwrap()
            ]));
            self.stop(name)?;
        }
        Ok(())
    }
    fn color_490e50(&mut self, n: &usize, name: &str, alpha: u8) -> Result<u32> {
        super::color_490e50(self.point(n, name)?, alpha, self.pc)
    }
}
fn snapshot(d: &Draft) -> Value {
    let nodes=d.nodes.iter().enumerate().map(|(index,n)|{
        let kind=d.kinds[index];let number=match kind{FxcKind::Equipment=>2,FxcKind::StopSfx=>4,FxcKind::ChangeAspect=>5,FxcKind::ChangeSkin=>6,FxcKind::CreateRfx=>7,FxcKind::PlayHitEffect=>9,_=>panic!("fixture unexpected kind")};
        let active=actor_state::active_fields(kind,||panic!("no permanent read"),||panic!("no duration read")).unwrap();
        let mut value=json!({"kind":number,"words":n.words,"bytes":n.bytes,
            "references":n.references.iter().map(|(k,v)|(k.to_string(),v.as_ref().map(|r|r.id.clone()))).collect::<BTreeMap<_,_>>(),
            "immediate":n.immediate,"chain":n.chain,"active":u8::from(active),"complete":n.complete});
        if number==5{value["texts"]=json!(n.texts);}
        if matches!(number,6|7){value["textures"]=json!(n.textures.iter().map(|(k,v)|(k.to_string(),v.as_ref().map(|h|String::from_utf16(h.logical_ids()).unwrap()))).collect::<BTreeMap<_,_>>());}
        value
    }).collect::<Vec<_>>();
    json!({"nodes":nodes,"roots":d.roots,"warnings":d.warnings})
}
#[test]
fn actual_native_six_subtype_fields_order_strings_resources_and_errors_all_precision() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-fxc-subtypes/native-fxc-subtypes.json"
    ))
    .unwrap();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 279);
    for row in rows {
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let roots = row["input"]["roots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        let mut input = Fixture {
            case: row["input"].clone(),
            events: vec![],
            application: Arc::new(()),
            pc,
        };
        let (draft, failed) = match compile_inputs(&mut input, roots) {
            Ok(d) => (d, false),
            Err(e) => (e.partial, true),
        };
        assert!(
            !draft
                .kinds
                .iter()
                .any(|k| matches!(k, FxcKind::RequestOnly { .. }))
        );
        let mut actual = snapshot(&draft);
        actual["events"] = json!(input.events);
        actual["failed"] = json!(failed);
        let mut expected = row["output"].clone();
        expected["failed"] = json!(!expected["error"].is_null());
        expected.as_object_mut().unwrap().remove("error");
        assert_eq!(
            actual, expected,
            "{} / {}",
            row["input"]["name"], row["precision"]
        );
    }
}
#[test]
#[ignore = "requires installed V14 source; source compiler only, no source Bank/runtime readiness"]
fn actual_equipment_source_units_compile_full_derived_fields() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut count = 0;
    for id in ["usa_infantry_mg", "usa_infantry_flame", "usa_paratrooper"] {
        let root = actors.resolved_actor(id).unwrap();
        let source = actors
            .actors
            .iter()
            .find(|a| a.id.eq_ignore_ascii_case(id))
            .unwrap()
            .source
            .clone();
        let mut input = Bound::new(&actors, Precision::Double53, source).unwrap();
        let mut draft = Draft::new();
        if let Some(view) = root.child("View").unwrap() {
            for name in [
                "WhenCreated",
                "WhenDestroyed",
                "WhenSelected",
                "WhenCommanded",
                "WhenCommandedAggressive",
                "WhenConstructed",
            ] {
                append_roots(&mut input, view.children(name).unwrap(), &mut draft).unwrap();
            }
        }
        append_roots(&mut input, root.children("WhenEnter").unwrap(), &mut draft).unwrap();
        for (i, kind) in draft.kinds.iter().enumerate() {
            if *kind == FxcKind::Equipment {
                let n = &draft.nodes[i];
                assert!(n.complete());
                assert!(n.word(0x3c).is_some());
                assert!(n.word(0x40).is_some());
                assert!(n.reference(0x44).is_some());
                assert!(n.reference(0x48).is_some());
                count += 1;
            }
        }
    }
    assert!(count > 0);
    println!("actual3 units complete Equipment nodes={count}");
}
