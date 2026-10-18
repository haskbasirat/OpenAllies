//! Metagame battle generation: what RTS battle the original sets up for a
//! contested territory (`KKC_MetaGameRTSLauncher::RequestGame`, 948EF0,
//! `MetaGameRTSLauncher.cpp`), stopping at the RTS launch order, and the
//! decision taken when the RTS battle returns (93A990).
//!
//! | native | role |
//! |---|---|
//! | 948EF0 | `RequestGame`: the order sequence of [`battle_setup`] |
//! | 947560 | participant builder ([`participants`]): attackers (955E50) then defenders (955E80) |
//! | 946010 / 945200 / 945BD0 / 945EF0 | participant record (0x2C bytes) find-or-create by (kingdom data, RMC kingdom id), add army |
//! | 93EB50 / 957910 | previous path territory; first link of the battle territory to it (`rmc_attacker_ids`) |
//! | 4A0A90 / 4A0900 / 4A0960 | RMC kingdom slot by id (independent slots first, then major; case-sensitive) |
//! | 76B920 | session player by kingdom name (case-sensitive, first in list order) |
//! | 506370 | RMC settings: slider midpoints ([`rmc_settings`]) |
//! | 946810 | `SetPlayerKingdom(player, "")` for every session player |
//! | 9459F0 / 4CCD80 | side stockpile sums (binary32 element adds) |
//! | 9466C0 / 945C90 / 945D10 | start properties: owned technologies, then division technologies of the side |
//! | 948BD0 / 9488B0 / 948770 | additional actors (armies, then divisions, per participant; no dedupe) |
//! | 954BE0 / 954B70 | friendly side test of a kingdom on the battle territory |
//! | 938BC0 | MG kingdom by campaign kingdom IDS (case-insensitive, first) |
//! | 93A990 | RTS result decision ([`rts_outcome`]) |
//!
//! Proven against `research/phase8-metagame/native-launch.json` (the
//! original code under Unicorn on perturbed InitCreate worlds of every shipped
//! campaign, PC53 first, PC24 and PC64 too) by `tests/native_launch.rs`.
//!
//! Engine inputs come through [`LaunchEnv`]: the session player list
//! (`B36550+0xBC`), the local player (`B36558+8`), kingdom actor names
//! (4BEDF0: actor `+0x18`), the
//! resource registry order (`B36530+0x34`; also the ResourceList size
//! `B3A660+0x28`, assumed to be the same registry) and the RMC sliders (RMC
//! `+0x50/+0x68/+0x74/+0x80`, read by 506370; the RMC loader is not ported).
//! The selected faction (`MGState+0x0C`) and the RTS winner name
//! (`MGState+0x48`) are read from [`MgState::faction`] / [`MgState::rts_winner`].
//!
//! Every path on which the original dereferences NULL (no RMC, no link to
//! the previous territory, an attacker path of one territory, a missing
//! session player for an attacker, a participant without kingdom data, ...)
//! returns [`Error::Unsupported`]; the orders the original would already have
//! sent before crashing are not returned.
use crate::battle::{
    BattleArmy, BattleContext, BattleKingdom, BattleShop, DivisionRef, KingdomActor, KingdomIndex,
    KingdomRelations, ShopEntry, SolveOutcome, SolveResult, TechnologyRef, TerritoryBattle,
};
use crate::commands::{CommandWorld, Validator};
use crate::data::{MetaGameData, ResourceValues, RmcDefinition, TerritoryDefinition};
use crate::state::{ArmyId, MgState};
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};

/// A session player: index in the session player list (`B36550+0xBC`, list order).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub u32);

/// The session player fields read by the launcher and 93A990.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPlayer {
    /// `+0x24`: name of the kingdom the player controls (compared with wcscmp).
    pub kingdom_name: String,
    /// `+0x0C` byte: non-zero = AI player.
    pub ai: u8,
    /// `+0x28` (765E00): the player's kingdom actor, if any.
    pub kingdom_actor: Option<KingdomActor>,
}

/// Engine inputs of the launcher.
pub trait LaunchEnv {
    /// Session players in list order (`B36550+0xBC`).
    fn players(&self) -> &[SessionPlayer];
    /// The local player (`B36558+8`); `None` for NULL.
    fn local_player(&self) -> Option<PlayerId>;
    /// `FUN_004BEDF0`: kingdom actor name (actor `+0x18`).
    fn kingdom_actor_name(&self, actor: KingdomActor) -> Result<String>;
    /// Resource type registry in native order (`B36530+0x34/+0x38`).
    fn resource_registry(&self) -> &[String];
    /// Sliders of an RMC (by RMC IDS), read by 506370.
    fn rmc_sliders(&self, rmc: &str) -> Result<RmcSliders>;
}

/// One RMC slider: a name and its authored range (binary32 bits).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmcSlider {
    pub name: String,
    pub min_bits: u32,
    pub max_bits: u32,
}

/// The RMC fields 506370 reads.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmcSliders {
    /// RMC `+0x50/+0x54` (entry `+4` min, `+8` max, name at `+0x14 -> +8`).
    pub first: Vec<RmcSlider>,
    /// RMC `+0x68/+0x6C` (entry `+0x18` min, `+0x1C` max, name at `+8`).
    pub second: Vec<RmcSlider>,
    /// RMC `+0x74/+0x78` (same layout), appended to the second list.
    pub third: Vec<RmcSlider>,
    /// RMC `+0x80` / `+0x84`.
    pub int_min: i32,
    pub int_max: i32,
}

