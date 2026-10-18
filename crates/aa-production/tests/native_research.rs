//! Replays research/phase8-economy/native_research.py: original research
//! request/affordability/blocked/completion code under Unicorn, together with
//! the phase6-accepted structure queue, at PC24/PC53/PC64.
use aa_economy::tick::Statistics;
use aa_economy::{Catalog, Economy, F32Bits, Limits, ResourceKind, State, X87Precision};
use aa_numeric::{Binary, Precision};
use aa_production::research::*;
use aa_production::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Deserialize, Serialize)]
struct Definition {
    organization: bool,
    recruit_time_bits: u32,
    research_duration_bits: u32,
    recruit_rate_bits: u32,
    #[serde(default)]
    default_layout: u32,
}
#[derive(Clone, Deserialize, Serialize)]
struct WorldInput {
    now_bits: u32,
    delta_bits: u32,
    instant_production: bool,
    disabled: bool,
    requires_supply: bool,
    in_supply: bool,
    can_reserve_upkeep: bool,
    fulfill_success: bool,
    kingdom: bool,
    definitions: BTreeMap<u32, Definition>,
    next_item_id: u32,
    recruitability: i32,
    cost_quotes: Vec<Vec<u32>>,
    quoted_upkeep: Vec<u32>,
    organization_manager: bool,
    organization_allowed: bool,
    detached_slot: u32,
}
#[derive(Clone, Deserialize)]
struct ResearchInput {
    items: Vec<ResearchItem>,
    state_active: bool,
    organization: bool,
    organization_properties: bool,
    organization_has_property: bool,
    kingdom_has_property: bool,
    requirements: i32,
    completed: Vec<u32>,
    #[serde(default)]
    kingdom_in_progress: Vec<Reservation>,
    statistics: bool,
    techs_researched: u32,
}
#[derive(Deserialize)]
struct Input {
    name: String,
    queue: Queue,
    world: WorldInput,
    catalog: Catalog,
    economy: State,
    limits: Limits,
    research: ResearchInput,
}

const ORG: u32 = 0x0e0e;
const ORG_PROPERTIES: u32 = 0x0e78;

struct TestWorld {
    input: WorldInput,
    research: ResearchInput,
    economy: Economy,
    component: ResearchComponent,
    kingdom_research: KingdomResearch,
    statistics: Statistics,
    precision: Precision,
    events: Vec<Value>,
    fail_after: Option<&'static str>,
}
impl TestWorld {
    fn event(&mut self, name: &str, fields: Value) {
        let mut event = json!({"name": name});
        event
            .as_object_mut()
            .unwrap()
            .extend(fields.as_object().unwrap().clone());
        self.events.push(event);
    }
    fn fail_after(&self, name: &'static str) -> Result<()> {
        if self.fail_after == Some(name) {
            Err(Error::World(format!("injected after {name}")))
        } else {
            Ok(())
        }
    }
    fn item(&self, id: u32) -> ResearchItem {
        self.research
            .items
            .iter()
            .find(|item| item.id == id)
            .cloned()
            .expect("research record")
    }
    fn words(values: &[u32]) -> Vec<F32Bits> {
        values.iter().copied().map(F32Bits).collect()
    }
}

