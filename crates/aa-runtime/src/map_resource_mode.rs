//! MapResource mode and loaded-header WorldRuleMode are separate native values.
//! `59E520` follows Resource+90 -> loaded Header+4 -> WorldRules+4; it does
//! not read a selected launch creator's conditions or Session+F8.
use aa_mapworld::header::MapHeader;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModeFlags {
    pub name: &'static str,
    /// `4981A0` payload+0, read by `511860` through `59E520`.
    pub saved_state: bool,
    pub metagame: bool,
}

/// Actual `4981A0` initialization, reached from CRT9CCA10 ->4994D0.
/// Seven is a native semantic enumeration, not a storage/resource budget.
pub fn world_rule_mode(mode: u32) -> Result<ModeFlags, String> {
    let (name, saved_state, metagame) = match mode {
        0 => ("playable", false, false),
        1 => ("cutscene", false, false),
        2 => ("savegame", true, false),
        3 => ("film", false, false),
        4 => ("savegame_metagame_rts", true, true),
        5 => ("savegame_metagame_map", true, true),
        6 => ("savegame_campaign", true, false),
        _ => {
            return Err("native WorldRuleMode access is outside its initialized enumeration".into());
        }
    };
    Ok(ModeFlags {
        name,
        saved_state,
        metagame,
    })
}

/// `59E520/511860` at the reached read. Borrow the actual retained loaded
/// Resource+90 header; the selected launch MapSettings is a different owner.
pub fn world_start_flag(header: &MapHeader) -> Result<bool, String> {
    Ok(world_rule_mode(header.rules.mode)?.saved_state)
}

/// Native LoadGameTask+20. Raw kinds other than1/2 take the normal branch;
/// representing them is not authority to invent a task or complete a launch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadGameTaskKind(u32);
impl LoadGameTaskKind {
    pub fn from_native_word(word: u32) -> Self {
        Self(word)
    }
    pub fn word(self) -> u32 {
        self.0
    }

    /// Task Run5A4280 ->5A3E20/5A3F20/5A3EB0 ->LoadMap5A2B90.
    /// The resulting argument is stored temporarily at Resource+A8 by5A2BF1.
    /// Normal return restores its earlier word; this is not a persistent
    /// replacement for the actual Resource owner or the complete task runner.
    pub fn map_resource_load_mode(self) -> u32 {
        match self.0 {
            1 => 2,
            2 => 4,
            _ => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScenarioTask {
    Load(LoadGameTaskKind),
    Film { editor: bool },
    MissingScenario,
}

/// The scenario branch of775EE0, after its required task/sync/reset prefix.
/// `captured_game_type` is read at775F69 before the diagnostic; the two cached
/// comparisons to3/5 survive that callback. `current_game_type` is the actual
/// later Session+54 word read by776006/77606D (no callback between these reads).
/// A caller with no intervening mutation supplies the same word twice.
/// Call only after selecting launch mode2; other launch paths are separate.
pub fn scenario_task(
    captured_game_type: u32,
    current_game_type: u32,
    scenario_present: bool,
) -> ScenarioTask {
    if !scenario_present && current_game_type != 4 {
        return ScenarioTask::MissingScenario;
    }
    if current_game_type == 4 {
        return ScenarioTask::Film {
            editor: captured_game_type == 3,
        };
    }
    let word = if captured_game_type == 3 {
        1
    } else if captured_game_type == 5 {
        2
    } else {
        0
    };
    ScenarioTask::Load(LoadGameTaskKind(word))
}