impl RmcSliders {
    /// The fields 506370 reads, taken from a loaded RMC ([`crate::rmc`],
    /// loader 4A5420): `+0x50` terrain sliders (slider range `+4/+8`, terrain
    /// IDS through `+0x14`), `+0x68` doodad groups and `+0x74` actor groups
    /// (slider range `+0x18/+0x1C`, group IDS `+8`), and `+0x80/+0x84`
    /// `random_heroes_min/max` (stored there by 4A5A45/4A5A57).
    pub fn from_rmc(rmc: &crate::rmc::RmcData) -> Self {
        let group = |g: &crate::rmc::ObjectGroup| RmcSlider {
            name: g.ids.clone(),
            min_bits: g.slider.range[0],
            max_bits: g.slider.range[1],
        };
        RmcSliders {
            first: rmc
                .terrain_sliders
                .iter()
                .map(|t| RmcSlider { name: t.terrain.clone(), min_bits: t.slider.range[0], max_bits: t.slider.range[1] })
                .collect(),
            second: rmc.doodad_groups.iter().map(group).collect(),
            third: rmc.actor_groups.iter().map(group).collect(),
            int_min: rmc.heroes.random_heroes_min,
            int_max: rmc.heroes.random_heroes_max,
        }
    }
}

/// RMC settings object built by 506010 / 506370 (the `SetRMCSettings` payload).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmcSettings {
    /// `+0x00`: binary32 sum of the first list values, in list order.
    pub total_bits: u32,
    /// `+0x04` list: (name, binary32 midpoint) of RMC `+0x50`.
    pub first: Vec<(String, u32)>,
    /// `+0x10` list: RMC `+0x68` entries then RMC `+0x74` entries.
    pub second: Vec<(String, u32)>,
    /// `+0x1C`: `(max - min) / 2 + min` in signed 32-bit integers.
    pub midpoint: i32,
}

/// `0.5f` at 9FBDB8.
const HALF_F32: u32 = 0x3f00_0000;

/// `FUN_00506370`: every value is `(max - min) * 0.5f + min` evaluated on
/// the x87 stack at `precision` and stored as binary32; the total adds each
/// stored value to the stored total (`fld value; fadd total; fstp total`).
pub fn rmc_settings(s: &RmcSliders, precision: Precision) -> Result<RmcSettings> {
    let mid = |sl: &RmcSlider| -> Result<u32> {
        let min = Binary::from_f32_bits(sl.min_bits)?;
        let max = Binary::from_f32_bits(sl.max_bits)?;
        Ok(max
            .sub(&min, precision)?
            .mul(&Binary::from_f32_bits(HALF_F32)?, precision)?
            .add(&min, precision)?
            .store_f32()?)
    };
    let mut total = 0u32;
    let mut first = Vec::with_capacity(s.first.len());
    for sl in &s.first {
        let v = mid(sl)?;
        total = Binary::from_f32_bits(v)?
            .add(&Binary::from_f32_bits(total)?, precision)?
            .store_f32()?;
        first.push((sl.name.clone(), v));
    }
    let mut second = Vec::with_capacity(s.second.len() + s.third.len());
    for sl in s.second.iter().chain(&s.third) {
        second.push((sl.name.clone(), mid(sl)?));
    }
    // cdq; sub eax, edx; sar eax, 1; add eax, ecx
    let d = s.int_max.wrapping_sub(s.int_min);
    let half = d.wrapping_sub(d >> 31) >> 1;
    Ok(RmcSettings {
        total_bits: total,
        first,
        second,
        midpoint: half.wrapping_add(s.int_min),
    })
}

/// The 0x28-byte state built by 40F7D0 / 7841E0 and 784130(4, 1), sent by
/// `SetNewState(2, state)` (serializer 784360).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewGameState {
    pub kind: u32,
    pub flag_04: bool,
    pub text: String,
    pub field_0c: u32,
    pub field_10: u32,
    pub flag_14: bool,
    pub flag_15: bool,
    pub flag_16: bool,
    pub field_18: u32,
    pub field_1c: u32,
    pub flag_20: bool,
    pub field_24: u32,
}
impl NewGameState {
    /// The constant state of 948EF0: 7841E0 defaults, then 784130 sets
    /// `+0 = 6`, `+0x18 = 4`, `+0x1C = 1`.
    pub fn rts_launch() -> Self {
        Self {
            kind: 6,
            flag_04: false,
            text: String::new(),
            field_0c: 0xffff_ffff,
            field_10: 1,
            flag_14: false,
            flag_15: false,
            flag_16: false,
            field_18: 4,
            field_1c: 1,
            flag_20: false,
            field_24: 0,
        }
    }
}

