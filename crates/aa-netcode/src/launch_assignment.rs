//! Native launch-settings value assignment, `0x779F00`, including its
//! `0x5A46D0`, `0x5145E0`, `0x513690` and `0x4145C0` descendants.
//!
//! `lobby::LaunchSettings` is the existing wire projection. The native copy
//! also preserves fields never transmitted by that codec; [`Retained`] owns
//! those values explicitly. They must come from the caller's real settings,
//! never from guessed zeroes. No wire layout or registry encoding changes.
//!
//! Oracle: `research/phase8-net-integration/native-launch-assignment.json`:
//! original and v14 instructions, nested grow/shrink/empty values, embedded
//! NUL/unpaired UTF-16 and self-assignment. Rust storage is growable; original
//! allocation capacities/CString reference counts are not gameplay state.
//! Allocation failure/exception parity is not established by that oracle.
use crate::lobby::{GameSettings, LaunchSettings, MapSettings, Session};
use crate::staging::SettingsSlot;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

/// The additional fields copied by KingdomSetup assignment `0x4A4C00`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomRetained {
    /// CString at +0x0C, omitted by the network kingdom codec.
    pub text_0c: Vec<u16>,
    pub flag_68: bool,
}

/// Same list order as each corresponding MapHeader::kingdoms vector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomLists {
    pub game: Vec<KingdomRetained>,
    pub map_default: Vec<KingdomRetained>,
    pub map_scenario: Vec<KingdomRetained>,
    pub map_random: Vec<KingdomRetained>,
    pub map_24: Vec<KingdomRetained>,
    pub map_metagame: Vec<KingdomRetained>,
}

/// Values copied by the native assignment but absent from its wire codec.
/// Deliberately has no Default: callers must supply the actual local state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retained {
    pub text_10: Vec<u16>,
    pub game_field_14: u32,
    pub random_text_74: Vec<u16>,
    pub map24_field_50: u32,
    pub metagame_flag_50: bool,
    pub metagame_field_54: u32,
    pub metagame_field_58: u32,
    pub kingdoms: KingdomLists,
}
impl Retained {
    /// Successful native `0x76EBD0` / `0x5104A0` reads replace every kingdom
    /// with an assignment from a `0x50F4C0(1)` temporary. Its non-wire +0x0C
    /// CString is the static empty string and +0x68 is zero. They are not the
    /// old destination values or private values belonging to the sender.
    /// Every other retained field is left untouched by the wire read.
    ///
    /// Proof: `native-snapshot-companions.json`, full original/v14 snapshot
    /// reads, including equal-count name replacement/reordering and shrink/grow.
    pub(crate) fn after_snapshot_read(&mut self, wire: &LaunchSettings) -> Result<()> {
        for (values, header) in [
            (&mut self.kingdoms.game, &wire.game.header),
            (&mut self.kingdoms.map_default, &wire.map_default.header),
            (&mut self.kingdoms.map_scenario, &wire.map_scenario.header),
            (&mut self.kingdoms.map_random, &wire.map_random.header),
            (&mut self.kingdoms.map_24, &wire.map_24.header),
            (&mut self.kingdoms.map_metagame, &wire.map_metagame.header),
        ] {
            let mut decoded = Vec::new();
            decoded
                .try_reserve_exact(header.kingdoms.len())
                .map_err(|_| {
                    Error::Protocol("decoded kingdom companion allocation failed".into())
                })?;
            decoded.resize_with(header.kingdoms.len(), || KingdomRetained {
                text_0c: Vec::new(),
                flag_68: false,
            });
            *values = decoded;
        }
        Ok(())
    }

    /// Checks vector extents only. The caller must preserve the exact kingdom
    /// record correspondence: equal-length reordering/replacement by a wire
    /// reader cannot be detected here and requires companion reconstruction.
    pub fn validate(&self, wire: &LaunchSettings) -> Result<()> {
        for (name, extras, header) in [
            ("game", &self.kingdoms.game, &wire.game.header),
            (
                "map_default",
                &self.kingdoms.map_default,
                &wire.map_default.header,
            ),
            (
                "map_scenario",
                &self.kingdoms.map_scenario,
                &wire.map_scenario.header,
            ),
            (
                "map_random",
                &self.kingdoms.map_random,
                &wire.map_random.header,
            ),
            ("map_24", &self.kingdoms.map_24, &wire.map_24.header),
            (
                "map_metagame",
                &self.kingdoms.map_metagame,
                &wire.map_metagame.header,
            ),
        ] {
            if extras.len() != header.kingdoms.len() {
                return Err(Error::Protocol(format!(
                    "launch assignment {name} has {} retained kingdom records for {} wire records",
                    extras.len(),
                    header.kingdoms.len()
                )));
            }
        }
        Ok(())
    }
}

/// A complete value owner; `wire` continues to use the original codecs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompleteSettings {
    pub wire: LaunchSettings,
    pub retained: Retained,
}
impl CompleteSettings {
    pub fn assign_from(&mut self, source: &Self) -> Result<()> {
        assign(
            &mut self.wire,
            &mut self.retained,
            &source.wire,
            &source.retained,
        )
    }
}

