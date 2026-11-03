//! The RTS battle of a campaign territory on the battle client:
//! `LaunchRTSGame` 9336E0 (the playbattle order, then RequestGame 932210 ->
//! 948EF0, `aa_metagame::launch::battle_setup`) and what comes back
//! (93A990, `Campaign::rts_finished`).
//!
//! The orders of 948EF0 become this client's launch:
//! - SetSessionRmc, SetMapSize, SetRmcSettings: the random map of the territory's
//!   RMC (`mapgen::RandomMapParams`; the settings are 506370's slider midpoints,
//!   which are `RandomMapParams`' defaults for missing values). Every kingdom
//!   of the RMC gets a start location; a kingdom without a player leaves the
//!   battle as the world's unused slots do (`new_skirmish`);
//! - SetPlayerKingdom / CreateExplicitAi / SetKingdomColor / SetKingdomNation:
//!   the players, a participant merged (SetKingdomMergeInto) takes no player;
//! - SetKingdomFaction: the local player's general (`PendingGenerals`);
//! - AddKingdomStartProperty, SetKingdomBonusStockpile (4E39D0 at the kingdom's
//!   StartMission) and AddKingdomAdditionalActor go to the world when the
//!   battle exists ([`PendingBattle`]).
//!
//! The additional actors go to the generator (`RandomMapParams::additional_actors`,
//! KKC_RMCPlaceWorldParametersActorsStage 0x594930), each to the kingdom it merges into.
//!
//! A battle the local player is not part of ("Watch") runs with an
//! observer (`Battle::observer`). The AI's general of a computer kingdom stays
//! the nation's random pick (CreateExplicitAi with an empty profile).
use super::{BattleLaunch, BattleOrigin, Game};
use aa_metagame::battle::KingdomActor;
use aa_metagame::launch::{self, BattleSetup, LaunchEnv, LaunchOrder, PlayerId, RmcSliders, SessionPlayer};
use aa_metagame::{Error, Result as MgResult};
use bevy::prelude::*;
use std::path::Path;

/// The 948EF0 environment of a single-player campaign.
pub struct Env {
    pub players: Vec<SessionPlayer>,
    pub local: Option<PlayerId>,
    pub resources: Vec<String>,
    pub kingdom_ids: Vec<String>,
}

impl LaunchEnv for Env {
    fn players(&self) -> &[SessionPlayer] {
        &self.players
    }
    fn local_player(&self) -> Option<PlayerId> {
        self.local
    }
    /// 4BEDF0: the world kingdom's name, the campaign kingdom's IDS (driver.rs).
    fn kingdom_actor_name(&self, actor: KingdomActor) -> MgResult<String> {
        self.kingdom_ids.get(actor.0 as usize).cloned().ok_or_else(|| Error::Unsupported("no such kingdom actor".into()))
    }
    fn resource_registry(&self) -> &[String] {
        &self.resources
    }
    /// The settings' sliders stay `RandomMapParams`' defaults (506370's midpoints).
    fn rmc_sliders(&self, _rmc: &str) -> MgResult<RmcSliders> {
        Ok(RmcSliders::default())
    }
}

/// [`Env`] of a running campaign: the session's players, one per MG kingdom
/// (9447E0 makes a world kingdom per campaign kingdom; 947560 finds an attacker's
/// player by its kingdom's name through 76B920, unguarded), the local one human
/// (`+0C` 0), the others the MGAI's computers; and the resources in the registry
/// order.
pub fn env(game: &Game, resources: Vec<String>) -> Env {
    let c = &game.campaign.data.campaigns[game.campaign.state.campaign];
    let kingdom_ids: Vec<String> = c.kingdoms.iter().map(|k| k.ids.clone()).collect();
    let local = game.campaign.local.map(|k| PlayerId(k as u32));
    let players = game
        .campaign
        .state
        .kingdoms
        .iter()
        .enumerate()
        .map(|(i, k)| SessionPlayer { kingdom_name: c.kingdoms[k.data].ids.clone(), ai: u8::from(game.campaign.local != Some(i)), kingdom_actor: k.actor })
        .collect();
    Env { players, local, resources, kingdom_ids }
}

/// What the battle still takes when it exists (StartMission and the additional
/// actors): by player index.
#[derive(Resource, Clone, Debug, Default)]
pub struct PendingBattle {
    pub properties: Vec<(usize, String)>,
    /// Resource IDS and binary32 amount.
    pub bonus: Vec<(usize, String, u32)>,
    pub actors: Vec<(usize, String)>,
    /// The local player is not part of it: an observer ("Watch").
    pub watch: bool,
}