/// Orders sent by 948EF0, in native order. `kingdom` fields are RMC
/// kingdom ids (a participant's `rmc_id`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaunchOrder {
    /// 75E700 / 75DD90 `TGC_SetSessionRMCOrder`: the RMC object, its IDS as
    /// the name, and the RMC game settings (RMC `+0x90`, copied by 5A44B0
    /// with its `+0x04` map-settings fields reset to a fresh 511DF0 object;
    /// opaque engine data, not modelled) plus their `+0x14` word.
    SetSessionRmc { rmc: String },
    /// 687510 / 683470 `TGC_SetRMCSettingsOrder` (506370 of the RMC).
    SetRmcSettings { settings: RmcSettings },
    /// 6837B0: the first `MapSize` entry of the RMC (binary32 bits).
    SetMapSize { width_bits: u32, height_bits: u32 },
    /// 4157A0 / 40E790 `TGC_SetNewStateOrder(2, state)`.
    SetNewState { state: u32, game_state: NewGameState },
    /// 40E6E0 `TGC_SetMapTypeOrder(0)`.
    SetMapType { map_type: u32 },
    /// 6971B0 / 6961B0 `TGC_SetPlayerKingdomOrder`.
    SetPlayerKingdom { player: Option<PlayerId>, kingdom: String },
    /// 6990C0 / 6960F0 `TGC_CreateExplicitAIOrder(kingdom, "", name)`: `name`
    /// is the participant's campaign kingdom `name` (kingdom data `+0x08`,
    /// 5B5620). Native 5B5620 replaces a `#key` by its string-table text
    /// (4B9F40) at load time; that table is not ported, so the key stands
    /// for the text (remaining seam). `second` is the empty string B43B40.
    CreateExplicitAi { kingdom: String, second: String, name: String },
    /// 9464A0 / 945600 `TGC_SetKingdomMergeIntoIDSOrder`.
    SetKingdomMergeInto { kingdom: String, merge_into: String },
    /// 946400 / 945540 `TGC_SetKingdomColorOrder` (kingdom data `+0x1C`).
    SetKingdomColor { kingdom: String, color: Option<String> },
    /// 697110 / 696270 `TGC_SetKingdomNationOrder` (kingdom data `+0x0C`).
    SetKingdomNation { kingdom: String, nation: Option<String> },
    /// 697110 / 696330 `TGC_SetKingdomFactionOrder` (`MGState+0x0C`, 5AA290).
    SetKingdomFaction { kingdom: String, faction: Option<String> },
    /// 946400 / 9456C0 `TGC_AddKingdomStartPropertyOrder` (technology IDS;
    /// `None` for a NULL division technology).
    AddKingdomStartProperty { kingdom: String, property: Option<String> },
    /// 946620 / 945900 `TGC_SetKingdomBonusStockpileOrder`.
    SetKingdomBonusStockpile { kingdom: String, resource: String, amount_bits: u32 },
    /// 946580 / 945780 `TGC_ClearKingdomAdditionalActorsOrder`.
    ClearKingdomAdditionalActors { kingdom: String },
    /// 946400 / 945840 `TGC_AddKingdomAdditionalActorOrder` (actor IDS).
    AddKingdomAdditionalActor { kingdom: String, actor: String },
    /// 40DF00 `TGC_LaunchOrder`: the RTS launch seam.
    Launch,
}

/// A battle participant (945BD0 record, 0x2C bytes).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Participant {
    /// `+0x00`: RMC kingdom id (link `rmc_attacker_ids` / territory `rmc_defender_ids`).
    pub rmc_id: String,
    /// `+0x04`: RMC kingdom slot type (`+0x1C` of the 4A0A90 slot): land 0, water 1.
    pub kind: u32,
    /// `+0x08`: controlling session player, `None` for NULL.
    pub player: Option<PlayerId>,
    /// `+0x0C`: campaign kingdom data (index into the campaign kingdoms), `None` for NULL.
    pub kingdom: Option<usize>,
    /// `+0x10`: RMC kingdom id this participant merges into ("" for none).
    pub merge_into: String,
    /// `+0x14`: army type of every added army (`MetaGameData::armies`), in order.
    pub armies: Vec<usize>,
    /// `+0x20`: live divisions of the added armies, first occurrence order.
    pub divisions: Vec<usize>,
}

/// What 948EF0 sets up for a contested territory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleSetup {
    /// RMC IDS of the territory (map territory `+0x74`).
    pub rmc: String,
    /// First `MapSize` entry (width, height) binary32 bits.
    pub map_size: (u32, u32),
    pub participants: Vec<Participant>,
    pub orders: Vec<LaunchOrder>,
}

fn unsupported<T>(what: impl Into<String>) -> Result<T> {
    Err(Error::Unsupported(what.into()))
}

/// Read-only view of everything the launcher consults.
struct Launch<'a, R: KingdomRelations, E: LaunchEnv> {
    state: &'a MgState,
    data: &'a MetaGameData,
    relations: &'a R,
    env: &'a E,
    precision: Precision,
}

