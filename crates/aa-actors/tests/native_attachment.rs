use aa_actors::attachment::{self, Settlement, Structure, World};
use serde_json::Value;
struct Fixture {
    structure: Option<Structure<u32>>,
    settlement: Option<Settlement<u32>>,
    virtual_kind: String,
    replace: bool,
    remove: bool,
}
impl World for Fixture {
    type Actor = u32;
    fn structure(&self, _: u32) -> attachment::Result<Option<Structure<u32>>> {
        Ok(self.structure)
    }
    fn settlement(&self, _: u32) -> attachment::Result<Option<Settlement<u32>>> {
        Ok(self.settlement)
    }
    fn is_attached_virtual(&mut self, actor: u32) -> attachment::Result<bool> {
        match self.virtual_kind.as_str() {
            "ordinary" => attachment::is_attached(actor, self),
            "true" => {
                if self.replace {
                    self.structure = Some(Structure {
                        owner: 1,
                        parent: Some(3),
                    });
                }
                if self.remove {
                    self.structure = None;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
#[test]
fn original_attachment_graph_and_post_virtual_parent_read_match() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actors/native-attachment.json"
    ))
    .unwrap();
    for row in corpus["cases"].as_array().unwrap() {
        let i = &row["input"];
        let output = &row["output"];
        let mut world = Fixture {
            structure: i["structure"].as_bool().unwrap().then_some(Structure {
                owner: 1,
                parent: i["parent"].as_bool().unwrap().then_some(2),
            }),
            settlement: Some(Settlement {
                attached_actor: Some(if i["matches"].as_bool().unwrap() {
                    1
                } else {
                    2
                }),
            }),
            virtual_kind: i["virtual"].as_str().unwrap().into(),
            replace: i["replace"].as_bool().unwrap(),
            remove: false,
        };
        assert_eq!(
            attachment::is_attached(1, &world).unwrap(),
            output["attached"].as_bool().unwrap()
        );
        assert_eq!(
            u64::from(attachment::parent(1, &mut world).unwrap().unwrap_or(0)),
            output["parent"].as_u64().unwrap()
        );
    }
}
#[test]
fn native_invalid_pointer_domains_are_errors() {
    let mut world = Fixture {
        structure: Some(Structure {
            owner: 1,
            parent: Some(2),
        }),
        settlement: None,
        virtual_kind: "ordinary".into(),
        replace: false,
        remove: false,
    };
    assert!(attachment::is_attached(1, &world).is_err());
    world.virtual_kind = "true".into();
    world.remove = true;
    assert!(attachment::parent(1, &mut world).is_err());
}