fn game_assign(dst: &mut GameSettings, src: &GameSettings) {
    // 0x5A46D0: conditions and header precede scalar/text fields.
    dst.conditions.clone_from(&src.conditions);
    dst.header.clone_from(&src.header);
    dst.version = src.version;
    dst.field_10 = src.field_10;
    dst.pair_20.clone_from(&src.pair_20);
    dst.pair_28.clone_from(&src.pair_28);
    dst.text_38.clone_from(&src.text_38);
    dst.text_3c.clone_from(&src.text_3c);
    dst.field_40 = src.field_40;
    dst.entries.clone_from(&src.entries);
    dst.time_low = src.time_low;
    dst.time_high = src.time_high;
    dst.field_30 = src.field_30;
    dst.field_34 = src.field_34;
}
fn map_assign(dst: &mut MapSettings, src: &MapSettings) {
    // 0x5145E0: map header, conditions and dimensions/definition identities.
    dst.header.clone_from(&src.header);
    dst.conditions.clone_from(&src.conditions);
    dst.size_x = src.size_x;
    dst.size_y = src.size_y;
    dst.def_44 = src.def_44;
    dst.def_48 = src.def_48;
    dst.field_4c = src.field_4c;
}

/// `0x779F00` value assignment. Both complete inputs must be present. A
/// mismatched projection is refused before mutation, rather than losing the
/// non-wire kingdom fields. All successful native value stores are reproduced;
/// memory-manager allocation failures are outside this port's proven domain.
pub fn assign(
    dst: &mut LaunchSettings,
    retained: &mut Retained,
    src: &LaunchSettings,
    other: &Retained,
) -> Result<()> {
    retained.validate(dst)?;
    other.validate(src)?;
    dst.game_type = src.game_type;
    dst.mode = src.mode;
    dst.file.clone_from(&src.file);
    dst.text_30.clone_from(&src.text_30);
    dst.field_0c = src.field_0c;
    retained.text_10.clone_from(&other.text_10);
    game_assign(&mut dst.game, &src.game);
    retained.game_field_14 = other.game_field_14;
    retained.kingdoms.game.clone_from(&other.kingdoms.game);
    map_assign(&mut dst.map_default, &src.map_default);
    retained
        .kingdoms
        .map_default
        .clone_from(&other.kingdoms.map_default);
    dst.map_default_flag = src.map_default_flag;
    map_assign(&mut dst.map_scenario, &src.map_scenario);
    retained
        .kingdoms
        .map_scenario
        .clone_from(&other.kingdoms.map_scenario);
    dst.scenario.clone_from(&src.scenario);
    map_assign(&mut dst.map_random, &src.map_random);
    retained
        .kingdoms
        .map_random
        .clone_from(&other.kingdoms.map_random);
    dst.random_map.clone_from(&src.random_map);
    dst.rmc_settings.clone_from(&src.rmc_settings);
    retained.random_text_74.clone_from(&other.random_text_74);
    map_assign(&mut dst.map_24, &src.map_24);
    retained.kingdoms.map_24.clone_from(&other.kingdoms.map_24);
    retained.map24_field_50 = other.map24_field_50;
    map_assign(&mut dst.map_metagame, &src.map_metagame);
    retained
        .kingdoms
        .map_metagame
        .clone_from(&other.kingdoms.map_metagame);
    retained.metagame_flag_50 = other.metagame_flag_50;
    retained.metagame_field_54 = other.metagame_field_54;
    retained.metagame_field_58 = other.metagame_field_58;
    dst.state.clone_from(&src.state);
    Ok(())
}

/// Concrete backing for the four slots used by native ChangeToState. Session
/// owns its two wire projections, this store owns their non-wire companions
/// and the last-used single-player/multiplayer settings. No duplicate live
/// Session::launch/launch_88 projection is stored here.
pub struct SettingsStore {
    pub launch: Retained,
    pub launch_88: Retained,
    pub single: CompleteSettings,
    pub multi: CompleteSettings,
}
impl SettingsStore {
    /// The four assignments actually requested by `0x77A750 / 0x77A6E0`.
    pub fn copy(
        &mut self,
        session: &mut Session,
        dst: SettingsSlot,
        src: SettingsSlot,
    ) -> Result<()> {
        match (dst, src) {
            (SettingsSlot::Launch, SettingsSlot::Launch88) => assign(
                &mut session.launch,
                &mut self.launch,
                &session.launch_88,
                &self.launch_88,
            ),
            (SettingsSlot::Launch88, SettingsSlot::Launch) => assign(
                &mut session.launch_88,
                &mut self.launch_88,
                &session.launch,
                &self.launch,
            ),
            (SettingsSlot::StoreSingle, SettingsSlot::Launch) => assign(
                &mut self.single.wire,
                &mut self.single.retained,
                &session.launch,
                &self.launch,
            ),
            (SettingsSlot::StoreMulti, SettingsSlot::Launch) => assign(
                &mut self.multi.wire,
                &mut self.multi.retained,
                &session.launch,
                &self.launch,
            ),
            _ => Err(Error::Unsupported(
                "launch-settings slot copy not called by 0x77A750/0x77A6E0",
            )),
        }
    }
}