impl<R: KingdomRelations, E: LaunchEnv> Launch<'_, R, E> {
    fn campaign(&self) -> &crate::data::Campaign {
        &self.data.campaigns[self.state.campaign]
    }
    fn definition(&self, t: usize) -> Result<&TerritoryDefinition> {
        let map = self
            .campaign()
            .map
            .ok_or_else(|| Error::Unsupported("campaign without map".into()))?;
        Ok(&self.data.maps[map].territories[self.state.territories[t].definition])
    }
    /// Map territory `+0x74`: the RMC resolved by `rmc_ids` at load.
    fn rmc(&self, t: usize) -> Result<&RmcDefinition> {
        let def = self.definition(t)?;
        if def.rmc_ids.is_empty() {
            return unsupported(format!("territory '{}' has no RMC (native reads NULL+0x44 / slot lists)", def.ids));
        }
        self.data
            .rmcs
            .iter()
            .find(|r| r.ids.eq_ignore_ascii_case(&def.rmc_ids))
            .ok_or_else(|| Error::Unsupported(format!("RMC '{}' not loaded", def.rmc_ids)))
    }
    /// `FUN_004A0A90` + slot `+0x1C`: independent slots (+0x104) first, then
    /// the major slots (+0xF8); wcscmp.
    fn rmc_kind(rmc: &RmcDefinition, id: &str) -> Result<u32> {
        let slot = rmc
            .kingdoms
            .iter()
            .filter(|k| k.independent)
            .chain(rmc.kingdoms.iter().filter(|k| !k.independent))
            .find(|k| k.ids == id)
            .ok_or_else(|| {
                Error::Unsupported(format!(
                    "RMC '{}' has no kingdom '{id}' (4A0A90 returns NULL, native reads +0x1C)",
                    rmc.ids
                ))
            })?;
        if slot.kind.eq_ignore_ascii_case("land") {
            Ok(0)
        } else if slot.kind.eq_ignore_ascii_case("water") {
            Ok(1)
        } else {
            unsupported(format!("RMC kingdom type '{}' (enum 4A1EA0 beyond land/water)", slot.kind))
        }
    }
    fn owner_actor(&self, t: usize) -> Result<Option<KingdomActor>> {
        match self.state.territories[t].owner {
            None => Ok(None),
            Some(k) => match self.state.kingdoms[k].actor {
                Some(a) => Ok(Some(a)),
                None => unsupported("territory owner without kingdom actor (native NULL virtual call)"),
            },
        }
    }
    /// Filter 955320 (955E50): unowned, or the owner regards the army kingdom as an enemy.
    fn attacker_side(&self, t: usize, army: ArmyId) -> Result<bool> {
        Ok(match self.owner_actor(t)? {
            None => true,
            Some(o) => self.relations.enemy(o, self.state.army(army).owner),
        })
    }
    /// Filter 9553A0 (955E80): owned and the owner's vtable+0x138 holds.
    fn defender_side(&self, t: usize, army: ArmyId) -> Result<bool> {
        Ok(match self.owner_actor(t)? {
            None => false,
            Some(o) => self.relations.relation_138(o, self.state.army(army).owner),
        })
    }
    /// `FUN_00938B70`: first MG kingdom whose actor is `actor`.
    fn kingdom_of_actor(&self, actor: KingdomActor) -> Option<usize> {
        self.state.kingdoms.iter().position(|k| k.actor == Some(actor))
    }
    /// `FUN_00938BC0`: first MG kingdom whose campaign kingdom IDS matches (wcsicmp).
    fn kingdom_by_data(&self, kd: usize) -> Option<usize> {
        let c = self.campaign();
        let ids = &c.kingdoms[kd].ids;
        self.state
            .kingdoms
            .iter()
            .position(|k| c.kingdoms[k.data].ids.eq_ignore_ascii_case(ids))
    }
    /// `FUN_0076B920`: first session player whose kingdom name equals `name` (wcscmp).
    fn player_by_name(&self, name: &str) -> Option<PlayerId> {
        self.env
            .players()
            .iter()
            .position(|p| p.kingdom_name == name)
            .map(|i| PlayerId(i as u32))
    }
    /// `FUN_00954BE0(territory, MG kingdom)` = `!954B70`: the kingdom is on
    /// the defending side (owned territory whose owner does not regard it as
    /// an enemy). NULL kingdom or unowned territory: false.
    fn friendly(&self, t: usize, k: Option<usize>) -> Result<bool> {
        let Some(k) = k else { return Ok(false) };
        let Some(owner) = self.owner_actor(t)? else { return Ok(false) };
        let Some(me) = self.state.kingdoms[k].actor else {
            return unsupported("954B70 with a kingdom without actor (native passes NULL to vtable+0x140)");
        };
        Ok(!self.relations.enemy(owner, me))
    }
    fn territory_battle(&self, t: usize) -> TerritoryBattle {
        territory_battle(self.state, self.data, t)
    }
    fn battle_kingdoms(&self) -> Result<Vec<BattleKingdom>> {
        battle_kingdoms(self.state, self.data)
    }

    /// `FUN_00946010` (with 945200 / 945BD0 / 945EF0).
    fn add(list: &mut Vec<Participant>, kingdom: Option<usize>, rmc_id: &str, kind: u32,
           player: Option<PlayerId>, merge_into: &str, army: &crate::state::MgArmy) {
        let index = match list.iter().position(|p| p.kingdom == kingdom && p.rmc_id == rmc_id) {
            Some(i) => i,
            None => {
                list.push(Participant {
                    rmc_id: rmc_id.to_string(),
                    kind,
                    player,
                    kingdom,
                    merge_into: merge_into.to_string(),
                    armies: Vec::new(),
                    divisions: Vec::new(),
                });
                list.len() - 1
            }
        };
        let p = &mut list[index];
        p.armies.push(army.definition);
        for d in &army.divisions {
            let d = d.0 as usize;
            if !p.divisions.contains(&d) {
                p.divisions.push(d);
            }
        }
    }

    /// `FUN_00947560`.
    fn participants(&self, t: usize) -> Result<Vec<Participant>> {
        let def = self.definition(t)?;
        let rmc = self.rmc(t)?;
        let mut list: Vec<Participant> = Vec::new();
        let mut first = String::new();
        for r in &self.state.territories[t].armies {
            if !self.attacker_side(t, r.army)? {
                continue;
            }
            let army = self.state.army(r.army);
            // 93EB50: the territory before the current one in the path
            if army.path.len() < 2 {
                return unsupported("947560: attacker path of at most one territory (93EB50 returns NULL)");
            }
            let previous = army.path[army.path.len() - 2];
            let origin = self.state.territories[previous].definition;
            // 957910: first link of the battle territory whose target is the origin
            let Some(link) = def.links.iter().find(|l| l.target == origin) else {
                return unsupported("947560: battle territory has no link to the attacker's previous territory (957910 returns NULL)");
            };
            let rmc_id = link.rmc_attacker_ids.clone();
            let kind = Self::rmc_kind(rmc, &rmc_id)?;
            let k = self.kingdom_of_actor(army.owner);
            let name = self.env.kingdom_actor_name(army.owner)?;
            let Some(p) = self.player_by_name(&name) else {
                return unsupported(format!("947560: no session player for attacker kingdom '{name}' (native reads NULL+0x0C)"));
            };
            let player = if self.env.players()[p.0 as usize].ai != 0 || !first.is_empty() {
                None
            } else {
                Some(p)
            };
            let Some(k) = k else {
                return unsupported("947560: attacker army kingdom has no MG kingdom (native reads NULL+0x08)");
            };
            let kd = self.state.kingdoms[k].data;
            Self::add(&mut list, Some(kd), &rmc_id, kind, player, &first, army);
            if first.is_empty() {
                first = rmc_id;
            }
        }
        // defenders: 955EB0 defending kingdom, map territory +0x78
        let kingdoms = self.battle_kingdoms()?;
        let damage: Vec<u32> = self.data.divisions.iter().map(|d| d.damage_bits).collect();
        let ctx = BattleContext {
            kingdoms: &kingdoms,
            division_damage_bits: &damage,
            relations: self.relations,
            technology_bonus_bits: 0,
            attack_win_probability_bits: 0,
            precision: self.precision,
        };
        let defending = ctx.defending_kingdom(&self.territory_battle(t))?;
        let kd = defending.map(|KingdomIndex(k)| self.state.kingdoms[k].data);
        let rmc_id = def.rmc_defender_ids.clone();
        let name = kd.map(|kd| self.campaign().kingdoms[kd].ids.clone()).unwrap_or_default();
        let player = self
            .player_by_name(&name)
            .filter(|p| self.env.players()[p.0 as usize].ai == 0);
        for r in &self.state.territories[t].armies {
            if !self.defender_side(t, r.army)? {
                continue;
            }
            let kind = Self::rmc_kind(rmc, &rmc_id)?;
            Self::add(&mut list, kd, &rmc_id, kind, player, "", self.state.army(r.army));
        }
        Ok(list)
    }

    fn resource_values(values: &ResourceValues, registry: &[String], what: &str) -> Result<Vec<u32>> {
        registry
            .iter()
            .map(|r| {
                values
                    .values
                    .iter()
                    .find(|(name, _)| name == r)
                    .map(|(_, v)| *v)
                    .ok_or_else(|| Error::Unsupported(format!("{what}: no value for resource '{r}' (registry mismatch)")))
            })
            .collect()
    }

    /// `FUN_004CCD80`: `dst[i] = f32(src[i] + dst[i])`.
    fn accumulate(&self, dst: &mut [u32], src: &[u32]) -> Result<()> {
        for (d, s) in dst.iter_mut().zip(src) {
            *d = Binary::from_f32_bits(*s)?
                .add(&Binary::from_f32_bits(*d)?, self.precision)?
                .store_f32()?;
        }
        Ok(())
    }

    /// `FUN_009459F0`: (friendly, hostile) side stockpiles.
    fn side_stockpiles(&self, t: usize, list: &[Participant]) -> Result<(Vec<u32>, Vec<u32>)> {
        let registry = self.env.resource_registry();
        let mut friendly = vec![0u32; registry.len()];
        let mut hostile = vec![0u32; registry.len()];
        let c = self.campaign();
        for p in list {
            let Some(kd) = p.kingdom else {
                return unsupported("9459F0: participant without kingdom data (938BC0 reads NULL+0x04)");
            };
            let friendly_side = self.friendly(t, self.kingdom_by_data(kd))?;
            let bucket = if friendly_side { &mut friendly } else { &mut hostile };
            let k = &c.kingdoms[kd];
            let own = if p.kind == 1 { &k.water_stockpile_bonus } else { &k.stockpile_bonus };
            let own = Self::resource_values(own, registry, &k.ids)?;
            self.accumulate(bucket, &own)?;
            for &a in &p.armies {
                let army = &self.data.armies[a];
                let values = if p.kind == 1 { &army.water_stockpile_bonus } else { &army.stockpile_bonus };
                let values = Self::resource_values(values, registry, &army.ids)?;
                self.accumulate(bucket, &values)?;
            }
        }
        Ok((friendly, hostile))
    }

    /// `FUN_00945C90` + `FUN_00945D10`.
    fn start_properties(&self, t: usize, kd: usize) -> Result<Vec<Option<String>>> {
        let Some(k) = self.kingdom_by_data(kd) else {
            return unsupported("945C90: kingdom data without MG kingdom (native reads NULL+0x24)");
        };
        let same = |a: &Option<String>, b: &Option<String>| match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
            _ => false,
        };
        let mut list: Vec<Option<String>> = Vec::new();
        let push = |v: Option<String>, list: &mut Vec<Option<String>>| {
            if !list.iter().any(|x| same(x, &v)) {
                list.push(v);
            }
        };
        for tech in &self.state.kingdoms[k].technologies {
            push(Some(tech.clone()), &mut list);
        }
        let friendly_side = self.friendly(t, Some(k))?;
        for r in &self.state.territories[t].armies {
            let on = if friendly_side {
                self.defender_side(t, r.army)?
            } else {
                self.attacker_side(t, r.army)?
            };
            if !on {
                continue;
            }
            for d in &self.state.army(r.army).divisions {
                for tech in &self.data.divisions[d.0 as usize].technologies {
                    push(tech.clone(), &mut list);
                }
            }
        }
        Ok(list)
    }

    /// `FUN_009488B0`: (rmc id, actor) rows, armies then divisions per participant.
    fn additional_actors(&self, list: &[Participant]) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for p in list {
            let pick = |land: &String, water: &String| -> Option<String> {
                let v = match p.kind {
                    1 => water,
                    0 => land,
                    _ => return None,
                };
                (!v.is_empty()).then(|| v.clone())
            };
            for &a in &p.armies {
                let army = &self.data.armies[a];
                if let Some(actor) = pick(&army.rts_actor_ids, &army.rts_water_actor_ids) {
                    out.push((p.rmc_id.clone(), actor));
                }
            }
            for &d in &p.divisions {
                let dv = &self.data.divisions[d];
                if let Some(actor) = pick(&dv.rts_actor_ids, &dv.rts_water_actor_ids) {
                    out.push((p.rmc_id.clone(), actor));
                }
            }
        }
        out
    }

    /// `FUN_00948EF0`.
    fn setup(&self, t: usize) -> Result<BattleSetup> {
        let list = self.participants(t)?;
        let rmc = self.rmc(t)?;
        let mut orders = Vec::new();
        orders.push(LaunchOrder::SetSessionRmc { rmc: rmc.ids.clone() });
        let settings = rmc_settings(&self.env.rmc_sliders(&rmc.ids)?, self.precision)?;
        orders.push(LaunchOrder::SetRmcSettings { settings });
        let Some(&(width_bits, height_bits)) = rmc.map_sizes.first() else {
            return unsupported(format!("RMC '{}' without MapSize (native reads entry 0 of an empty list)", rmc.ids));
        };
        orders.push(LaunchOrder::SetMapSize { width_bits, height_bits });
        orders.push(LaunchOrder::SetNewState { state: 2, game_state: NewGameState::rts_launch() });
        orders.push(LaunchOrder::SetMapType { map_type: 0 });
        // 946810
        for i in 0..self.env.players().len() {
            orders.push(LaunchOrder::SetPlayerKingdom { player: Some(PlayerId(i as u32)), kingdom: String::new() });
        }
        let (friendly, hostile) = self.side_stockpiles(t, &list)?;
        let c = self.campaign();
        let local = self.env.local_player();
        let mut local_seen = false;
        for p in &list {
            let Some(kd) = p.kingdom else {
                return unsupported("948EF0: participant without kingdom data");
            };
            let k = &c.kingdoms[kd];
            let mut appearance = true;
            match p.player {
                None if !p.merge_into.is_empty() => {
                    orders.push(LaunchOrder::SetKingdomMergeInto {
                        kingdom: p.rmc_id.clone(),
                        merge_into: p.merge_into.clone(),
                    });
                    appearance = false;
                }
                None => orders.push(LaunchOrder::CreateExplicitAi {
                    kingdom: p.rmc_id.clone(),
                    second: String::new(),
                    name: k.name.clone(),
                }),
                Some(player) => orders.push(LaunchOrder::SetPlayerKingdom {
                    player: Some(player),
                    kingdom: p.rmc_id.clone(),
                }),
            }
            if appearance {
                orders.push(LaunchOrder::SetKingdomColor {
                    kingdom: p.rmc_id.clone(),
                    color: (!k.color_ids.is_empty()).then(|| k.color_ids.clone()),
                });
                orders.push(LaunchOrder::SetKingdomNation {
                    kingdom: p.rmc_id.clone(),
                    nation: (!k.nation_ids.is_empty()).then(|| k.nation_ids.clone()),
                });
            }
            if p.player == local {
                local_seen = true;
                orders.push(LaunchOrder::SetKingdomFaction { kingdom: p.rmc_id.clone(), faction: self.state.faction.clone() });
            }
            for property in self.start_properties(t, kd)? {
                orders.push(LaunchOrder::AddKingdomStartProperty { kingdom: p.rmc_id.clone(), property });
            }
            let side = if self.friendly(t, self.kingdom_by_data(kd))? { &friendly } else { &hostile };
            for (r, &amount_bits) in self.env.resource_registry().iter().zip(side) {
                orders.push(LaunchOrder::SetKingdomBonusStockpile {
                    kingdom: p.rmc_id.clone(),
                    resource: r.clone(),
                    amount_bits,
                });
            }
        }
        if !local_seen {
            orders.push(LaunchOrder::SetPlayerKingdom { player: local, kingdom: String::new() });
        }
        // 948BD0
        for p in &list {
            orders.push(LaunchOrder::ClearKingdomAdditionalActors { kingdom: p.rmc_id.clone() });
        }
        for (kingdom, actor) in self.additional_actors(&list) {
            orders.push(LaunchOrder::AddKingdomAdditionalActor { kingdom, actor });
        }
        orders.push(LaunchOrder::Launch);
        Ok(BattleSetup {
            rmc: rmc.ids.clone(),
            map_size: (width_bits, height_bits),
            participants: list,
            orders,
        })
    }
}

