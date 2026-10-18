//! User variables (`UVars`, manager `0xB39938`, registered in `0x41F9D0`)
//! as far as the options dialog reads and writes them.
//!
//! * Registration `(name, default[, max, min], flags)`: range variables
//!   clamp on write (`0x40E0A0` for floats, inline for ints); writes go
//!   through the guard `0x4B29B0`.
//! * The defaults copy (`0xB39960`) is the code default overlaid by
//!   `Data/UVars.tgi` (INFERRED from the options dialog's Defaults, which
//!   read it). OK saves the user values against it to the AVar
//!   `UserUVarsFilename` (User/UVars.tgi); this port never writes the
//!   original profile: the caller persists [`UVars::overrides`] in its own
//!   settings file instead.
//!
//! Code defaults and ranges below come from the registration calls
//! (research/phase8-ui/scratch-options/uvars.tsv); float range registers
//! the disassembly left symbolic (`ebp` = 1.0, `ebx` = 0) follow the
//! options-dialog research.

use crate::tgi::TgiStore;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Bool,
    Int,
    Float,
    Str,
}

/// One registered variable.
#[derive(Clone, Copy, Debug)]
pub struct Def {
    pub name: &'static str,
    pub kind: Kind,
    /// Code default (text).
    pub default: &'static str,
    /// (min, max) for range variables.
    pub range: Option<(f64, f64)>,
}

const fn d(name: &'static str, kind: Kind, default: &'static str, range: Option<(f64, f64)>) -> Def {
    Def { name, kind, default, range }
}

use Kind::*;

