//! ModifierState Add/RemovePropertyModifiers table kinds against
//! research/phase8-combat/native-modifier-tables-fixtures.json.
use aa_body::{
    F32, Result,
    credit::Object,
    modifier_tables::*,
    properties::{ModifierId, ModifierState},
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Default)]
struct Names(BTreeMap<String, u32>);
impl Names {
    fn id(&mut self, label: &Value) -> Option<Object> {
        let label = label.as_str()?;
        let next = self.0.len() as u32 + 1;
        Some(Object(*self.0.entry(label.to_owned()).or_insert(next)))
    }
    fn label(&self, object: Option<Object>) -> Value {
        match object {
            None => json!(0),
            Some(o) => self
                .0
                .iter()
                .find(|(_, v)| **v == o.0)
                .map(|(k, _)| json!(k))
                .unwrap(),
        }
    }
}

struct Existing {
    definition: Object,
    enchantment: bool,
    enchantment_type: Option<Object>,
    source: Option<Object>,
    allegiance: Option<Object>,
}

struct World {
    state: ModifierState,
    names: Names,
    existing: Vec<Existing>,
    allied: Vec<Option<Object>>,
    events: Vec<Value>,
    next_ability: u32,
}
impl TableWorld for World {
    fn modifier_state(&mut self) -> &mut ModifierState {
        &mut self.state
    }
    fn resource_count(&self) -> Result<usize> {
        Ok(3)
    }
    fn refresh_actor(&mut self, _: Object) -> Result<()> {
        self.events.push(json!({"refresh": "actor"}));
        Ok(())
    }
    fn create_reverse_ability(&mut self, _: Object) -> Result<Object> {
        self.next_ability += 1;
        Ok(Object(0x9000 + self.next_ability))
    }
    fn destroy_reverse_ability(&mut self, _: Object) -> Result<()> {
        Ok(())
    }
    fn actor_instances(&mut self, _: Object) -> Result<Vec<ActorInstance>> {
        Ok(self
            .existing
            .iter()
            .map(|e| ActorInstance {
                definition: e.definition,
                enchantment: e.enchantment,
                enchantment_type: e.enchantment_type,
                source: e.source,
                allegiance: e.allegiance,
            })
            .collect())
    }
    fn allied(&mut self, _: Object, allegiance: Option<Object>) -> Result<bool> {
        self.events
            .push(json!({"allied_query": self.names.label(allegiance)}));
        Ok(allegiance.is_some() && self.allied.contains(&allegiance))
    }
    fn remove_property(
        &mut self,
        _: Object,
        definition: Object,
        source: Option<Object>,
    ) -> Result<()> {
        self.events
            .push(json!({"remove": self.names.label(Some(definition)),
            "source": self.names.label(source)}));
        if let Some(i) = self
            .existing
            .iter()
            .position(|e| e.definition == definition && e.source == source)
        {
            self.existing.remove(i);
        }
        Ok(())
    }
}

fn table_modifier(spec: &Value, names: &mut Names) -> TableModifier {
    let bits = |v: &Value| F32(v.as_u64().unwrap() as u32);
    match spec["kind"].as_u64().unwrap() {
        3 => TableModifier::Immunity {
            enchantment_type: names.id(&spec["type"]),
        },
        4 => TableModifier::AbilityEffect {
            ability_type_index: spec["ability_type_index"].as_u64().unwrap() as u32,
            effect: names.id(&spec["effect"]).unwrap(),
        },
        6 => TableModifier::PeriodicDamage {
            damage_type: names.id(&spec["damage_type"]).unwrap(),
            amount: bits(&spec["amount"]),
        },
        7 => TableModifier::PeriodicHeal {
            amount: bits(&spec["amount"]),
        },
        8 => TableModifier::ReverseAbility {
            definition: names.id(&spec["definition"]).unwrap(),
        },
        9 => TableModifier::ResourceProduction {
            values: spec["values"]
                .as_array()
                .unwrap()
                .iter()
                .map(bits)
                .collect(),
        },
        other => panic!("kind {other}"),
    }
}