/// Territory view for the battle code (954FD0 / 955EB0 / 95F480).
fn territory_battle(state: &MgState, data: &MetaGameData, t: usize) -> TerritoryBattle {
    let tt = &state.territories[t];
    TerritoryBattle {
        armies: tt
            .armies
            .iter()
            .map(|r| {
                let a = state.army(r.army);
                BattleArmy {
                    owner: a.owner,
                    damage_bits: data.armies[a.definition].damage_bits,
                    divisions: a.divisions.clone(),
                }
            })
            .collect(),
        enemy_army_count: tt.enemy_army_count,
        owner: tt.owner.map(KingdomIndex),
    }
}

/// Kingdom views with owned technologies and shops. Technology identity is
/// the registry object, i.e. the IDS resolved case-insensitively; every IDS
/// is interned to a [`TechnologyRef`].
fn battle_kingdoms(state: &MgState, data: &MetaGameData) -> Result<Vec<BattleKingdom>> {
    let c = &data.campaigns[state.campaign];
    let mut interned: Vec<String> = Vec::new();
    let mut intern = |ids: &str| -> TechnologyRef {
        let i = match interned.iter().position(|x| x.eq_ignore_ascii_case(ids)) {
            Some(i) => i,
            None => {
                interned.push(ids.to_string());
                interned.len() - 1
            }
        };
        TechnologyRef(i as u32)
    };
    let mut out = Vec::with_capacity(state.kingdoms.len());
    for k in &state.kingdoms {
        let Some(actor) = k.actor else {
            return unsupported("MG kingdom without kingdom actor (native NULL dereference)");
        };
        let shop = &c.kingdoms[k.data].shop;
        let owned_technologies = k.technologies.iter().map(|t| intern(t)).collect();
        let technologies = shop
            .technologies
            .iter()
            .filter_map(|r| r.ids.as_ref().map(|ids| ShopEntry { item: intern(ids), cost: r.cost }))
            .collect();
        let divisions = shop
            .divisions
            .iter()
            .filter_map(|r| {
                let ids = r.ids.as_ref()?;
                let d = data.divisions.iter().position(|x| &x.ids == ids)?;
                Some(ShopEntry { item: DivisionRef(d as u32), cost: r.cost })
            })
            .collect();
        out.push(BattleKingdom { actor, owned_technologies, shop: BattleShop { divisions, technologies } });
    }
    Ok(out)
}

