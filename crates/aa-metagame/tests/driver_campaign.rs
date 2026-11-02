//! The single-player campaign driver (`aa_metagame::driver`) on the shipped
//! campaigns (research/phase8-metagame/metagame-data.json, the oracle input):
//! - its team relations are the relation matrices every MGAI controller
//!   oracle world (native-mgai-controller-c*.json) was built with, so the
//!   proofs of the controller, PlayTurn, orders and resolution carry over;
//! - a headless campaign of computer players runs whole rounds: armies
//!   move, battles are started and resolved by quickbattle, rounds advance;
//! - the same seed gives the same campaign (the fiber thread is a pure
//!   hand-off, no timing enters without a budget);
//! - a local player ends its turn, the computer kingdoms play theirs and
//!   the turn comes back to the local player in the next round;
//! - the right click's command choice (9521A0) and the refusal after a
//!   computer player's failure.
use aa_metagame::battle::{KingdomActor, KingdomRelations, Side};
use aa_metagame::commands::Command;
use aa_metagame::data::MetaGameData;
use aa_metagame::driver::{Campaign, Note, Options, TeamRelations};
use aa_metagame::mgai::controller::OrderEffect;
use aa_metagame::resolve::EngineCall;
use aa_metagame::state::ArmyId;
use serde_json::Value;
use std::sync::Arc;

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-metagame")
}

fn data() -> Arc<MetaGameData> {
    let bytes = std::fs::read(dir().join("metagame-data.json")).unwrap();
    Arc::new(serde_json::from_slice(&bytes).unwrap())
}

fn never_busy(_: ArmyId) -> bool {
    false
}