impl World for TestWorld {
    fn actor_state(&mut self, _: &mut Queue, state: u32, enabled: bool) -> Result<()> {
        self.event("actor_state", json!({"state": state, "enabled": enabled}));
        Ok(())
    }
    fn cancel_schedule(&mut self, queue: &mut Queue) -> Result<()> {
        self.event("cancel_schedule", json!({}));
        queue.scheduled = false;
        Ok(())
    }
    fn schedule_zero(&mut self, queue: &mut Queue) -> Result<()> {
        self.event("schedule_zero", json!({"delay": 0}));
        queue.scheduled = true;
        Ok(())
    }
    fn organization_definition(&mut self, id: u32) -> Result<bool> {
        Ok(self.input.definitions[&id].organization)
    }
    fn organization_recruit_time(&mut self, id: u32) -> Result<u32> {
        Ok(self.input.definitions[&id].recruit_time_bits)
    }
    fn research_duration(&mut self, id: u32) -> Result<u32> {
        Ok(self.input.definitions[&id].research_duration_bits)
    }
    fn recruit_rate(&mut self, id: u32) -> Result<Binary> {
        let bits = self.input.definitions[&id].recruit_rate_bits;
        self.event("recruit_rate", json!({"definition": id, "value": bits}));
        Ok(Binary::from_f32_bits(bits)?)
    }
    fn limited_upkeep(&mut self, item: &QueueItem) -> Result<Vec<u32>> {
        Ok(item
            .requested_upkeep
            .iter()
            .zip(&self.economy.catalog().resources)
            .map(|(&value, resource)| {
                if resource.kind == ResourceKind::LimitedConsumed {
                    value
                } else {
                    0
                }
            })
            .collect())
    }
    fn can_reserve_upkeep(&mut self, _: &mut Queue, _: &QueueItem, notify: bool) -> Result<bool> {
        self.event("can_reserve_upkeep", json!({"notify": notify}));
        Ok(self.input.can_reserve_upkeep)
    }
    fn adjust_upkeep(&mut self, _: &mut Queue, values: &[u32], add: bool) -> Result<()> {
        if self.input.kingdom {
            self.event("adjust_upkeep", json!({"values": values, "add": add}));
            self.economy.adjust_consumption(&Self::words(values), add)?;
        }
        Ok(())
    }
    fn actor_disabled(&mut self, _: &mut Queue) -> Result<bool> {
        Ok(self.input.disabled)
    }
    fn actor_requires_supply(&mut self, _: &mut Queue) -> Result<bool> {
        self.event("requires_supply", json!({}));
        Ok(self.input.requires_supply)
    }
    fn actor_in_supply(&mut self, _: &mut Queue) -> Result<bool> {
        self.event("in_supply", json!({}));
        Ok(self.input.in_supply)
    }
    fn notify_pause(&mut self, _: &mut Queue, reason: i32) -> Result<()> {
        self.event("notify_pause", json!({"reason": reason}));
        Ok(())
    }
    fn detach_organization_slot(&mut self, _: &mut Queue, item: &QueueItem) -> Result<Option<u32>> {
        if self.input.organization_manager {
            self.event("detach_organization_slot", json!({"id": item.id}));
            Ok(Some(self.input.detached_slot))
        } else {
            Ok(None)
        }
    }
    fn fulfill(&mut self, _: &mut Queue, item: &QueueItem) -> Result<bool> {
        assert_eq!(
            item.kind, 1,
            "only research items are queued in this corpus"
        );
        // 0x589F90 -> 0x584120 on actor+0xB4 for research items.
        let record = self.item(item.property);
        let mut component = std::mem::take(&mut self.component);
        let precision = self.precision;
        let result = complete_research(&mut component, &record, precision, self);
        self.component = component;
        result
    }
    fn refund(&mut self, _: &mut Queue, item: &QueueItem) -> Result<()> {
        assert_eq!(
            item.kind, 1,
            "only research items are queued in this corpus"
        );
        self.event("refund", json!({"kind": item.kind}));
        let record = self.item(item.property);
        refund_research(&record, &item.paid_cost, self)
    }
    fn release_item(&mut self, _: &mut Queue, item: &QueueItem) -> Result<()> {
        self.event(
            "release_item",
            json!({"id": item.id, "organization_slot": item.organization_slot}),
        );
        Ok(())
    }
}

