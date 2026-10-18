//! 0x45E090 / 0x4BBF90 against research/phase8-combat/native-combat-value-fixtures.json
//! (Unicorn, original code including the native CRT pow; all x87 precisions).
//! Modifier getters bind the aa-body ModifierState ports (501A30/501AA0,
//! 501560, 501D50/501DB0).
use aa_body::{F32, properties::ModifierState};
use aa_combat::{
    Error, Result,
    combat_value::{
        self, AbilityDefinition, AbilityLocal, Extent, MeanValues, Settings, ThingInput,
        UnitInputs, World,
    },
};
use aa_numeric::{Binary, Precision, crt_pow::CrtEnvironment};
use serde_json::Value;

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn f(v: &Value) -> F32 {
    F32(u(v))
}
fn num(v: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(v.0)?)
}
fn body(e: aa_body::Error) -> Error {
    Error::World(e.to_string())
}

#[derive(Clone)]
struct Adapter {
    modifier: Option<ModifierState>,
    clamp: F32,
    svars: Value,
    log: bool,
    p: Precision,
}
impl Adapter {
    fn state(&self) -> Result<&ModifierState> {
        self.modifier.as_ref().ok_or(Error::Invalid("no modifier"))
    }
}
impl World for Adapter {
    fn ranged_minimum(&mut self) -> Result<F32> {
        Ok(f(&self.svars["CombatValueRangedMinimum"]))
    }
    fn bind_defense_weight_ranged(&mut self) -> Result<()> {
        self.svars
            .get("CombatValueDefenseWeightRanged")
            .ok_or(Error::Invalid(
                "missing native ranged defense weight source identity",
            ))?;
        Ok(())
    }
    fn defense_weight_ranged(&mut self) -> Result<F32> {
        Ok(f(&self.svars["CombatValueDefenseWeightRanged"]))
    }
    fn modify_projectile_range(&mut self, _: u32, base: F32) -> Result<Binary> {
        num(self
            .state()?
            .projectile_range(base, self.clamp, self.p)
            .map_err(body)?)
    }
    fn modify_bombard_range(&mut self, _: u32, base: F32) -> Result<Binary> {
        num(self
            .state()?
            .bombard_range(base, self.clamp, self.p)
            .map_err(body)?)
    }
    fn modify_power(&mut self, _: u32, base: F32, ranged: bool) -> Result<Binary> {
        num(self
            .state()?
            .power(ranged, base, self.clamp, self.p)
            .map_err(body)?)
    }
    fn modify_recharge(&mut self, _: u32, base: F32) -> Result<Binary> {
        self.state()?.recharge(base, self.p).map_err(body)
    }
    fn log_enabled(&mut self) -> Result<bool> {
        Ok(self.log)
    }
}

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-combat-value-fixtures.json"
    ))
    .unwrap()
}

fn local(definition: AbilityDefinition, v: &Value) -> AbilityLocal {
    AbilityLocal {
        definition,
        value_04: f(&v["value_04"]),
        recharge_18: f(&v["recharge_18"]),
        first_entry_10: v["first_entry_10"].as_u64().map(|b| F32(b as u32)),
    }
}