/// What the metagame needs to know when the battle returns (93A990).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MetaBattle {
    pub attacker: String,
    pub defender: String,
    /// `true` when the local player plays the attacker, `false` the defender.
    pub local_attacker: Option<bool>,
}

fn set_nation(data: &aa_play::GameData, ids: Option<&str>) -> Option<String> {
    let ids = ids?;
    // MG kingdom data's nation IDS ("nation_usa"); aa-play takes either spelling
    data.nation(ids).map(|n| n.id.clone()).or_else(|| data.nation(ids.trim_start_matches("nation_")).map(|n| n.id.clone()))
}

fn color_of(data: &aa_play::GameData, ids: Option<&str>) -> Option<[u8; 3]> {
    let ids = ids?;
    data.kingdom_colors.iter().find(|(i, _)| i.eq_ignore_ascii_case(ids)).map(|(_, c)| c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8))
}

/// The client's launch for a 948EF0 result.
pub fn from_orders(setup: &BattleSetup, parts: &aa_play::mapgen::CreatorParts, data: &aa_play::GameData, width: f32, height: f32, seed: u64) -> (BattleLaunch, PendingBattle, MetaBattle) {
    // the RMC's major kingdoms in creator order: the start locations
    let majors: Vec<&aa_play::mapgen::RmcKingdom> = parts.kingdoms.iter().filter(|k| !k.independent).collect();
    let start_of = |rmc: &str| majors.iter().position(|k| k.ids == rmc);
    let defender = setup.participants.iter().rev().find(|p| !p.merge_into.is_empty() || p.player.is_some() || !p.armies.is_empty()).map(|p| p.rmc_id.clone()).unwrap_or_default();
    #[derive(Default, Clone)]
    struct Row {
        rmc: String,
        name: String,
        nation: Option<String>,
        color: Option<[u8; 3]>,
        human: bool,
        ai: bool,
        faction: Option<String>,
    }
    let mut rows: Vec<Row> = Vec::new();
    let mut merged: Vec<(String, String)> = Vec::new();
    let mut pending = PendingBattle::default();
    let mut raw_props: Vec<(String, String)> = Vec::new();
    let mut raw_bonus: Vec<(String, String, u32)> = Vec::new();
    let mut raw_actors: Vec<(String, String)> = Vec::new();
    fn row<'a>(rows: &'a mut Vec<Row>, id: &str) -> &'a mut Row {
        if let Some(i) = rows.iter().position(|r| r.rmc == id) {
            return &mut rows[i];
        }
        rows.push(Row { rmc: id.to_string(), ..Default::default() });
        rows.last_mut().expect("just pushed")
    }
    for o in &setup.orders {
        match o {
            LaunchOrder::SetPlayerKingdom { player: Some(_), kingdom } if !kingdom.is_empty() => row(&mut rows, kingdom).human = true,
            LaunchOrder::CreateExplicitAi { kingdom, name, .. } => {
                let r = row(&mut rows, kingdom);
                r.ai = true;
                r.name = name.clone();
            }
            LaunchOrder::SetKingdomMergeInto { kingdom, merge_into } => merged.push((kingdom.clone(), merge_into.clone())),
            LaunchOrder::SetKingdomColor { kingdom, color } => row(&mut rows, kingdom).color = color_of(data, color.as_deref()),
            LaunchOrder::SetKingdomNation { kingdom, nation } => row(&mut rows, kingdom).nation = set_nation(data, nation.as_deref()),
            LaunchOrder::SetKingdomFaction { kingdom, faction } => row(&mut rows, kingdom).faction = faction.clone(),
            LaunchOrder::AddKingdomStartProperty { kingdom, property: Some(p) } => raw_props.push((kingdom.clone(), p.clone())),
            LaunchOrder::SetKingdomBonusStockpile { kingdom, resource, amount_bits } => raw_bonus.push((kingdom.clone(), resource.clone(), *amount_bits)),
            LaunchOrder::AddKingdomAdditionalActor { kingdom, actor } => raw_actors.push((kingdom.clone(), actor.clone())),
            _ => {}
        }
    }
    // the human first: the battle's local kingdom is kingdom 0 (scene.rs)
    rows.retain(|r| r.human || r.ai);
    rows.sort_by_key(|r| !r.human);
    let target = |id: &str| merged.iter().find(|(k, _)| k == id).map_or(id.to_string(), |(_, m)| m.clone());
    let index = |rows: &[Row], id: &str| rows.iter().position(|r| r.rmc == target(id));
    for (k, p) in raw_props {
        if let Some(i) = index(&rows, &k) {
            pending.properties.push((i, p));
        }
    }
    for (k, r, a) in raw_bonus {
        // a merged kingdom's stockpile is the kingdom it merges into
        if merged.iter().any(|(m, _)| *m == k) {
            continue;
        }
        if let Some(i) = index(&rows, &k) {
            pending.bonus.push((i, r, a));
        }
    }
    let mut additional_actors = Vec::new();
    for (k, a) in raw_actors {
        if let Some(i) = index(&rows, &k) {
            additional_actors.push((rows[i].rmc.clone(), a.clone()));
            pending.actors.push((i, a));
        }
    }
    let mut players = Vec::new();
    for r in &rows {
        let Some(start) = start_of(&r.rmc) else {
            warn!("the RMC {} has no kingdom {}", setup.rmc, r.rmc);
            continue;
        };
        players.push(aa_play::PlayerSetup {
            name: if r.human { "Player".to_string() } else { r.name.clone() },
            nation: r.nation.clone().unwrap_or_else(|| "usa".to_string()),
            team: u32::from(r.rmc == defender),
            color: r.color.unwrap_or([255, 255, 255]),
            controller: if r.human { aa_play::Controller::Human } else { aa_play::Controller::Computer(1) },
            start,
        });
    }
    // the sides: every major kingdom of the RMC with its team
    let team_index = |k: &aa_play::mapgen::RmcKingdom| k.team.as_ref().and_then(|t| parts.teams.iter().position(|x| x.eq_ignore_ascii_case(t))).unwrap_or(0) as u32;
    let params = aa_play::mapgen::RandomMapParams {
        rmc: setup.rmc.clone(),
        width,
        height,
        sides: majors.len() as u32,
        teams: majors.iter().map(|k| team_index(k)).collect(),
        terrain_weights: Vec::new(),
        object_counts: Default::default(),
        heroes: 0,
        seed,
        additional_actors,
    };
    let factions: Vec<(usize, Option<String>)> = rows.iter().enumerate().filter(|(_, r)| r.faction.is_some()).map(|(i, r)| (i, r.faction.clone())).collect();
    let human_rmc = rows.iter().find(|r| r.human).map(|r| r.rmc.clone());
    pending.watch = human_rmc.is_none();
    let attacker = setup.participants.first().map(|p| p.rmc_id.clone()).unwrap_or_default();
    let meta = MetaBattle { local_attacker: human_rmc.as_ref().map(|h| *h != defender), attacker, defender };
    let battle = BattleLaunch {
        setup: aa_play::SkirmishSetup { map_id: setup.rmc.clone(), players, starting_resources: None, seed },
        random: Some(params),
        difficulty: None,
        profiles: Vec::new(),
        names: Vec::new(),
        factions,
        starting_experience: None,
        fog: None,
        origin: BattleOrigin::MetaGame,
    };
    (battle, pending, meta)
}

