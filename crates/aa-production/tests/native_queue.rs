use aa_economy::{Catalog, Economy, F32Bits, Limits, ResourceKind, State, X87Precision};
use aa_numeric::{Binary, Precision};
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
#[derive(Deserialize)]
struct Input {
    name: String,
    queue: Queue,
    world: WorldInput,
    catalog: Catalog,
    economy: State,
    limits: Limits,
}
struct DiagnosticWorld {
    input: WorldInput,
    economy: Economy,
    events: Vec<Value>,
    quote_index: usize,
    fail_after: Option<&'static str>,
}

impl Recruitment for DiagnosticWorld {
    fn kingdom(&mut self) -> Result<Option<u32>> {
        Ok(self.input.kingdom.then_some(1))
    }
    fn can_enroll_organization(&mut self, definition: u32) -> Result<bool> {
        if self.input.organization_manager {
            self.event("can_enroll_organization", json!({"definition":definition}));
            Ok(self.input.organization_allowed)
        } else {
            Ok(true)
        }
    }
    fn default_layout(&mut self, id: u32) -> Result<u32> {
        Ok(self.input.definitions[&id].default_layout)
    }
    fn recruitability(&mut self, definition: u32, layout: u32) -> Result<i32> {
        self.event(
            "recruitability",
            json!({"definition":definition,"layout":layout}),
        );
        Ok(self.input.recruitability)
    }
    fn quote_cost(&mut self, _: u32, _: u32) -> Result<Vec<u32>> {
        let values =
            self.input.cost_quotes[self.quote_index.min(self.input.cost_quotes.len() - 1)].clone();
        self.quote_index += 1;
        self.event("quote_cost", json!({"values":values}));
        Ok(values)
    }
    fn can_afford(&mut self, kingdom: u32, cost: &[u32]) -> Result<bool> {
        assert_eq!(kingdom, 1);
        Ok(self
            .economy
            .can_afford(&cost.iter().copied().map(F32Bits).collect::<Vec<_>>())?)
    }
    fn notify_unaffordable(&mut self) -> Result<()> {
        self.event("notify_unaffordable", json!({}));
        Ok(())
    }
    fn debit(&mut self, kingdom: u32, cost: &[u32]) -> Result<()> {
        assert_eq!(kingdom, 1);
        self.event("debit", json!({"values":cost}));
        self.economy
            .debit_stockpiles(&cost.iter().copied().map(F32Bits).collect::<Vec<_>>())?;
        self.fail_after("debit")
    }
    fn reserve_recruit(&mut self, kingdom: u32, definition: u32, layout: u32) -> Result<()> {
        assert_eq!(kingdom, 1);
        self.event(
            "reserve_recruit",
            json!({"definition":definition,"layout":layout}),
        );
        Ok(())
    }
    fn quote_upkeep(&mut self, _: u32, _: u32) -> Result<Vec<u32>> {
        let values = self.input.quoted_upkeep.clone();
        self.event("quote_upkeep", json!({"values":values}));
        Ok(values)
    }
    fn next_item_id(&mut self) -> Result<u32> {
        self.input.next_item_id = self.input.next_item_id.wrapping_add(1);
        Ok(self.input.next_item_id)
    }
    fn register_organization_item(&mut self, _: &mut Queue, id: u32) -> Result<()> {
        if self.input.organization_manager {
            self.event("register_organization_item", json!({"id":id}));
        }
        Ok(())
    }
}
impl DiagnosticWorld {
    fn event(&mut self, name: &str, fields: Value) {
        let mut event = json!({"name":name});
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
}
impl World for DiagnosticWorld {
    fn actor_state(&mut self, _: &mut Queue, state: u32, enabled: bool) -> Result<()> {
        self.event("actor_state", json!({"state":state,"enabled":enabled}));
        Ok(())
    }
    fn cancel_schedule(&mut self, queue: &mut Queue) -> Result<()> {
        self.event("cancel_schedule", json!({}));
        queue.scheduled = false;
        Ok(())
    }
    fn schedule_zero(&mut self, queue: &mut Queue) -> Result<()> {
        self.event("schedule_zero", json!({"delay":0}));
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
        self.event("recruit_rate", json!({"definition":id,"value":bits}));
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
        self.event("can_reserve_upkeep", json!({"notify":notify}));
        Ok(self.input.can_reserve_upkeep)
    }
    fn adjust_upkeep(&mut self, _: &mut Queue, values: &[u32], add: bool) -> Result<()> {
        if self.input.kingdom {
            self.event("adjust_upkeep", json!({"values":values,"add":add}));
            self.economy.adjust_consumption(
                &values.iter().copied().map(F32Bits).collect::<Vec<_>>(),
                add,
            )?;
        }
        self.fail_after("adjust_upkeep")
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
        self.event("notify_pause", json!({"reason":reason}));
        Ok(())
    }
    fn detach_organization_slot(&mut self, _: &mut Queue, item: &QueueItem) -> Result<Option<u32>> {
        if self.input.organization_manager {
            self.event("detach_organization_slot", json!({"id":item.id}));
            Ok(Some(self.input.detached_slot))
        } else {
            Ok(None)
        }
    }
    fn fulfill(&mut self, _: &mut Queue, item: &QueueItem) -> Result<bool> {
        let mut fields = json!({"kind":item.kind,"success":self.input.fulfill_success});
        if matches!(item.kind, 0 | 2) {
            fields["organization_slot"] = json!(item.organization_slot);
        }
        self.event("fulfill", fields);
        Ok(self.input.fulfill_success)
    }
    fn refund(&mut self, _: &mut Queue, item: &QueueItem) -> Result<()> {
        self.event("refund", json!({"kind":item.kind}));
        if self.input.kingdom {
            self.economy.credit_stockpiles(
                &item
                    .paid_cost
                    .iter()
                    .copied()
                    .map(F32Bits)
                    .collect::<Vec<_>>(),
            )?;
        }
        Ok(())
    }
    fn release_item(&mut self, _: &mut Queue, item: &QueueItem) -> Result<()> {
        self.event(
            "release_item",
            json!({"id":item.id,"organization_slot":item.organization_slot}),
        );
        Ok(())
    }
}

#[test]
fn original_queue_sequences_match_every_state_resource_and_event() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-production/native-production-fixtures.json"
    ))
    .unwrap();
    let cases = document["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 231);
    for case in cases {
        let input: Input = serde_json::from_value(case["input"].clone()).unwrap();
        let pc: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let econ_pc: X87Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let mut queue = input.queue;
        let mut world = DiagnosticWorld {
            input: input.world,
            economy: Economy::new(input.catalog, input.economy, input.limits, econ_pc).unwrap(),
            events: vec![],
            quote_index: 0,
            fail_after: None,
        };
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
            let mut accepted = None;
            let index = match action["kind"].as_str().unwrap() {
                "start" => {
                    start_head(&mut queue, context, &mut world).unwrap();
                    None
                }
                "process" => {
                    process(&mut queue, context, &mut world).unwrap();
                    None
                }
                "cancel" => Some(
                    cancel(
                        &mut queue,
                        action["id"].as_u64().unwrap() as u32,
                        &mut world,
                    )
                    .unwrap()
                    .map_or(-1, |i| i as i32),
                ),
                "request" => {
                    world.quote_index = 0;
                    accepted = Some(
                        request_recruit(
                            &mut queue,
                            serde_json::from_value(action["request"].clone()).unwrap(),
                            context,
                            &mut world,
                        )
                        .unwrap(),
                    );
                    None
                }
                _ => unreachable!(),
            };
            let mut actual = json!({"queue":queue,"economy":world.economy.state(),"events":world.events,"next_item_id":world.input.next_item_id});
            if let Some(index) = index {
                actual["index"] = json!(index);
            }
            if let Some(accepted) = accepted {
                actual["accepted"] = json!(accepted);
            }
            assert_eq!(actual, row["output"], "{} {:?} {}", input.name, pc, action);
        }
    }
}

