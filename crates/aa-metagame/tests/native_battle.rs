//! Replays every case of research/phase8-metagame/native-battle.json (original
//! AA.exe code executed under Unicorn) against the Rust port.
use aa_metagame::battle::*;
use aa_metagame::rng::MetaGameRng;
use aa_numeric::{Binary, Precision};
use serde_json::Value;
use std::cmp::Ordering;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-metagame/native-battle.json"
    );
    serde_json::from_slice(&std::fs::read(path).expect("fixture")).expect("json")
}

fn precision(name: &str) -> Precision {
    match name {
        "single24" => Precision::Single24,
        "double53" => Precision::Double53,
        "extended64" => Precision::Extended64,
        other => panic!("precision {other}"),
    }
}

/// Exact Binary from an x87 80-bit fixture value.
fn ext(v: &Value) -> Binary {
    let sign = v["sign"].as_u64().unwrap();
    let exponent = v["exponent"].as_u64().unwrap() as i32;
    let significand = u64::from_str_radix(v["significand"].as_str().unwrap(), 16).unwrap();
    if exponent == 0 && significand == 0 {
        let z = Binary::from_u32(0);
        return if sign == 1 { z.negated() } else { z };
    }
    assert!(exponent != 0x7fff && exponent != 0, "fixture outside finite normal domain");
    let p = Precision::Extended64;
    let hi = Binary::from_u32((significand >> 32) as u32);
    let lo = Binary::from_u32(significand as u32);
    let two32 = Binary::from_f64_bits(0x41f0_0000_0000_0000).unwrap();
    let mantissa = hi.mul(&two32, p).unwrap().add(&lo, p).unwrap();
    // value = mantissa * 2^(exponent - 16383 - 63)
    let mut scale = exponent - 16383 - 63;
    let mut value = mantissa;
    while scale != 0 {
        let step = scale.clamp(-1000, 1000);
        let factor = Binary::from_f64_bits(((step + 1023) as u64) << 52).unwrap();
        value = value.mul(&factor, p).unwrap();
        scale -= step;
    }
    if sign == 1 { value.negated() } else { value }
}

fn same(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Equal && a.is_negative_nonzero() == b.is_negative_nonzero()
}

struct Table {
    enemy_140: Vec<Vec<bool>>,
    relation_138: Vec<Vec<bool>>,
}
impl KingdomRelations for Table {
    fn enemy(&self, s: KingdomActor, o: KingdomActor) -> bool {
        self.enemy_140[s.0 as usize][o.0 as usize]
    }
    fn relation_138(&self, s: KingdomActor, o: KingdomActor) -> bool {
        self.relation_138[s.0 as usize][o.0 as usize]
    }
}

fn bools(v: &Value) -> Vec<Vec<bool>> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_array().unwrap().iter().map(|b| b.as_bool().unwrap()).collect())
        .collect()
}
fn u32s(v: &Value) -> Vec<u32> {
    v.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u32).collect()
}
fn opt_index(v: &Value) -> Option<usize> {
    v.as_u64().map(|x| x as usize)
}

struct Case {
    kingdoms: Vec<BattleKingdom>,
    divisions: Vec<u32>,
    relations: Table,
    territory: TerritoryBattle,
    rng: u32,
    bonus: u32,
    base: u32,
}