impl Research for TestWorld {
    fn research_kingdom(&mut self) -> Result<Option<u32>> {
        self.event("kingdom", json!({}));
        Ok(self.input.kingdom.then_some(1))
    }
    fn research_state_active(&mut self, state: u32) -> Result<bool> {
        self.event("research_state_active", json!({"state": state}));
        Ok(self.research.state_active)
    }
    fn organization(&mut self) -> Result<Option<Organization>> {
        self.event("organization", json!({}));
        Ok(self.research.organization.then_some(Organization {
            id: ORG,
            properties: if self.research.organization_properties {
                ORG_PROPERTIES
            } else {
                0
            },
        }))
    }
    fn kingdom_research(&mut self, kingdom: u32) -> Result<&mut KingdomResearch> {
        assert_eq!(kingdom, 1);
        Ok(&mut self.kingdom_research)
    }
    fn organization_has_property(&mut self, properties: u32, definition: u32) -> Result<bool> {
        assert_eq!(properties, ORG_PROPERTIES);
        self.event(
            "organization_has_property",
            json!({"definition": definition}),
        );
        Ok(self.research.organization_has_property)
    }
    fn kingdom_has_property(&mut self, kingdom: u32, definition: u32) -> Result<bool> {
        assert_eq!(kingdom, 1);
        self.event("kingdom_has_property", json!({"definition": definition}));
        Ok(self.research.kingdom_has_property)
    }
    fn research_requirements(
        &mut self,
        kingdom: Option<u32>,
        organization: Option<Organization>,
        item: &ResearchItem,
        notify: bool,
    ) -> Result<i32> {
        self.event(
            "research_requirements",
            json!({"kingdom": kingdom.is_some(), "organization": organization.is_some(),
                   "item": item.id, "notify": notify}),
        );
        Ok(self.research.requirements)
    }
    fn research_can_afford(&mut self, kingdom: u32, cost: &[u32]) -> Result<bool> {
        assert_eq!(kingdom, 1);
        Ok(self.economy.can_afford(&Self::words(cost))?)
    }
    fn research_debit(&mut self, kingdom: u32, cost: &[u32]) -> Result<()> {
        assert_eq!(kingdom, 1);
        self.event("debit", json!({"values": cost}));
        self.economy.debit_stockpiles(&Self::words(cost))?;
        self.fail_after("debit")
    }
    fn research_credit(&mut self, kingdom: u32, cost: &[u32]) -> Result<()> {
        assert_eq!(kingdom, 1);
        self.economy.credit_stockpiles(&Self::words(cost))?;
        Ok(())
    }
    fn notify_research_blocked(&mut self, _: &ResearchItem, kingdom: Option<u32>) -> Result<()> {
        self.event(
            "notify_research_blocked",
            json!({"kingdom": kingdom.is_some()}),
        );
        Ok(())
    }
    fn notify_research_unaffordable(
        &mut self,
        _: &ResearchItem,
        _: &[u32],
        kingdom: Option<u32>,
    ) -> Result<()> {
        self.event(
            "notify_research_unaffordable",
            json!({"kingdom": kingdom.is_some()}),
        );
        Ok(())
    }
    fn next_research_item_id(&mut self) -> Result<u32> {
        self.input.next_item_id = self.input.next_item_id.wrapping_add(1);
        Ok(self.input.next_item_id)
    }
    fn statistics(&mut self, kingdom: u32) -> Result<Option<&mut Statistics>> {
        assert_eq!(kingdom, 1);
        Ok(self.research.statistics.then_some(&mut self.statistics))
    }
    fn research_complete_event(&mut self, kingdom: Option<u32>, item: &ResearchItem) -> Result<()> {
        self.event(
            "research_complete_event",
            json!({"kingdom": kingdom.is_some(), "property": item.definition}),
        );
        Ok(())
    }
    fn apply_property(&mut self, target: PropertyTarget, definition: u32) -> Result<()> {
        let fields = match target {
            PropertyTarget::Actor => {
                json!({"target": "actor", "definition": definition, "source": 2, "actor": false})
            }
            PropertyTarget::Organization(org) => {
                assert_eq!(org, ORG);
                json!({"target": "organization", "definition": definition, "source": 8, "actor": true})
            }
        };
        self.event("apply_property", fields);
        Ok(())
    }
    fn kingdom_add_property(&mut self, kingdom: u32, definition: u32) -> Result<()> {
        assert_eq!(kingdom, 1);
        self.event(
            "kingdom_add_property",
            json!({"definition": definition, "source": 2, "third": 0, "notify": 0}),
        );
        Ok(())
    }
    fn when_researched(&mut self, item: &ResearchItem) -> Result<()> {
        self.event("when_researched", json!({"definition": item.definition}));
        Ok(())
    }
}

fn document() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase8-economy/native-research-fixtures.json"
    ))
    .unwrap()
}

fn build(case: &Value) -> (Input, Queue, TestWorld, Precision) {
    let input: Input = serde_json::from_value(case["input"].clone()).unwrap();
    let pc: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
    let econ_pc: X87Precision = serde_json::from_value(case["precision"].clone()).unwrap();
    let queue = input.queue.clone();
    let mut scores = vec![F32Bits::ZERO; 62];
    scores[2] = F32Bits(input.research.techs_researched);
    let world = TestWorld {
        input: input.world.clone(),
        research: input.research.clone(),
        economy: Economy::new(
            input.catalog.clone(),
            input.economy.clone(),
            input.limits,
            econ_pc,
        )
        .unwrap(),
        component: ResearchComponent {
            completed: input.research.completed.clone(),
        },
        kingdom_research: KingdomResearch {
            in_progress: input.research.kingdom_in_progress.clone(),
        },
        statistics: Statistics {
            production: vec![],
            scores,
        },
        precision: pc,
        events: vec![],
        fail_after: None,
    };
    (input, queue, world, pc)
}