#[test]
fn ability_value_matches_native() {
    let doc = fixture();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 480);
    let (mut logged, mut modified) = (0, 0);
    for case in cases {
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let d = &case["definition"];
        let definition = AbilityDefinition {
            flags_54: u(&d["flags_54"]),
            range_24: f(&d["range_24"]),
            area_38: f(&d["area_38"]),
            projectile_3c: d["projectile_3c"].as_bool().unwrap(),
            power_64: f(&d["power_64"]),
        };
        let local = local(definition, &case["local"]);
        let un = &case["unit"];
        let unit = UnitInputs {
            defense: f(&un["defense"]),
            health: f(&un["health"]),
            speed: f(&un["speed"]),
            additive_a: f(&un["additive_a"]),
            additive_b: f(&un["additive_b"]),
        };
        let m = &case["means"];
        let means = MeanValues {
            maximum_74: F32::ZERO,
            attack: f(&m["attack"]),
            attack_rate: F32::ZERO,
            defense: f(&m["defense"]),
            movement: f(&m["movement"]),
            health: F32::ZERO,
            lifetime: f(&m["lifetime"]),
            offense: f(&m["offense"]),
        };
        let s = &case["settings"];
        let settings = Settings {
            damage_minimum: f(&s["damage_minimum"]),
            range_power: f(&s["range_power"]),
            area_power: f(&s["area_power"]),
            offense_weight: f(&s["offense_weight"]),
            defense_weight: f(&s["defense_weight"]),
            movement_weight: f(&s["movement_weight"]),
            movement_default: f(&s["movement_default"]),
        };
        let modifier = case["modifier"].as_object().map(|scalars| {
            let mut state = ModifierState::new(1);
            for (offset, bits) in scalars {
                let offset = u32::from_str_radix(offset.trim_start_matches("0x"), 16).unwrap();
                state.scalars[(offset / 4 - 1) as usize] = f(bits);
            }
            state
        });
        modified += usize::from(modifier.is_some());
        let mut world = Adapter {
            modifier,
            clamp: f(&s["clamp"]),
            svars: case["svars"].clone(),
            log: case["log_enabled"].as_bool().unwrap(),
            p,
        };
        let id = world.modifier.as_ref().map(|_| 1);
        let mut lazy_world=world.clone();
        let out = combat_value::ability_value(
            &local,
            &unit,
            id,
            &means,
            &settings,
            CrtEnvironment::SSE2_DOUBLE53,
            &mut world,
            p,
        )
        .unwrap();
        assert_eq!(out.value, f(&case["result"]), "{case}");
        let mut samples=0;
        let lazy=combat_value::ability_value_with_environment(&local,&unit,id,&means,&settings,|| {
            samples+=1;Ok(CrtEnvironment::SSE2_DOUBLE53)
        },&mut lazy_world,p).unwrap();
        assert_eq!(lazy,out,"lazy environment changed native result {case}");
        let expected=if definition.flags_54&0x80==0 {
            usize::from(f32::from_bits(definition.power_64.0)>0.0)*2+usize::from(world.log)*2
        }else{0};
        assert_eq!(samples,expected,"CRT sample branch/count {case}");
        let csv: Vec<&Value> = case["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e.get("csv").is_some())
            .collect();
        match out.log {
            None => assert!(csv.is_empty(), "{case}"),
            Some(row) => {
                logged += 1;
                assert_eq!(csv.len(), 1);
                assert_eq!(csv[0]["stream"], 0x4c);
                let native: Vec<u64> = csv[0]["csv"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
                    .collect();
                assert_eq!(row.to_vec(), native, "{case}");
            }
        }
    }
    assert!(logged > 100 && modified > 150, "{logged} {modified}");
}

#[test]
fn mean_values_match_native() {
    let doc = fixture();
    let cases = doc["mean_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 108);
    let (mut asserted, mut empty) = (0, 0);
    for case in cases {
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let things: Vec<ThingInput> = case["things"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| {
                let e = &t["extent"];
                ThingInput {
                    flags_174: u(&t["flags_174"]),
                    extent: Extent {
                        count_468: e["count_468"].as_i64().unwrap() as i32,
                        base_478: f(&e["base_478"]),
                        nested_3c: f(&e["nested_3c"]),
                    },
                    abilities: t["abilities"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|a| {
                            local(
                                AbilityDefinition {
                                    flags_54: 0,
                                    range_24: F32::ZERO,
                                    area_38: F32::ZERO,
                                    projectile_3c: false,
                                    power_64: f(&a["power_64"]),
                                },
                                a,
                            )
                        })
                        .collect(),
                    defense_288: f(&t["defense_288"]),
                    movement_228: f(&t["movement_228"]),
                    health_284: f(&t["health_284"]),
                }
            })
            .collect();
        let result = combat_value::mean_values(
            &things,
            f(&case["discard"]),
            case["suppressed"].as_bool().unwrap(),
            p,
        );
        let m = &case["means"];
        if u(&m["attack"]) & 0x7fff_ffff > 0x7f80_0000 {
            // Native 1/0 * 0 = NaN means over an empty trimmed range: the port
            // rejects the input explicitly instead of carrying a NaN.
            assert!(matches!(result, Err(Error::Invalid(_))), "{case}");
            empty += 1;
            continue;
        }
        let out = result.unwrap();
        let got = out.means;
        for (name, value) in [
            ("maximum_74", got.maximum_74),
            ("attack", got.attack),
            ("attack_rate", got.attack_rate),
            ("defense", got.defense),
            ("movement", got.movement),
            ("health", got.health),
            ("lifetime", got.lifetime),
            ("offense", got.offense),
        ] {
            assert_eq!(value, f(&m[name]), "{name} {case}");
        }
        let events = case["events"].as_array().unwrap();
        let native_assert = events.iter().any(|e| e.get("assert").is_some());
        assert_eq!(out.assertion, native_assert, "{case}");
        asserted += usize::from(native_assert);
        let logs: Vec<u64> = events
            .iter()
            .filter_map(|e| e.get("value"))
            .map(|v| u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
            .collect();
        let stored = [
            got.attack,
            got.attack_rate,
            got.defense,
            got.movement,
            got.health,
            got.offense,
            got.lifetime,
        ];
        let expected: Vec<u64> = stored
            .iter()
            .map(|v| f64::from(f32::from_bits(v.0)).to_bits())
            .collect();
        assert_eq!(logs, expected, "{case}");
    }
    assert!(asserted > 5 && empty <= 6, "{asserted} {empty}");
}

fn definition(d: &Value) -> AbilityDefinition {
    AbilityDefinition {
        flags_54: u(&d["flags_54"]),
        range_24: f(&d["range_24"]),
        area_38: f(&d["area_38"]),
        projectile_3c: d["projectile_3c"].as_bool().unwrap(),
        power_64: f(&d["power_64"]),
    }
}

#[test]
fn performer_value_matches_native() {
    use aa_combat::combat_value::{PerformerInputs, TableEntry};
    let doc = fixture();
    let cases = doc["performer_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 180);
    let (mut nonzero, mut logs) = (0, 0);
    for case in cases {
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let m = &case["means"];
        let means = MeanValues {
            maximum_74: F32::ZERO,
            attack: f(&m["attack"]),
            attack_rate: F32::ZERO,
            defense: f(&m["defense"]),
            movement: f(&m["movement"]),
            health: F32::ZERO,
            lifetime: f(&m["lifetime"]),
            offense: f(&m["offense"]),
        };
        let s = &case["settings"];
        let settings = Settings {
            damage_minimum: f(&s["damage_minimum"]),
            range_power: f(&s["range_power"]),
            area_power: f(&s["area_power"]),
            offense_weight: f(&s["offense_weight"]),
            defense_weight: f(&s["defense_weight"]),
            movement_weight: f(&s["movement_weight"]),
            movement_default: f(&s["movement_default"]),
        };
        let table = |rows: &Value| -> Vec<TableEntry> {
            rows.as_array()
                .unwrap()
                .iter()
                .map(|r| TableEntry {
                    enabled: r["enabled"].as_bool().unwrap(),
                    local: local(definition(&r["definition"]), &r["local"]),
                })
                .collect()
        };
        let modifier = case["modifier"].as_object().map(|scalars| {
            let mut state = ModifierState::new(1);
            for (offset, bits) in scalars {
                let offset = u32::from_str_radix(offset.trim_start_matches("0x"), 16).unwrap();
                state.scalars[(offset / 4 - 1) as usize] = f(bits);
            }
            state
        });
        let inputs = PerformerInputs {
            property_values: case["property_values"]
                .as_array()
                .map(|v| v.iter().map(f).collect()),
            modifier: modifier.as_ref().map(|_| 1),
            body: case["body"].as_array().map(|b| (f(&b[0]), f(&b[1]))),
            speed: case["speed"].as_u64().map(|b| F32(b as u32)),
            bonus_210: f(&case["bonus_210"]),
            abilities: table(&case["abilities"]),
            secondary: (!case["secondary"].is_null()).then(|| table(&case["secondary"])),
        };
        let mut world = Adapter {
            modifier,
            clamp: f(&s["clamp"]),
            svars: case["svars"].clone(),
            log: case["log_enabled"].as_bool().unwrap(),
            p,
        };
        let out = combat_value::performer_value(
            &inputs,
            &means,
            &settings,
            CrtEnvironment::SSE2_DOUBLE53,
            &mut world,
            p,
        )
        .unwrap();
        assert_eq!(out.value, f(&case["value"]), "{case}");
        nonzero += usize::from(out.value != F32::ZERO);
        let native: Vec<Vec<u64>> = case["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| e.get("csv"))
            .map(|row| {
                row.as_array()
                    .unwrap()
                    .iter()
                    .map(|v| u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
                    .collect()
            })
            .collect();
        let ours: Vec<Vec<u64>> = out.logs.iter().map(|r| r.to_vec()).collect();
        assert_eq!(ours, native, "{case}");
        logs += ours.len();
    }
    assert!(nonzero > 60 && logs > 20, "{nonzero} {logs}");
}

struct Definitions {
    inner: Adapter,
    things: Vec<aa_combat::combat_value::DefinitionThing>,
    fields: std::collections::BTreeMap<(u32, u8), F32>,
}
impl World for Definitions {
    fn ranged_minimum(&mut self) -> Result<F32> {
        self.inner.ranged_minimum()
    }
    fn bind_defense_weight_ranged(&mut self) -> Result<()> {
        self.inner.bind_defense_weight_ranged()
    }
    fn defense_weight_ranged(&mut self) -> Result<F32> {
        self.inner.defense_weight_ranged()
    }
    fn modify_projectile_range(&mut self, m: u32, base: F32) -> Result<Binary> {
        self.inner.modify_projectile_range(m, base)
    }
    fn modify_bombard_range(&mut self, m: u32, base: F32) -> Result<Binary> {
        self.inner.modify_bombard_range(m, base)
    }
    fn modify_power(&mut self, m: u32, base: F32, ranged: bool) -> Result<Binary> {
        self.inner.modify_power(m, base, ranged)
    }
    fn modify_recharge(&mut self, m: u32, base: F32) -> Result<Binary> {
        self.inner.modify_recharge(m, base)
    }
    fn log_enabled(&mut self) -> Result<bool> {
        Ok(false)
    }
}
impl aa_combat::combat_value::DefinitionWorld for Definitions {
    fn definition(&mut self, id: u32) -> Result<aa_combat::combat_value::DefinitionThing> {
        Ok(self.things[id as usize].clone())
    }
    fn flags_174(&mut self, id: u32) -> Result<u32> {
        Ok(self.things[id as usize].flags_174)
    }
    fn field(&mut self, id: u32, field: aa_combat::combat_value::DefinitionField) -> Result<F32> {
        Ok(self.fields[&(id, field as u8)])
    }
    fn set_field(
        &mut self,
        id: u32,
        field: aa_combat::combat_value::DefinitionField,
        value: F32,
    ) -> Result<()> {
        self.fields.insert((id, field as u8), value);
        Ok(())
    }
    fn log_prefix(&mut self, _: u32) -> Result<()> {
        Err(Error::Invalid(
            "diagnostic stream disabled in these fixtures",
        ))
    }
    fn log_row(&mut self, _: u32, _: [u64; 17]) -> Result<()> {
        Err(Error::Invalid(
            "diagnostic stream disabled in these fixtures",
        ))
    }
    fn csv_begin(&mut self, _: &MeanValues) -> Result<()> {
        // Fixture SVar CombatValueCSV is empty: nothing is opened.
        Ok(())
    }
    fn csv_close(&mut self) -> Result<()> {
        Err(Error::Invalid(
            "diagnostic stream disabled in these fixtures",
        ))
    }
}

#[test]
fn definition_value_matches_native() {
    use aa_combat::combat_value::{DefinitionField, DefinitionThing};
    let doc = fixture();
    let cases = doc["definition_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 180);
    let (mut recursed, mut lists) = (0, 0);
    for case in cases {
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let m = &case["means"];
        let means = MeanValues {
            maximum_74: F32::ZERO,
            attack: f(&m["attack"]),
            attack_rate: F32::ZERO,
            defense: f(&m["defense"]),
            movement: f(&m["movement"]),
            health: F32::ZERO,
            lifetime: f(&m["lifetime"]),
            offense: f(&m["offense"]),
        };
        let s = &case["settings"];
        let settings = Settings {
            damage_minimum: f(&s["damage_minimum"]),
            range_power: f(&s["range_power"]),
            area_power: f(&s["area_power"]),
            offense_weight: f(&s["offense_weight"]),
            defense_weight: f(&s["defense_weight"]),
            movement_weight: f(&s["movement_weight"]),
            movement_default: f(&s["movement_default"]),
        };
        let raw = case["things"].as_array().unwrap();
        let mut fields = std::collections::BTreeMap::new();
        let things: Vec<DefinitionThing> = raw
            .iter()
            .enumerate()
            .map(|(i, t)| {
                fields.insert(
                    (i as u32, DefinitionField::Value1E4 as u8),
                    f(&t["initial_1e4"]),
                );
                for field in [DefinitionField::Children30C, DefinitionField::Weighted36C] {
                    fields.insert((i as u32, field as u8), F32(0x1234_5678));
                }
                DefinitionThing {
                    flags_174: u(&t["flags_174"]),
                    list_1d4: t["list_1d4"].as_array().unwrap().iter().map(f).collect(),
                    defense_288: f(&t["defense_288"]),
                    health_284: f(&t["health_284"]),
                    speed_228: f(&t["speed_228"]),
                    bonus_210: f(&t["bonus_210"]),
                    abilities: t["abilities"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|a| local(definition(&a["definition"]), &a["local"]))
                        .collect(),
                    children_2f0: t["children_2f0"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|c| c.as_u64().map(|c| c as u32))
                        .collect(),
                    weighted_35c: t["weighted_35c"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|w| (u(&w[0]), w[1].as_i64().unwrap() as i32))
                        .collect(),
                }
            })
            .collect();
        let mut world = Definitions {
            inner: Adapter {
                modifier: None,
                clamp: f(&s["clamp"]),
                svars: case["svars"].clone(),
                log: false,
                p,
            },
            things,
            fields,
        };
        match case["entry"].as_str().unwrap() {
            "definition_4618b0" => combat_value::definition_value(
                0,
                &means,
                &settings,
                CrtEnvironment::SSE2_DOUBLE53,
                &mut world,
                p,
            ),
            "list_4bb590" => {
                lists += 1;
                let ids: Vec<u32> = (0..raw.len() as u32).collect();
                combat_value::actor_data_values(
                    &ids,
                    &means,
                    &settings,
                    CrtEnvironment::SSE2_DOUBLE53,
                    &mut world,
                    p,
                )
            }
            other => panic!("{other}"),
        }
        .unwrap();
        for (i, out) in case["out"].as_array().unwrap().iter().enumerate() {
            let i = i as u32;
            assert_eq!(world.fields[&(i, 0)], f(&out["value_1e4"]), "{i} {case}");
            assert_eq!(world.fields[&(i, 1)], f(&out["children_30c"]), "{i} {case}");
            assert_eq!(world.fields[&(i, 2)], f(&out["weighted_36c"]), "{i} {case}");
            if i > 0 && f(&out["value_1e4"]) != f(&raw[i as usize]["initial_1e4"]) {
                recursed += 1;
            }
        }
    }
    assert!(recursed > 30 && lists == 90, "{recursed} {lists}");
}