fn case(input: &Value) -> Case {
    let kingdoms = input["mg_kingdoms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| BattleKingdom {
            actor: KingdomActor(k["actor"].as_u64().unwrap() as u32),
            owned_technologies: u32s(&k["owned_technologies"]).into_iter().map(TechnologyRef).collect(),
            shop: BattleShop {
                technologies: k["shop_technologies"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|e| ShopEntry {
                        item: TechnologyRef(e[0].as_u64().unwrap() as u32),
                        cost: e[1].as_i64().unwrap() as i32,
                    })
                    .collect(),
                divisions: k["shop_divisions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|e| ShopEntry {
                        item: DivisionRef(e[0].as_u64().unwrap() as u32),
                        cost: e[1].as_i64().unwrap() as i32,
                    })
                    .collect(),
            },
        })
        .collect();
    let armies = input["armies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| BattleArmy {
            owner: KingdomActor(a["kingdom_actor"].as_u64().unwrap() as u32),
            damage_bits: a["army_damage_bits"].as_u64().unwrap() as u32,
            divisions: u32s(&a["divisions"]).into_iter().map(DivisionRef).collect(),
        })
        .collect();
    Case {
        kingdoms,
        divisions: u32s(&input["division_damage_bits"]),
        relations: Table {
            enemy_140: bools(&input["enemy_140"]),
            relation_138: bools(&input["relation_138"]),
        },
        territory: TerritoryBattle {
            armies,
            enemy_army_count: input["enemy_army_count"].as_i64().unwrap() as i32,
            owner: opt_index(&input["owner"]).map(KingdomIndex),
        },
        rng: input["rng_state"].as_u64().unwrap() as u32,
        bonus: input["technology_bonus_bits"].as_u64().unwrap() as u32,
        base: input["attack_win_probability_bits"].as_u64().unwrap() as u32,
    }
}

#[test]
fn rng_matches_native_85d840() {
    let f = fixture();
    let rows = f["rng"].as_array().unwrap();
    assert!(rows.len() >= 200);
    for row in rows {
        let mut rng = MetaGameRng::new(row["state"].as_u64().unwrap() as u32);
        assert_eq!(u64::from(rng.next_u32()), row["returned"].as_u64().unwrap());
        assert_eq!(u64::from(rng.state), row["state_after"].as_u64().unwrap());
    }
}

#[test]
fn destroy_behaviour_table_matches_native_95c7b0() {
    let f = fixture();
    let rows = f["destroy_behaviour_enum"].as_array().unwrap();
    assert_eq!(rows.len(), DestroyBehavior::TABLE.len());
    for (row, (name, value)) in rows.iter().zip(DestroyBehavior::TABLE) {
        assert_eq!(row["name"].as_str().unwrap(), name);
        assert_eq!(row["value"].as_u64().unwrap(), u64::from(value.native_value()));
        assert_eq!(row["third_argument"].as_u64().unwrap(), 0);
    }
}

#[test]
fn odds_clamp_matches_native_95f2f0() {
    let f = fixture();
    let relations = Table { enemy_140: vec![], relation_138: vec![] };
    let mut checked = 0;
    for (pc, rows) in f["odds_clamp"].as_object().unwrap() {
        for row in rows.as_array().unwrap() {
            let ctx = BattleContext {
                kingdoms: &[],
                division_damage_bits: &[],
                relations: &relations,
                technology_bonus_bits: 0,
                attack_win_probability_bits: row["base_bits"].as_u64().unwrap() as u32,
                precision: precision(pc),
            };
            let got = ctx
                .odds_clamp(
                    row["defender_bits"].as_u64().unwrap() as u32,
                    row["attacker_bits"].as_u64().unwrap() as u32,
                )
                .unwrap();
            assert!(same(&got, &ext(&row["result"])), "{pc} {row}");
            checked += 1;
        }
    }
    assert_eq!(checked, 1200);
}