fn diagnostic(name: &str) -> (Queue, DiagnosticWorld, Context) {
    let document: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-production/native-production-fixtures.json"
    ))
    .unwrap();
    let case = document["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["input"]["name"] == name && case["precision"] == "Double53")
        .unwrap();
    let input: Input = serde_json::from_value(case["input"].clone()).unwrap();
    let context = Context {
        now_bits: input.world.now_bits,
        delta_bits: input.world.delta_bits,
        instant_production: input.world.instant_production,
        precision: Precision::Double53,
    };
    (
        input.queue,
        DiagnosticWorld {
            input: input.world,
            economy: Economy::new(
                input.catalog,
                input.economy,
                input.limits,
                X87Precision::Double53,
            )
            .unwrap(),
            events: vec![],
            quote_index: 0,
            fail_after: None,
        },
        context,
    )
}

#[test]
fn payment_failure_stops_before_reservation_without_inventing_rollback() {
    let (mut queue, mut world, context) = diagnostic("request_complete");
    world.fail_after = Some("debit");
    let request = RecruitRequest {
        actor_definition: 1,
        organization_layout: 0,
        location_bits: None,
    };
    assert!(request_recruit(&mut queue, request, context, &mut world).is_err());
    assert!(queue.items.is_empty());
    assert_eq!(
        world.economy.state().stockpiled[0],
        F32Bits(40f32.to_bits())
    );
    assert_eq!(world.input.next_item_id, 100);
    assert_eq!(world.events.last().unwrap()["name"], "debit");
    assert!(
        !world
            .events
            .iter()
            .any(|event| event["name"] == "reserve_recruit")
    );
}

#[test]
fn removal_failure_never_fulfills_or_refunds_a_partially_removed_item() {
    let (mut queue, mut world, context) = diagnostic("recruit_complete");
    start_head(&mut queue, context, &mut world).unwrap();
    world.events.clear();
    world.fail_after = Some("adjust_upkeep");
    assert!(
        process(
            &mut queue,
            Context {
                now_bits: 14f32.to_bits(),
                ..context
            },
            &mut world
        )
        .is_err()
    );
    assert!(queue.items.is_empty());
    assert!(!queue.scheduled);
    assert_eq!(world.economy.state().consumed[2], F32Bits(0));
    assert_eq!(
        world.economy.state().stockpiled[0],
        F32Bits(50f32.to_bits())
    );
    assert_eq!(world.events.last().unwrap()["name"], "adjust_upkeep");
    assert!(
        !world
            .events
            .iter()
            .any(|event| event["name"] == "fulfill" || event["name"] == "refund")
    );
}

#[test]
fn nonfinite_clock_is_rejected_before_processing_effects() {
    let (mut queue, mut world, context) = diagnostic("recruit_complete");
    let original = queue.clone();
    assert!(
        process(
            &mut queue,
            Context {
                now_bits: f32::NAN.to_bits(),
                ..context
            },
            &mut world
        )
        .is_err()
    );
    assert_eq!(queue, original);
    assert!(world.events.is_empty());
}