#[test]
fn team_relations_are_the_oracle_worlds_relations() {
    let data = data();
    let mut checked = 0;
    for ci in 0..data.campaigns.len() {
        let fx: Value = serde_json::from_slice(&std::fs::read(dir().join(format!("native-mgai-controller-c{ci}.json"))).unwrap()).unwrap();
        let rel = TeamRelations::of_campaign(&data, ci);
        for w in fx["worlds"].as_array().unwrap() {
            for op in w["ops"].as_array().unwrap().iter().filter(|op| op[0] == "relations") {
                for (m, f) in [(&op[1], 0), (&op[2], 1)] {
                    for (s, row) in m.as_array().unwrap().iter().enumerate() {
                        for (o, v) in row.as_array().unwrap().iter().enumerate() {
                            let (s, o) = (KingdomActor(s as u32), KingdomActor(o as u32));
                            let ours = if f == 0 { rel.enemy(s, o) } else { rel.relation_138(s, o) };
                            assert_eq!(ours, v.as_bool().unwrap(), "campaign {ci} matrix {f} {s:?} {o:?}");
                        }
                    }
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 15);
}

/// Counts of what a run did.
#[derive(Debug, Default, PartialEq, Eq)]
struct Tally {
    moves: usize,
    battles: usize,
    kills: usize,
    purchases: usize,
    end_turns: usize,
    ai_failed: Vec<String>,
}

fn tally(notes: &[Note], t: &mut Tally) {
    for n in notes {
        match n {
            Note::Engine(EngineCall::MoveVisual { .. }) => t.moves += 1,
            Note::Engine(EngineCall::BattleMessages { .. }) => t.battles += 1,
            Note::Engine(EngineCall::KillArmy { .. }) => t.kills += 1,
            Note::Order { result, command, move_key, .. } => {
                // 9537B0 chooses a text for every move and attack order
                assert_eq!(move_key.is_some(), matches!(command.data.map(|d| d.index), Some(0 | 1)) && command.territory.is_some());
                match &result.effect {
                Some(OrderEffect::Process(out)) if out.created.is_some() => t.purchases += 1,
                Some(OrderEffect::EndTurn(_)) => t.end_turns += 1,
                _ => {}
                }
            }
            Note::AiFailed { error, .. } => t.ai_failed.push(error.clone()),
            _ => {}
        }
    }
}

fn headless(ci: usize, seed: u32, rounds: i32) -> (Campaign, Tally) {
    let options = Options { auto_quick_battle: true, ..Options::default() };
    let mut c = Campaign::new(data(), ci, None, None, seed, options).unwrap();
    let mut t = Tally::default();
    let mut ticks = 0;
    while c.state.round <= rounds && !c.state.game_over {
        c.tick(&never_busy).unwrap();
        tally(&std::mem::take(&mut c.notes), &mut t);
        ticks += 1;
        assert!(ticks < 200_000, "campaign {ci} stuck in round {} (kingdom {:?})", c.state.round, c.state.current);
    }
    (c, t)
}

#[test]
fn computer_campaigns_play_whole_rounds() {
    for ci in 0..5 {
        let (c, t) = headless(ci, 1234, 3);
        println!("campaign {ci}: round {} {:?}", c.state.round, t);
        assert!(c.state.round > 3 || c.state.game_over, "campaign {ci}");
        assert!(t.ai_failed.is_empty(), "campaign {ci}: {:?}", t.ai_failed);
        assert!(t.end_turns >= 15, "campaign {ci}: every kingdom ended three turns: {t:?}");
        assert!(t.moves > 0, "campaign {ci}: armies moved");
        assert!(t.purchases > 0, "campaign {ci}: armies bought");
    }
}

#[test]
fn same_seed_same_campaign() {
    let (a, ta) = headless(3, 77, 2);
    let (b, tb) = headless(3, 77, 2);
    assert_eq!(ta, tb);
    assert_eq!(a.state, b.state);
}

#[test]
fn local_player_turn_comes_back() {
    let data = data();
    let ci = 0;
    let local = data.campaigns[ci].default_kingdom;
    let mut c = Campaign::new(data, ci, Some(local), None, 5, Options::default()).unwrap();
    // InitCreate: kingdom 0 plays first, the campaign lists the player kingdom first
    assert_eq!(c.state.current, Some(local));
    assert!(c.local_turn());
    let end = Campaign::command("endturn").unwrap();
    assert_eq!(c.validate_local(&end).unwrap(), 0);
    c.send_local(end).unwrap();
    let mut ticks = 0;
    let mut t = Tally::default();
    loop {
        c.tick(&never_busy).unwrap();
        tally(&std::mem::take(&mut c.notes), &mut t);
        ticks += 1;
        if c.battle_waiting().unwrap() {
            // the local player answers every Start Battle dialog with Quick Battle
            let q = Campaign::command("quickbattle").unwrap();
            if !c.sent.iter().any(|(_, x)| x == &q) {
                c.send_local(q).unwrap();
            }
        }
        if c.local_turn() && c.state.round == 2 && c.sent.is_empty() {
            break;
        }
        assert!(ticks < 100_000, "stuck: round {} kingdom {:?}", c.state.round, c.state.current);
        if c.state.game_over {
            for (k, kk) in c.state.kingdoms.iter().enumerate() {
                println!("kingdom {k} capital {:?} owner {:?} territories {}", kk.capital, kk.capital.map(|t| c.state.territories[t].owner), kk.territories.len());
            }
            println!("won {:?} round {} current {:?} tally {t:?}", c.won, c.state.round, c.state.current);
        }
        assert!(!c.state.game_over);
    }
    assert!(t.ai_failed.is_empty(), "{:?}", t.ai_failed);
    assert_eq!(t.end_turns, 5, "the local player and the four computer kingdoms");
    // the local turn: the fiber does not play it (76B960: human TGC player)
    for _ in 0..50 {
        c.tick(&never_busy).unwrap();
    }
    assert!(c.local_turn());
    assert!(c.sent.is_empty());
}

#[test]
fn right_click_chooses_the_hot_command() {
    let data = data();
    let ci = 0;
    let local = data.campaigns[ci].default_kingdom;
    let c = Campaign::new(data, ci, Some(local), None, 5, Options::default()).unwrap();
    let me = c.local_actor().unwrap();
    let army = c.state.armies.iter().position(|a| a.as_ref().is_some_and(|a| a.owner == me)).map(|i| ArmyId(i as u32)).expect("the local kingdom starts with armies");
    let map = c.data.campaigns[ci].map.unwrap();
    let order = |t: usize, army: ArmyId| {
        let mut cmd = Command::default();
        cmd.set_territory(&c.data, map, c.state.territories[t].definition);
        cmd.army = Some(army);
        c.hot_command_for(&cmd, me).map(|d| d.index)
    };
    // an own territory: the first hot command that fits is move (index 0)
    let own = c.state.kingdoms[local].territories[0];
    assert_eq!(order(own, army), Some(0));
    // a hostile or unowned territory: attack (index 1, 954B30)
    let hostile = (0..c.state.territories.len()).find(|&t| c.state.owner_actor(t).is_none_or(|o| c.relations.enemy(o, me))).expect("a hostile territory");
    assert_eq!(order(hostile, army), Some(1));
    // somebody else's army: no hot command needs 0x800 unmet
    let other = c.state.armies.iter().position(|a| a.as_ref().is_some_and(|a| a.owner != me)).map(|i| ArmyId(i as u32)).expect("other armies");
    assert_eq!(order(own, other), None);
    // no territory: nothing to choose
    let mut cmd = Command::default();
    cmd.army = Some(army);
    assert_eq!(c.hot_command_for(&cmd, me), None);
}

#[test]
fn a_stopped_campaign_refuses_to_go_on() {
    let mut c = Campaign::new(data(), 0, None, None, 1, Options::default()).unwrap();
    c.stopped = Some("computer player failed".into());
    let tick = c.tick;
    assert!(c.tick(&never_busy).is_err());
    assert_eq!(c.tick, tick, "no world tick ran");
}

#[test]
fn fast_mgai_option_reaches_the_state() {
    let options = Options { fast_mgai: true, auto_quick_battle: true, ..Options::default() };
    let mut c = Campaign::new(data(), 0, None, None, 1, options).unwrap();
    assert_eq!(c.state.fast_ai, 0);
    c.tick(&never_busy).unwrap();
    assert_eq!(c.state.fast_ai, 1);
}

#[test]
fn a_waiting_battle_answers_the_dialog_queries() {
    let data = data();
    let local = data.campaigns[0].default_kingdom;
    let mut c = Campaign::new(data, 0, Some(local), None, 5, Options::default()).unwrap();
    let end = Campaign::command("endturn").unwrap();
    let mut ticks = 0;
    // the local player only ends its turns until a computer attack waits
    while !c.battle_waiting().unwrap() {
        if c.local_turn() && c.sent.is_empty() {
            c.send_local(end.clone()).unwrap();
        }
        c.tick(&never_busy).unwrap();
        ticks += 1;
        assert!(ticks < 100_000, "no battle in round {}", c.state.round);
    }
    let t = c.pending_battle().unwrap().expect("a waiting battle has its territory");
    assert!(!c.side_armies(t, Side::Attacker).is_empty());
    assert!(!c.side_armies(t, Side::Defender).is_empty());
    let odds = f32::from_bits(c.attack_odds_bits(t, aa_numeric::Precision::Single24).unwrap());
    assert!((0.0..=1.0).contains(&odds), "{odds}");
    // 94D620: part of it only as the defender or on the local turn
    let defender = c.defending_kingdom(t).unwrap();
    assert_eq!(c.local_involved().unwrap(), defender == Some(local) || c.local_turn());
    // the answer of the dialog ends the wait
    c.send_local(Campaign::command("quickbattle").unwrap()).unwrap();
    c.tick(&never_busy).unwrap();
    assert!(!c.battle_waiting().unwrap());
}