/// 9336E0's RequestGame for the pending battle.
pub fn launch(game: &Game, root: &Path, seed: u64) -> Result<(BattleLaunch, PendingBattle, MetaBattle), String> {
    let t = game.campaign.pending_battle().map_err(|e| e.to_string())?.ok_or("no pending battle")?;
    let data = crate::preload::game_data(root).map_err(|e| format!("{e:#}"))?;
    let resources: Vec<String> = data.resources.iter().map(|r| r.id.clone()).collect();
    let env = env(game, resources);
    let setup = launch::battle_setup(&game.campaign.state, &game.campaign.data, &game.campaign.relations, t, &env, aa_metagame::GAME_PRECISION).map_err(|e| e.to_string())?;
    let width = f32::from_bits(setup.map_size.0);
    let height = f32::from_bits(setup.map_size.1);
    let maps = crate::preload::random_maps(root).map_err(|e| format!("{e:#}"))?;
    let probe = aa_play::mapgen::RandomMapParams { rmc: setup.rmc.clone(), width, height, sides: 0, teams: Vec::new(), terrain_weights: Vec::new(), object_counts: Default::default(), heroes: 0, seed , additional_actors: Vec::new() };
    let parts = maps.creator_parts(&probe).map_err(|e| format!("{e:#}"))?;
    Ok(from_orders(&setup, &parts, &data, width, height, seed))
}