/// `FUN_00947560`: the participants of a battle in territory `t` (MG territory index).
pub fn participants<R: KingdomRelations, E: LaunchEnv>(
    state: &MgState,
    data: &MetaGameData,
    relations: &R,
    t: usize,
    env: &E,
    precision: Precision,
) -> Result<Vec<Participant>> {
    Launch { state, data, relations, env, precision }.participants(t)
}

/// `FUN_00948EF0` (`KKC_MetaGameRTSLauncher::RequestGame`) for MG territory
/// `t`: participants and every order up to and including `Launch`.
pub fn battle_setup<R: KingdomRelations, E: LaunchEnv>(
    state: &MgState,
    data: &MetaGameData,
    relations: &R,
    t: usize,
    env: &E,
    precision: Precision,
) -> Result<BattleSetup> {
    Launch { state, data, relations, env, precision }.setup(t)
}

/// The decision 93A990 passes to `KKC_MGCommand::ResolveBattle` (954130).
#[derive(Clone, Debug, Serialize)]
pub struct RtsDecision {
    /// Pending battle territory (93A320).
    pub territory: usize,
    /// MG kingdom of the first attacker (955420 / 938B70).
    pub attacker: Option<usize>,
    /// Territory owner (`+0x24`).
    pub defender: Option<usize>,
    pub attacker_won: bool,
    /// The quick-battle solve used when the RTS reported no winner and the
    /// local player is on neither side.
    pub solved: Option<SolveOutcome>,
}