#[test]
fn original_research_paths_match_every_state_resource_and_event() {
    let document = document();
    let cases = document["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 147);
    let mut steps = 0;
    for case in cases {
        let (input, mut queue, mut world, pc) = build(case);
        for row in case["steps"].as_array().unwrap() {
            let action = &row["input"];
            if let Some(update) = action.get("world") {
                let mut value = serde_json::to_value(&world.input).unwrap();
                value
                    .as_object_mut()
                    .unwrap()
                    .extend(update.as_object().unwrap().clone());
                world.input = serde_json::from_value(value).unwrap();
            }
            let context = Context {
                now_bits: world.input.now_bits,
                delta_bits: world.input.delta_bits,
                instant_production: world.input.instant_production,
                precision: pc,
            };
            world.events.clear();
            let label = format!("{} {:?} {}", input.name, pc, action);
            let mut actual = json!({});
            match action["kind"].as_str().unwrap() {
                "start" => start_head(&mut queue, context, &mut world).expect(&label),
                "process" => process(&mut queue, context, &mut world).expect(&label),
                "cancel" => {
                    let index = cancel(
                        &mut queue,
                        action["id"].as_u64().unwrap() as u32,
                        &mut world,
                    )
                    .expect(&label)
                    .map_or(-1, |i| i as i32);
                    actual["index"] = json!(index);
                }
                "research" => {
                    let item = world.item(action["item"].as_u64().unwrap() as u32);
                    let component = world.component.clone();
                    let accepted =
                        request_research(&component, &mut queue, &item, context, &mut world)
                            .expect(&label);
                    actual["result"] = json!(accepted);
                }
                "blocked" => {
                    let item = world.item(action["item"].as_u64().unwrap() as u32);
                    let component = world.component.clone();
                    let notify = action["notify"].as_bool().unwrap_or(false);
                    let reason = research_blocked(&component, &queue, &item, notify, &mut world)
                        .expect(&label);
                    actual["result"] = json!(reason as u32);
                }
                "afford" => {
                    let item = world.item(action["item"].as_u64().unwrap() as u32);
                    let notify = action["notify"].as_bool().unwrap_or(false);
                    actual["result"] =
                        json!(research_affordable(&item, notify, &mut world).expect(&label));
                }
                "complete" => {
                    let item = world.item(action["item"].as_u64().unwrap() as u32);
                    let mut component = std::mem::take(&mut world.component);
                    let done =
                        complete_research(&mut component, &item, pc, &mut world).expect(&label);
                    world.component = component;
                    actual["result"] = json!(done);
                }
                other => panic!("unexpected action {other}"),
            }
            actual["queue"] = json!(queue);
            actual["economy"] = json!(world.economy.state());
            actual["events"] = json!(world.events);
            actual["next_item_id"] = json!(world.input.next_item_id);
            actual["research"] = json!({
                "completed": world.component.completed,
                "kingdom_in_progress": world.kingdom_research.in_progress,
                "techs_researched": world.statistics.scores[2],
            });
            assert_eq!(actual, row["output"], "{label}");
            steps += 1;
        }
    }
    assert_eq!(steps, 561);
}

#[test]
fn debit_failure_stops_before_queueing_and_reservation() {
    let document = document();
    let case = document["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["input"]["name"] == "fifo_and_reservations" && c["precision"] == "Double53")
        .unwrap();
    let (_, mut queue, mut world, pc) = build(case);
    world.fail_after = Some("debit");
    let item = world.item(4);
    let component = world.component.clone();
    let context = Context {
        now_bits: world.input.now_bits,
        delta_bits: world.input.delta_bits,
        instant_production: false,
        precision: pc,
    };
    assert!(request_research(&component, &mut queue, &item, context, &mut world).is_err());
    assert!(queue.items.is_empty());
    assert!(world.kingdom_research.in_progress.is_empty());
    assert_eq!(world.input.next_item_id, 100);
    assert_eq!(
        world.economy.state().stockpiled[0],
        F32Bits(42.75f32.to_bits())
    );
}

#[test]
fn kingdom_level_completion_without_owner_is_rejected() {
    let document = document();
    let case = document["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| {
            c["input"]["name"] == "complete_without_kingdom_actor_level"
                && c["precision"] == "Double53"
        })
        .unwrap();
    let (_, _, mut world, pc) = build(case);
    let item = world.item(4);
    let mut component = std::mem::take(&mut world.component);
    assert!(matches!(
        complete_research(&mut component, &item, pc, &mut world),
        Err(Error::Invalid(_))
    ));
}
