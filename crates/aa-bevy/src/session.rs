//! Shared contract between the menus/HUD/input side and the battle renderer.
//! Shared file: change only by agreement (add, never repurpose).
use aa_parity::playback::{FilmEvent, FilmPlayer};
use aa_play::{Command, KingdomId, SkirmishSetup, Unit, World};
use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    /// Original front-end menus (main menu, Single Player, Custom setup).
    #[default]
    Menu,
    /// Original loading screen while the battle is built.
    Loading,
    Battle,
}

/// The custom battle the setup screen asked for; the loader builds `Battle`.
/// Flow: the UI inserts this and sets AppState::Loading; the lead's loader
/// builds the battle in the background, updates `LoadProgress`, inserts
/// `Battle` and sets AppState::Battle (or AppState::Menu with `error` set).
#[derive(Resource, Clone, Debug)]
pub struct PendingSetup(pub SkirmishSetup);

/// Set together with `PendingSetup` when the player chose Random Map: the
/// loader generates the map from these original options instead of loading
/// `SkirmishSetup::map_id`.
#[derive(Resource, Clone, Debug)]
pub struct PendingRandomMap(pub aa_play::mapgen::RandomMapParams);

/// The session's Sun world rule (EnumValue IDS) for the loader (scene.rs,
/// StartMission 0x4FDCE0).
#[derive(Resource, Clone, Debug)]
pub struct PendingSunRule(pub String);

/// Loader progress for the original loading screen.
#[derive(Resource, Clone, Debug, Default)]
pub struct LoadProgress {
    /// 0..1.
    pub fraction: f32,
    /// Status text key or message.
    pub status: String,
    /// Set when loading failed; the UI shows it and returns to the menus.
    pub error: Option<String>,
}

/// The loader's current stage as the original's status key (Localization
/// `app_status_*`, set by the world load `0x4FC600` / `0x682EE0` in this
/// order: loading_data, generating_map / verifying_map for a random map,
/// loading_objects, loading_resources; `0x6E8C00` finished). Written by the
/// background loader thread, read by the loading screen. (Appended by the UI
/// owner.)
pub static LOAD_STATUS: std::sync::Mutex<&'static str> = std::sync::Mutex::new("app_status_loading_data");

/// Set the loader's status key (see `LOAD_STATUS`).
pub fn set_load_status(key: &'static str) {
    if let Ok(mut s) = LOAD_STATUS.lock() {
        *s = key;
    }
}

/// The loader's status key now.
pub fn load_status() -> &'static str {
    LOAD_STATUS.lock().map(|s| *s).unwrap_or("app_status_loading_data")
}

/// The running battle.
#[derive(Resource)]
pub struct Battle {
    pub world: World,
    /// The kingdom the views read (`0x5C0E50`): the local player's kingdom,
    /// or, without one, the kingdom an observer looks through (interface
    /// +0x1E0, `0x5C0E90`), [`NO_KINGDOM`] for none. An id outside the world
    /// is "no kingdom" to `aa_play::fog` and `aa_play::setup::relation`.
    pub local: KingdomId,
    pub selection: Vec<Unit>,
    /// Commands from input and HUD, applied on the next tick.
    pub pending: Vec<Command>,
    pub paused: bool,
    /// Game speed multiplier (the original's game speed setting).
    pub speed: f32,
    accumulator: f32,
    /// Events of every tick since the last frame, for HUD/sound/effects.
    pub frame_events: Vec<aa_play::Event>,
    /// The local player has no kingdom: a film's viewer (`[4610] ...
    /// observer`, FILM-PLAYBACK.md 8.2), or the mission-lost box's Observe
    /// (`RequestObserve` 0x60CB40 -> KKC_GamePlayerObserverOrderDecree
    /// 0x60EE60 -> 0x61EE60: ClearSelection, then the player's kingdom is
    /// set to none, 0x74B3B0(0) / 0x765E10(0)). [`Self::local_kingdom`] is
    /// what "no local kingdom" reads: the mission boxes, the observer buttons
    /// and the order gate; `local` is then the view kingdom.
    /// (Appended by the front-end owner.)
    pub observer: bool,
    /// The film being played (state 4, Session+0xE4). The world keeps its
    /// player after END_OF_FILM: Admin+0x26C = 0 only stops the reading
    /// (FILM-PLAYBACK.md 5.3).
    pub film: Option<FilmPlayer>,
    /// The film's notifications since the front end last read them
    /// ([`Self::take_film_events`]).
    film_events: Vec<FilmEvent>,
    /// The decree stream of a battle and its film (`aa_parity::recording`,
    /// `crate::film_record`); none while viewing a film. Dropping it closes the
    /// film (0x770500, leaving the state). (Appended by the recording owner.)
    pub recorder: Option<aa_parity::recording::Recorder>,
    /// Seconds to the next admin tick (Admin+0x28C, 1/16 s of real time, not
    /// scaled by the game speed).
    admin_clock: f32,
}