/// The variables the options dialog uses.
pub const DEFS: &[Def] = &[
    // message logs
    d("MessageLogMaximumLines", Int, "50", None),
    d("UIMessageLogCycleTimeout", Float, "1", None),
    d("MetaGameUIMessageLogMaximumLines", Int, "10", Some((1.0, 15.0))),
    d("UIMessageFilterHero", Bool, "true", None),
    d("UIMessageFilterCompany", Bool, "true", None),
    d("UIMessageFilterRegiment", Bool, "true", None),
    d("UIMessageFilterBuilding", Bool, "true", None),
    d("UIMessageFilterGeneric", Bool, "true", None),
    d("UIMessageFilterTutorial", Bool, "true", None),
    // HUD (registration 0x41F9D0: slots 0x2C, 0x7D..0x7F, 0x80, 0x82)
    d("CameraInformation", Bool, "false", None),
    d("UIRecruitFloaterIsOpen", Bool, "true", None),
    d("UIGroupFloaterIsOpen", Bool, "true", None),
    d("UIAbilityFloaterIsOpen", Bool, "true", None),
    d("UIColorHighlight", Str, "1,1,0", None),
    d("UIColorOptionValid", Str, "0,1,0", None),
    d("UIColorOptionInvalid", Str, "1,0,0", None),
    // game play
    d("UIMessageLogMaximumLines", Int, "5", Some((1.0, 15.0))),
    d("UIMessageLogFadeTime", Int, "10000", Some((1.0, 20000.0))),
    d("CameraScrollSpeed", Float, "2", Some((1.0, 20.0))),
    d("CameraScrollAcceleration", Float, "0.25", Some((0.0, 20.0))),
    d("CameraScrollSpeedMax", Float, "20", Some((1.0, 30.0))),
    d("CameraScaleZoom", Float, "0.5", None),
    d("CameraRotate", Bool, "false", None),
    d("UIToolTipsShow", Bool, "true", None),
    d("UIMessageBoxConfirmation", Bool, "true", None),
    d("UIStructureAutoPlacement", Bool, "false", None),
    d("ViewShowVFXObjectName", Bool, "true", None),
    d("UIEventFilterSiege", Bool, "true", None),
    d("UIEventFilterEngagement", Bool, "true", None),
    d("UIEventFilterEngagementMessages", Bool, "false", None),
    d("UIEventFilterProduction", Bool, "true", None),
    d("ViewShowDamage", Bool, "false", None),
    d("ViewShowZoneFriendlySupply", Bool, "false", None),
    d("ViewShowElapsedGameTime", Bool, "false", None),
    d("HotkeyConfiguration", Str, "classic_hotkey_config", None),
    // video
    d("ResolutionX", Int, "800", None),
    d("ResolutionY", Int, "600", None),
    d("RenderDrawFootsteps", Bool, "true", None),
    d("TerrainExtraQuality", Bool, "true", None),
    d("TerrainQuadSize", Int, "2", Some((1.0, 2.0))),
    d("TerrainTrilinearFilter", Bool, "true", None),
    d("RenderLODShadows", Int, "1", Some((0.0, 3.0))),
    d("ParticleGeneratePCG", Float, "1", Some((0.0, 1.0))),
    d("UnitFogSimplified", Bool, "false", None),
    d("RendererUseHighQualityTextures", Bool, "true", None),
    d("PerformanceDisableDecals", Bool, "false", None),
    // audio
    d("AudioMainVolume", Float, "1", Some((0.0, 1.0))),
    d("Audio2DVolume", Float, "1", Some((0.0, 1.0))),
    d("Audio3DVolume", Float, "1", Some((0.0, 1.0))),
    d("AudioSpeechVolume", Float, "1", Some((0.0, 1.0))),
    d("AudioMusicVolume", Float, "1", Some((0.0, 1.0))),
    d("UnitAcknowledgement", Bool, "true", None),
    d("UIEventFilterChatSound", Bool, "true", None),
    // network
    d("NetworkReportGameInProgress", Bool, "true", None),
    d("AFKTime", Int, "20", None),
    d("UIMessageFilterBuddy", Bool, "true", None),
    d("UIMessageFilterChat", Bool, "true", None),
    d("UIMessageFilterAFK", Bool, "true", None),
    d("UIMessageFilterPlayerJoinLeave", Bool, "true", None),
    d("NetworkFilterSameVersion", Bool, "true", None),
    d("NetworkFilterSameGroupRoom", Bool, "false", None),
    d("NetworkFilterFullGames", Bool, "true", None),
    d("NetworkFilterPlayingGames", Bool, "false", None),
    d("NetworkFilterSameLanguage", Bool, "false", None),
    d("NetworkAutoRefresh", Bool, "false", None),
    d("NetworkRefreshTime", Int, "30", None),
    // metagame
    d("UIMessageFilterMetaGameGeneric", Bool, "true", None),
    d("UIMessageFilterMetaGameMovement", Bool, "false", None),
    d("UIMessageFilterMetaGamePurchases", Bool, "true", None),
    d("UIMessageFilterMetaGameBattles", Bool, "true", None),
    d("MetaGameViewShowVFXMovement", Bool, "false", None),
    d("MetaGameViewShowVFXObjectName", Bool, "false", None),
    d("MetaGameViewShowVFXArrows", Bool, "true", None),
    d("MetaGameUIShowHelpPopup", Bool, "true", None),
    d("MetaGameUIFastMGAI", Bool, "true", None),
    d("MetaGameUIComputerQuickBattle", Bool, "false", None),
    d("MetaGameUIPurchaseConfirm", Bool, "true", None),
];

/// Parse a TGI number (`80%` = 0.8).
pub fn parse_number(s: &str) -> Option<f64> {
    let t = s.trim().trim_end_matches('f');
    match t.strip_suffix('%') {
        Some(p) => p.trim().parse::<f64>().ok().map(|v| v / 100.0),
        None => t.parse::<f64>().ok(),
    }
}

#[derive(Clone, Debug, Default)]
pub struct UVars {
    defs: HashMap<String, Def>,
    /// Defaults copy (`0xB39960`), by lower-case name.
    defaults: HashMap<String, String>,
    /// User values (`0xB39938`).
    values: HashMap<String, String>,
}

impl UVars {
    /// Code defaults overlaid by `Data/UVars.tgi`.
    pub fn load(store: &mut TgiStore) -> Self {
        let mut v = UVars::default();
        for def in DEFS {
            v.defs.insert(def.name.to_ascii_lowercase(), *def);
            v.defaults.insert(def.name.to_ascii_lowercase(), def.default.to_string());
        }
        if let Ok(node) = store.node("UVars.tgi:Vars") {
            for f in &node.fields {
                if let Some(raw) = &f.raw {
                    let key = f.name.to_ascii_lowercase();
                    if v.defs.contains_key(&key) {
                        v.defaults.insert(key, crate::node::unquote(raw).trim().to_string());
                    }
                }
            }
        }
        v.values = v.defaults.clone();
        v
    }