#[test]
fn modifier_tables_match_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-modifier-tables-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 120);
    for case in cases {
        let mut names = Names::default();
        let existing = case["existing"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| Existing {
                definition: names.id(&e["property"]).unwrap(),
                enchantment: e["enchantment"].as_bool().unwrap(),
                enchantment_type: names.id(&e["type"]),
                source: names.id(&e["source"]),
                allegiance: names.id(&e["allegiance"]),
            })
            .collect();
        let allied = case["allied"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| names.id(a))
            .collect();
        let properties: Vec<(Object, Vec<TableModifier>, InstanceIdentity)> = case["properties"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                let definition = names.id(&p["property"]).unwrap();
                let modifiers = p["modifiers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|m| table_modifier(m, &mut names))
                    .collect();
                let enchantment = p["enchantment"].as_bool().unwrap();
                let identity = InstanceIdentity {
                    source_id: u32::from(p["source"] != json!(0)),
                    allegiance_id: u32::from(enchantment && p["allegiance"] != json!(0)),
                };
                (definition, modifiers, identity)
            })
            .collect();
        let mut world = World {
            state: ModifierState::new(0),
            names,
            existing,
            allied,
            events: vec![],
            next_ability: 0,
        };
        for step in case["steps"].as_array().unwrap() {
            world.events.clear();
            let (definition, modifiers, identity) =
                &properties[step["property"].as_u64().unwrap() as usize];
            let actor = step["actor"].as_bool().unwrap().then_some(Object(0x77));
            for (index, modifier) in modifiers.iter().enumerate() {
                let id = ModifierId {
                    definition: *definition,
                    index: index as u32,
                };
                if step["op"] == "add" {
                    add_modifier(
                        id,
                        modifier,
                        *identity,
                        actor,
                        &mut world,
                        Precision::Double53,
                    )
                    .unwrap();
                } else {
                    remove_modifier(id, modifier, actor, &mut world, Precision::Double53).unwrap();
                }
            }
            assert_eq!(Value::Array(world.events.clone()), step["events"], "{step}");
            let native = &step["state"];
            let label = |o: Option<Object>| world.names.label(o);
            let modifier_label = |m: ModifierId| {
                json!(format!(
                    "{}_mod{}",
                    label(Some(m.definition)).as_str().unwrap(),
                    m.index
                ))
            };
            let immunity: Vec<Value> = world
                .state
                .immunity_types
                .iter()
                .map(|t| label(*t))
                .collect();
            assert_eq!(Value::Array(immunity), native["immunity"], "{step}");
            let production: Vec<Value> =
                world.state.production.iter().map(|f| json!(f.0)).collect();
            assert_eq!(Value::Array(production), native["production"], "{step}");
            let lists: Vec<Value> = world
                .state
                .ability_effects
                .iter()
                .map(|l| Value::Array(l.iter().map(|e| label(Some(*e))).collect()))
                .collect();
            assert_eq!(Value::Array(lists), native["ability_effects"], "{step}");
            let damage: Vec<Value> = world
                .state
                .periodic_damage
                .iter()
                .map(|r| {
                    json!([
                        modifier_label(r.modifier),
                        r.source_id != 0,
                        r.allegiance_id != 0
                    ])
                })
                .collect();
            let native_damage: Vec<Value> = native["periodic_damage"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| json!([r[0], r[1] != json!(0), r[2] != json!(0)]))
                .collect();
            assert_eq!(damage, native_damage, "{step}");
            let heals: Vec<Value> = world
                .state
                .periodic_heals
                .iter()
                .map(|h| modifier_label(h.modifier))
                .collect();
            assert_eq!(Value::Array(heals), native["periodic_heals"], "{step}");
            let reverse: Vec<Value> = world
                .state
                .reverse_abilities
                .iter()
                .map(|a| label(Some(a.definition)))
                .collect();
            let native_reverse: Vec<Value> = native["reverse_abilities"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| {
                    // 4C86D0: no local definition, main power from 4BE3F0(0).
                    assert_eq!(
                        (a[1].as_u64(), a[2].as_u64()),
                        (Some(7.5f32.to_bits() as u64), Some(0))
                    );
                    a[0].clone()
                })
                .collect();
            assert_eq!(reverse, native_reverse, "{step}");
            let remaining: Vec<Value> = world
                .existing
                .iter()
                .map(|e| label(Some(e.definition)))
                .collect();
            assert_eq!(Value::Array(remaining), step["actor_properties"], "{step}");
        }
    }
}