/// No kingdom: the value of [`Battle::local`] without a view kingdom.
pub const NO_KINGDOM: KingdomId = KingdomId::MAX;

impl Battle {
    pub fn new(world: World, local: KingdomId) -> Self {
        Self { world, local, selection: Vec::new(), pending: Vec::new(), paused: false, speed: 1.0, accumulator: 0.0, frame_events: Vec::new(), observer: false, film: None, film_events: Vec::new(), recorder: None, admin_clock: 0.0 }
    }

    /// The player's kingdom (0x5C0E30): none while observing.
    pub fn local_kingdom(&self) -> Option<KingdomId> {
        (!self.observer).then_some(self.local)
    }

    /// `0x5C0E50`: the local player's kingdom if it has one, else the
    /// kingdom the observer looks through (interface+0x1E0), else none: no
    /// fog, every object on the minimap, no resource bar.
    pub fn view_kingdom(&self) -> Option<KingdomId> {
        (self.local != NO_KINGDOM).then_some(self.local)
    }

    /// `0x5C0E90`: the observer's view kingdom; none while Session+0x168 is
    /// set (`FilmPlayer::view_locked`, the film header's +0x3C string).
    pub fn set_view_kingdom(&mut self, kingdom: Option<KingdomId>) {
        if self.observer {
            self.local = if self.view_locked() { NO_KINGDOM } else { kingdom.unwrap_or(NO_KINGDOM) };
        }
    }

    /// Session+0x168.
    pub fn view_locked(&self) -> bool {
        self.film.as_ref().is_some_and(|f| f.view_locked())
    }

    /// `0x61EE60`: ClearSelection, then the player's kingdom is none and the
    /// view has no kingdom (interface+0x1E0 is still 0).
    pub fn observe(&mut self) {
        self.selection.clear();
        self.pending.clear();
        self.observer = true;
        self.local = NO_KINGDOM;
    }

    /// Session+0xE4 == 4 (`0x766C70`).
    pub fn in_film(&self) -> bool {
        self.film.is_some()
    }

    /// The speed limits as exponents (`0x76B360` minimum, `0x76B3A0`
    /// maximum): SVars GameSpeedMin / GameSpeedMax, in film state (not
    /// networked here, Session+0xF4) GameSpeedFilmMin / GameSpeedFilmMax; the
    /// maximum also with GWorld+0x275 set.
    pub fn speed_limits(&self) -> (f32, f32) {
        let rule = |name: &str| self.world.data.rule_ci(name);
        let min = match self.in_film() {
            true => rule("GameSpeedFilmMin").unwrap_or(-4.0),
            false => rule("GameSpeedMin").unwrap_or(-2.0),
        };
        let max = match self.in_film() || aa_play::triggers::cutscene(&self.world) {
            true => rule("GameSpeedFilmMax").unwrap_or(4.0),
            false => rule("GameSpeedMax").unwrap_or(2.0),
        };
        (min, max)
    }

    /// `0x76F370`: the speed clamped to [`Self::speed_limits`].
    pub fn set_speed_exponent(&mut self, exponent: f32) {
        let (lo, hi) = self.speed_limits();
        self.speed = 2f32.powf(clamp_speed(exponent, lo, hi));
    }