    fn raw(&self, name: &str) -> Option<&str> {
        self.values.get(&name.to_ascii_lowercase()).map(String::as_str)
    }

    fn raw_default(&self, name: &str) -> Option<&str> {
        self.defaults.get(&name.to_ascii_lowercase()).map(String::as_str)
    }

    pub fn def(&self, name: &str) -> Option<Def> {
        self.defs.get(&name.to_ascii_lowercase()).copied()
    }

    fn to_bool(s: &str) -> bool {
        s.trim().eq_ignore_ascii_case("true") || s.trim() == "1"
    }

    pub fn get_bool(&self, name: &str) -> bool {
        self.raw(name).map(Self::to_bool).unwrap_or(false)
    }
    pub fn get_f32(&self, name: &str) -> f32 {
        self.raw(name).and_then(parse_number).unwrap_or(0.0) as f32
    }
    pub fn get_i32(&self, name: &str) -> i32 {
        self.raw(name).and_then(parse_number).unwrap_or(0.0) as i32
    }
    /// A `point3` variable (r,g,b); missing components are 0.
    pub fn get_point3(&self, name: &str) -> [f32; 3] {
        let mut out = [0.0f32; 3];
        for (i, part) in self.raw(name).unwrap_or("").split(',').take(3).enumerate() {
            out[i] = parse_number(part.trim()).unwrap_or(0.0) as f32;
        }
        out
    }
    pub fn get_str(&self, name: &str) -> String {
        self.raw(name).unwrap_or("").to_string()
    }
    pub fn default_bool(&self, name: &str) -> bool {
        self.raw_default(name).map(Self::to_bool).unwrap_or(false)
    }
    pub fn default_f32(&self, name: &str) -> f32 {
        self.raw_default(name).and_then(parse_number).unwrap_or(0.0) as f32
    }
    pub fn default_i32(&self, name: &str) -> i32 {
        self.raw_default(name).and_then(parse_number).unwrap_or(0.0) as i32
    }
    pub fn default_str(&self, name: &str) -> String {
        self.raw_default(name).unwrap_or("").to_string()
    }

    /// Registered range (min, max).
    pub fn range(&self, name: &str) -> Option<(f32, f32)> {
        self.def(name).and_then(|d| d.range).map(|(a, b)| (a as f32, b as f32))
    }

    pub fn set_bool(&mut self, name: &str, v: bool) {
        self.values.insert(name.to_ascii_lowercase(), if v { "true" } else { "false" }.into());
    }
    /// Float write with the range clamp (`0x40E0A0`).
    pub fn set_f32(&mut self, name: &str, v: f32) {
        let v = match self.range(name) {
            Some((lo, hi)) => v.clamp(lo, hi),
            None => v,
        };
        self.values.insert(name.to_ascii_lowercase(), format!("{v}"));
    }
    /// Int write with the range clamp.
    pub fn set_i32(&mut self, name: &str, v: i32) {
        let v = match self.range(name) {
            Some((lo, hi)) => v.clamp(lo as i32, hi as i32),
            None => v,
        };
        self.values.insert(name.to_ascii_lowercase(), format!("{v}"));
    }
    pub fn set_str(&mut self, name: &str, v: &str) {
        self.values.insert(name.to_ascii_lowercase(), v.to_string());
    }

    /// Values that differ from the defaults copy, by registered name.
    pub fn overrides(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        for def in DEFS {
            let k = def.name.to_ascii_lowercase();
            let (v, d) = (self.values.get(&k), self.defaults.get(&k));
            if v.is_some() && v != d {
                let same_number = match (v.and_then(|x| parse_number(x)), d.and_then(|x| parse_number(x))) {
                    (Some(a), Some(b)) => (a - b).abs() < 1e-9,
                    _ => false,
                };
                if !same_number {
                    out.insert(def.name.to_string(), v.cloned().unwrap_or_default());
                }
            }
        }
        out
    }

    /// Load saved user values (unknown names are ignored).
    pub fn apply_overrides(&mut self, map: &BTreeMap<String, String>) {
        for (k, v) in map {
            let key = k.to_ascii_lowercase();
            if self.defs.contains_key(&key) {
                self.values.insert(key, v.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_numbers() {
        assert_eq!(parse_number("80%"), Some(0.8));
        assert_eq!(parse_number("0.5"), Some(0.5));
        assert_eq!(parse_number(" 10000 "), Some(10000.0));
    }
}