#[test]
fn quick_battle_and_selection_match_native() {
    let f = fixture();
    let battles = f["battles"].as_array().unwrap();
    assert_eq!(battles.len(), 160);
    let mut solved = [0usize; 3];
    for b in battles {
        let c = case(&b["input"]);
        for (pc, native) in b["native"].as_object().unwrap() {
            let ctx = BattleContext {
                kingdoms: &c.kingdoms,
                division_damage_bits: &c.divisions,
                relations: &c.relations,
                technology_bonus_bits: c.bonus,
                attack_win_probability_bits: c.base,
                precision: precision(pc),
            };
            let label = format!("case {} {pc}", b["input"]["index"]);
            for (army, want) in c.territory.armies.iter().zip(native["army_strength"].as_array().unwrap()) {
                assert!(same(&ctx.army_strength(army).unwrap(), &ext(want)), "{label} strength");
            }
            let defender = ctx.defender_strength(&c.territory).unwrap();
            assert!(same(&Binary::from_f32_bits(defender).unwrap(), &ext(&native["defender_strength"])), "{label} defender");
            assert_eq!(
                ctx.defending_kingdom(&c.territory).unwrap().map(|k| k.0),
                opt_index(&native["defending_kingdom"]),
                "{label} defending kingdom"
            );
            let bonuses = native["technology_bonus"].as_array().unwrap();
            for (i, want) in bonuses.iter().enumerate() {
                let k = (i < c.kingdoms.len()).then_some(KingdomIndex(i));
                let got = ctx.technology_bonus(k).unwrap();
                assert!(same(&Binary::from_f32_bits(got).unwrap(), &ext(want)), "{label} bonus {i}");
            }
            assert!(same(&ctx.attack_odds(&c.territory).unwrap(), &ext(&native["attack_odds"])), "{label} odds");
            let sel = &native["selection"];
            for (name, rule) in [
                ("weakest", SelectionRule::Weakest),
                ("strongest", SelectionRule::Strongest),
                ("most_expensive", SelectionRule::MostExpensive),
                ("least_expensive", SelectionRule::LeastExpensive),
            ] {
                for (side, sname) in [(Side::Attacker, 0), (Side::Defender, 1)] {
                    assert_eq!(
                        ctx.select_army(&c.territory, side, rule).unwrap(),
                        opt_index(&sel[format!("{name}_{sname}")]),
                        "{label} {name} side {sname}"
                    );
                }
            }
            // SolveBattle
            let mut rng = MetaGameRng::new(c.rng);
            let out = ctx.solve_battle(&c.territory, &mut rng).unwrap();
            let solve = &native["solve"];
            assert_eq!(u64::from(out.result.native_value()), solve["result"].as_u64().unwrap(), "{label} solve");
            solved[out.result.native_value() as usize] += 1;
            assert_eq!(u64::from(rng.state), solve["rng_state_after"].as_u64().unwrap(), "{label} rng");
            let log = solve["log"].as_array().unwrap();
            if out.result == SolveResult::NoBattle {
                assert!(log.is_empty());
            } else {
                assert_eq!(log.len(), 1);
                let doubles = log[0]["double_bits"].as_array().unwrap();
                let odds_log = u64::from_str_radix(doubles[0].as_str().unwrap(), 16).unwrap();
                let roll_log = u64::from_str_radix(doubles[1].as_str().unwrap(), 16).unwrap();
                // odds logged from its binary32 store; roll logged unrounded as binary64
                assert!(same(&Binary::from_f64_bits(odds_log).unwrap(), &Binary::from_f32_bits(out.odds_bits.unwrap()).unwrap()), "{label} odds log");
                assert_eq!(out.roll_log_f64_bits.unwrap(), roll_log, "{label} roll log");
            }
            // random destroy selection (state reset per side by the producer)
            for (side, key) in [(Side::Attacker, "0"), (Side::Defender, "1")] {
                let mut rng = MetaGameRng::new(c.rng);
                let got = ctx.random_army(&c.territory, side, &mut rng).unwrap();
                let want = &native["random_destroy"][key];
                let removed: Vec<usize> = want["removed"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
                assert_eq!(got.into_iter().collect::<Vec<_>>(), removed, "{label} random {key}");
                assert_eq!(u64::from(rng.state), want["rng_state_after"].as_u64().unwrap(), "{label} random rng {key}");
                assert_eq!(want["returned_low_byte"].as_u64().unwrap(), u64::from(got.is_some()), "{label} random ret {key}");
            }
        }
    }
    // the generated inputs must exercise every solve outcome
    assert!(solved.iter().all(|&n| n > 0), "{solved:?}");
}