    /// `0x76F530`, SpeedUp / SpeedDown: the exponent rounded to the nearest
    /// half, then half a step (`0x76F370` both times). Refused while
    /// GWorld+0x275 is set and for a non-host (none offline).
    pub fn speed_step(&mut self, dir: f32) {
        if aa_play::triggers::cutscene(&self.world) {
            return;
        }
        let (lo, hi) = self.speed_limits();
        self.speed = 2f32.powf(stepped_speed(self.speed.max(1e-6).log2(), dir, lo, hi));
    }

    /// The film's notifications since the last call (0x652020 reads them).
    pub fn take_film_events(&mut self) -> Vec<FilmEvent> {
        std::mem::take(&mut self.film_events)
    }

    /// The film's player on its world (OnGWorldReadyForState 0x7757F0, state
    /// 4): the viewer has no kingdom and the view none (`[4610]
    /// VWorldState::StartMission, observer`).
    pub fn attach_film(&mut self, player: FilmPlayer) {
        self.film = Some(player);
        self.film_events.clear();
        self.observe();
    }
}

/// The client's own profile folder, as the original's personal folder is its
/// own (`%USERDATA%`): the settings beside `data/`, its films in `data/Films/`.
pub fn own_profile() -> Option<std::path::PathBuf> {
    std::env::var_os("APPDATA").map(|a| std::path::PathBuf::from(a).join("AxisAlliesRust"))
}

/// Where the converted art of a loose data folder is kept: `<own profile>/art/<key>`,
/// one folder for each data folder, written by `research/film-sources/convert_overlay.py`
/// (the same exporter as the stock art). The profile's data itself is never written.
pub fn art_folder(data_folder: &std::path::Path) -> Option<std::path::PathBuf> {
    use sha2::{Digest, Sha256};
    let path = data_folder.to_string_lossy().replace('\\', "/").to_lowercase();
    let digest = Sha256::digest(path.trim_end_matches('/').as_bytes());
    let key: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
    Some(own_profile()?.join("art").join(key))
}

/// The float clamp of `0x76F370`.
fn clamp_speed(v: f32, lo: f32, hi: f32) -> f32 {
    if v < lo {
        lo
    } else if hi < v {
        hi
    } else {
        v
    }
}

/// The exponent after one `0x76F530` step: `floor(v / 0.5 + 0.5) * 0.5` plus
/// or minus 0.5, clamped.
fn stepped_speed(v: f32, dir: f32, lo: f32, hi: f32) -> f32 {
    clamp_speed((v / 0.5 + 0.5).floor() * 0.5 + 0.5 * dir, lo, hi)
}

/// The world and player of a film launch, opened before the loader starts
/// (`KKC_ResourceManagerTaskLoadFilm`, 0x5A3980). The loader draws this world's
/// map and hands both to the `Battle`.
#[derive(Resource)]
pub struct PendingFilm(pub std::sync::Mutex<Option<(World, FilmPlayer)>>);

/// Fixed 16 Hz simulation, decoupled from the frame rate.
pub fn step_battle(time: Res<Time>, battle: Option<ResMut<Battle>>) {
    let Some(mut battle) = battle else { return };
    battle.frame_events.clear();
    // an observer has no kingdom to give orders for (0x61EE60)
    if battle.local_kingdom().is_none() {
        battle.selection.clear();
        battle.pending.clear();
    }
    // Neither the pause nor the speed reaches a film: the original's film of a
    // game with both has no 73, 75, 76, 80 or 81, only 311s further apart in
    // records at a higher speed.
    if battle.paused {
        return;
    }
    battle.accumulator += time.delta_secs() * battle.speed;
    // The admin ticks that ran (0x755290 with 0x76AEE0): every 17th a checkpoint.
    battle.admin_clock += time.delta_secs();
    while battle.admin_clock >= aa_play::DT {
        battle.admin_clock -= aa_play::DT;
        if let Some(recorder) = battle.recorder.as_mut() {
            recorder.admin_tick();
        }
    }
    let mut steps = 0;
    while battle.accumulator >= aa_play::DT && steps < 8 {
        battle.accumulator -= aa_play::DT;
        let commands = std::mem::take(&mut battle.pending);
        // a film's world tick: its record, then the update (0x752AE0); the
        // viewer gives no commands (the order gate, input.rs)
        let b = &mut *battle;
        match b.film.as_mut() {
            Some(film) => b.film_events.extend(film.step(&mut b.world)),
            None => match b.recorder.as_mut() {
                Some(recorder) => recorder.step(&mut b.world, commands),
                None => b.world.step(commands),
            },
        }
        let events = std::mem::take(&mut battle.world.events);
        battle.frame_events.extend(events.iter().cloned());
        battle.world.events = events;
        steps += 1;
    }
    if steps == 8 {
        battle.accumulator = 0.0;
    }
}