struct NoWorld;
impl CommandWorld for NoWorld {
    fn controllable(&self, _: ArmyId, _: KingdomActor) -> bool {
        false
    }
    fn reach(&self, _: &MgState, _: ArmyId, _: usize) -> Result<bool> {
        unsupported("93A990 does not search paths")
    }
}

/// `FUN_0093A990` up to its `ResolveBattle` call: with the application in
/// state 5 (`B36550+0xE4`, `app_state`), a pending battle (93A320) and the RTS
/// play flag (`MGState+0x45`) set, decide whether the attacker won from the
/// RTS winner name (`MGState+0x48`, [`MgState::rts_winner`]): a non-empty name wins for the
/// attacker unless it equals the territory's `rmc_defender_ids` (wcscmp). An
/// empty name: the local player's side loses (attacker side checked first,
/// both sides -> attacker wins); a local player on neither side gets one
/// `SolveBattle` draw (95F220 / 95F480). Clears `MGState+0x45` and returns the
/// decision; the caller applies it with [`crate::resolve::MgContext::resolve_battle`].
/// `Ok(None)` when a guard fails (no state change).
pub fn rts_outcome<R: KingdomRelations, E: LaunchEnv>(
    state: &mut MgState,
    data: &MetaGameData,
    relations: &R,
    env: &E,
    app_state: u32,
    precision: Precision,
) -> Result<Option<RtsDecision>> {
    if app_state != 5 {
        return Ok(None);
    }
    let pending = Validator { state: &*state, data, relations, world: &NoWorld, precision }.pending_battle()?;
    let Some(t) = pending else { return Ok(None) };
    if !state.battle_play_requested {
        return Ok(None);
    }
    // 955420(0): first army the owner regards as an enemy (all when unowned)
    let owner_actor = match state.territories[t].owner {
        None => None,
        Some(k) => Some(state.kingdoms[k].actor.ok_or_else(|| {
            Error::Unsupported("93A990: territory owner without kingdom actor".into())
        })?),
    };
    let first = state.territories[t].armies.iter().find(|r| match owner_actor {
        None => true,
        Some(o) => relations.enemy(o, state.army(r.army).owner),
    });
    let Some(first) = first else {
        return unsupported("93A990: pending battle without attacker (955420 returns NULL)");
    };
    let first_owner = state.army(first.army).owner;
    let attacker = state.kingdoms.iter().position(|k| k.actor == Some(first_owner));
    let defender = state.territories[t].owner;
    let map = data.campaigns[state.campaign]
        .map
        .ok_or_else(|| Error::Unsupported("campaign without map".into()))?;
    let def = &data.maps[map].territories[state.territories[t].definition];
    let winner = state.rts_winner.clone();
    let mut won = winner != def.rmc_defender_ids;
    let mut solved = None;
    if winner.is_empty() {
        let local = env.local_player();
        // 933170 on the MG kingdom (NULL kingdom: native crash), 76B960
        let player_of = |k: Option<usize>| -> Result<Option<PlayerId>> {
            let Some(k) = k else {
                return unsupported("93A990: 933170 on a NULL MG kingdom");
            };
            let actor = state.kingdoms[k].actor;
            Ok(env
                .players()
                .iter()
                .position(|p| p.kingdom_actor == actor)
                .map(|i| PlayerId(i as u32)))
        };
        let pa = player_of(attacker)?;
        let pd = player_of(defender)?;
        if local == pa {
            won = local == pd;
        } else if local == pd {
            won = true;
        } else {
            let kingdoms = battle_kingdoms(state, data)?;
            let damage: Vec<u32> = data.divisions.iter().map(|d| d.damage_bits).collect();
            let ctx = BattleContext {
                kingdoms: &kingdoms,
                division_damage_bits: &damage,
                relations,
                technology_bonus_bits: data.settings.technology_bonus_bits,
                attack_win_probability_bits: data.settings.attack_win_probability_bits,
                precision,
            };
            let tb = territory_battle(state, data, t);
            let outcome = ctx.solve_battle(&tb, &mut state.rng)?;
            won = outcome.result == SolveResult::AttackerWins;
            solved = Some(outcome);
        }
    }
    state.battle_play_requested = false;
    Ok(Some(RtsDecision { territory: t, attacker, defender, attacker_won: won, solved }))
}

/// `FUN_0093A990` complete: [`rts_outcome`] then `ResolveBattle` (954130,
/// [`crate::resolve::MgContext::resolve_battle`]) with the decision; `Ok(None)`
/// when a guard fails. The decision is proven by `native-launch.json` (954130
/// seamed there); 954130 itself by `native-resolve.json`.
pub fn process_rts_result<R: KingdomRelations, G: crate::resolve::Engine, E: LaunchEnv>(
    ctx: &mut crate::resolve::MgContext<'_, R, G>,
    env: &E,
    app_state: u32,
) -> Result<Option<bool>> {
    let decision = rts_outcome(&mut *ctx.state, ctx.data, ctx.relations, env, app_state, ctx.precision)?;
    let Some(d) = decision else { return Ok(None) };
    Ok(Some(ctx.resolve_battle(d.attacker, d.defender, d.territory, d.attacker_won)?))
}