/// Battle start: [`PendingBattle`] on the new world, before its first tick.
pub fn start(mut battle: Option<ResMut<crate::session::Battle>>, pending: Option<Res<PendingBattle>>, mut commands: Commands) {
    let (Some(b), Some(p)) = (battle.as_mut(), pending.as_deref()) else { return };
    if !b.is_added() {
        return;
    }
    if p.watch {
        b.observer = true;
    }
    let extras = aa_parity::recording::BattleExtras { properties: p.properties.clone(), bonus: p.bonus.clone(), actors: p.actors.clone() };
    aa_parity::recording::apply_battle_extras(&mut b.world, &extras);
    commands.remove_resource::<PendingBattle>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_metagame::launch::{NewGameState, Participant, RmcSettings};

    #[test]
    fn the_orders_become_players_and_the_human_comes_first() {
        let p = |id: &str, player: Option<PlayerId>, merge: &str| Participant { rmc_id: id.into(), kind: 0, player, kingdom: Some(0), merge_into: merge.into(), armies: vec![0], divisions: Vec::new() };
        let setup = BattleSetup {
            rmc: "mg_rmc_x".into(),
            map_size: (384f32.to_bits(), 384f32.to_bits()),
            participants: vec![p("kingdom_attacker_1", None, ""), p("kingdom_attacker_2", None, "kingdom_attacker_1"), p("kingdom_defender", Some(PlayerId(0)), "")],
            orders: vec![
                LaunchOrder::SetSessionRmc { rmc: "mg_rmc_x".into() },
                LaunchOrder::SetRmcSettings { settings: RmcSettings { total_bits: 0, first: Vec::new(), second: Vec::new(), midpoint: 0 } },
                LaunchOrder::SetNewState { state: 2, game_state: NewGameState::rts_launch() },
                LaunchOrder::SetPlayerKingdom { player: Some(PlayerId(0)), kingdom: String::new() },
                LaunchOrder::CreateExplicitAi { kingdom: "kingdom_attacker_1".into(), second: String::new(), name: "#k".into() },
                LaunchOrder::SetKingdomMergeInto { kingdom: "kingdom_attacker_2".into(), merge_into: "kingdom_attacker_1".into() },
                LaunchOrder::SetPlayerKingdom { player: Some(PlayerId(0)), kingdom: "kingdom_defender".into() },
                LaunchOrder::AddKingdomStartProperty { kingdom: "kingdom_defender".into(), property: Some("tech".into()) },
                LaunchOrder::SetKingdomBonusStockpile { kingdom: "kingdom_attacker_2".into(), resource: "money".into(), amount_bits: 5f32.to_bits() },
                LaunchOrder::SetKingdomBonusStockpile { kingdom: "kingdom_defender".into(), resource: "money".into(), amount_bits: 7f32.to_bits() },
                LaunchOrder::AddKingdomAdditionalActor { kingdom: "kingdom_attacker_2".into(), actor: "tank".into() },
                LaunchOrder::Launch,
            ],
        };
        let k = |id: &str, independent: bool| aa_play::mapgen::RmcKingdom { ids: id.into(), name: String::new(), independent, color: None, team: None };
        let parts = aa_play::mapgen::CreatorParts {
            kingdoms: vec![k("kingdom_attacker_1", false), k("kingdom_defender", false), k("kingdom_attacker_2", false), k("independent_0", true)],
            teams: Vec::new(),
            terrain_weights: Vec::new(),
            total_terrain_weight: 0.0,
            object_counts: Vec::new(),
        };
        let data = aa_play::GameData::default();
        let (battle, pending, meta) = from_orders(&setup, &parts, &data, 384.0, 384.0, 7);
        let starts: Vec<usize> = battle.setup.players.iter().map(|p| p.start).collect();
        // the human (kingdom_defender, start 1) first, the merged kingdom without a player
        assert_eq!(starts, [1, 0]);
        assert_eq!(battle.setup.players[0].controller, aa_play::Controller::Human);
        assert_eq!(battle.setup.players[0].team, 1);
        assert_eq!(battle.setup.players[1].team, 0);
        assert_eq!(battle.random.as_ref().unwrap().sides, 3);
        assert_eq!(pending.properties, [(0, "tech".to_string())]);
        // a merged kingdom's bonus is skipped, its actors go to the kingdom it merges into
        assert_eq!(pending.bonus, [(0, "money".to_string(), 7f32.to_bits())]);
        assert_eq!(pending.actors, [(1, "tank".to_string())]);
        assert_eq!(battle.random.as_ref().unwrap().additional_actors, [("kingdom_attacker_1".to_string(), "tank".to_string())]);
        assert_eq!(meta.local_attacker, Some(false));
        assert_eq!((meta.attacker.as_str(), meta.defender.as_str()), ("kingdom_attacker_1", "kingdom_defender"));
    }
}