/// Fraction of the way to the next tick, for smooth drawing between ticks.
pub fn tick_fraction(battle: &Battle) -> f32 {
    (battle.accumulator / aa_play::DT).clamp(0.0, 1.0)
}

/// Set together with `PendingSetup` by the staging screen's launch: the
/// computer general (SAI profile `IDS`, Data/SAI/*.tgi, e.g. what the
/// slot's StagingSelectSAI picked or the random pick for the nation) per
/// computer player, as (index into `SkirmishSetup::players`, profile ids),
/// for `aa_play::ai::set_profile` on that player's kingdom.
#[derive(Resource, Clone, Debug, Default)]
pub struct PendingAiProfiles(pub Vec<(usize, String)>);

/// Object animations the FX system started on actors (FXC
/// PLAY_OBJECT_ANIMATION, e.g. a tank's `recoil1` when it fires): actor ->
/// (the animation IDS, lower case, and the battle time it started). Written
/// by the effects (effects/mod.rs); battle_view can play the rig clip of
/// that name (case-insensitive, e.g. "Recoil1") once from that time.
/// (Appended by the effects and sound owner.)
#[derive(Resource, Default)]
pub struct ObjectAnimations(pub std::collections::HashMap<aa_play::ActorId, (String, f32)>);

#[cfg(test)]
mod tests {
    use super::*;

    /// 0x76F530 from 100% in a film ([O] 71, 50, 35, 25, 18 going down; 141,
    /// 200 going up), and the two ends of the film range.
    #[test]
    fn film_speed_steps_show_what_the_original_shows() {
        let percent = |v: f32| (2f32.powf(v) * 100.0).round() as i32;
        let mut v = 0.0;
        let mut down = Vec::new();
        for _ in 0..8 {
            v = stepped_speed(v, -1.0, -4.0, 4.0);
            down.push(percent(v));
        }
        assert_eq!(down, [71, 50, 35, 25, 18, 13, 9, 6]);
        assert_eq!(stepped_speed(-4.0, -1.0, -4.0, 4.0), -4.0);
        let mut v = 0.0;
        let mut up = Vec::new();
        for _ in 0..8 {
            v = stepped_speed(v, 1.0, -4.0, 4.0);
            up.push(percent(v));
        }
        assert_eq!(up, [141, 200, 283, 400, 566, 800, 1131, 1600]);
        assert_eq!(stepped_speed(4.0, 1.0, -4.0, 4.0), 4.0);
    }

    /// A game stops at 25% and 400% (GameSpeedMin / GameSpeedMax).
    #[test]
    fn game_speed_keeps_the_game_range() {
        assert_eq!(stepped_speed(-2.0, -1.0, -2.0, 2.0), -2.0);
        assert_eq!(stepped_speed(2.0, 1.0, -2.0, 2.0), 2.0);
        assert_eq!(clamp_speed(3.0, -2.0, 2.0), 2.0);
        assert_eq!(clamp_speed(-3.0, -2.0, 2.0), -2.0);
    }

    /// The step rounds to the nearest half first: floor(v / 0.5 + 0.5) * 0.5.
    #[test]
    fn speed_step_rounds_to_the_nearest_half() {
        assert_eq!(stepped_speed(0.3, 1.0, -4.0, 4.0), 1.0);
        assert_eq!(stepped_speed(0.2, 1.0, -4.0, 4.0), 0.5);
        assert_eq!(stepped_speed(0.25, -1.0, -4.0, 4.0), 0.0);
    }
}
