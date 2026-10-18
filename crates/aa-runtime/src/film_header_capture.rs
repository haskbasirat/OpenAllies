//!5A26E0 current GWorld -> already constructed MapHeader coordinator.
//! The source and destination are distinct actual owners. The mandatory
//!510EB0/5A17D0 producers and Campaign manager services are never empty defaults.
use aa_mapworld::header::{HeaderEntry, MapHeader, Rule, WorldParameters};
pub type Result<T> = std::result::Result<T, String>;
pub trait World {
    type Campaign: Copy;
    ///6FB1F0 actual current local FILETIME [low,high], not a saved timestamp.
    fn current_local_filetime(&mut self) -> Result<[u32; 2]>;
    fn mode_10(&mut self) -> Result<u32>;
    ///World+14 is the actual immutable GameType definition reference. Its
    ///serializer uses its IDS; this representation requires that same identity.
    fn game_type_14(&mut self) -> Result<Vec<u16>>;
    ///41D130 actual source World+18 -> target Rules+C, including native partial
    ///effects. Do not substitute a precomputed detached rules clone at startup.
    fn copy_rules_18(&mut self, target: &mut Vec<Rule>) -> Result<()>;
    fn rule_extra_24(&mut self) -> Result<u32>;
    fn capture_parameters_510eb0(&mut self, target: &mut WorldParameters) -> Result<()>;
    fn extent_3c(&mut self) -> Result<u32>;
    fn extent_40(&mut self) -> Result<u32>;
    fn current_campaign_4f1310(&mut self) -> Result<Option<Self::Campaign>>;
    ///411790 is always called, even when current_campaign returned null. Its
    ///create flag is the low-byte boolean of that result, not an inferred mode.
    fn lookup_campaign_411790(&mut self, create: bool) -> Result<Option<Self::Campaign>>;
    fn campaign_name_08(&mut self, current: Self::Campaign) -> Result<Vec<u16>>;
    fn campaign_index_14(&mut self, looked_up: Self::Campaign) -> Result<u32>;
    fn capture_entries_5a17d0(
        &mut self,
        parameters: &mut WorldParameters,
        target: &mut Vec<HeaderEntry>,
    ) -> Result<()>;
}
///The target must be a native-constructed header (including display extent
///storage), not the original map's parsed header. Rules+18 is a nonserialized
///native word, so it has its own required actual destination alongside it.
pub fn capture<W: World>(
    header: &mut MapHeader,
    rule_extra: &mut u32,
    world: &mut W,
) -> Result<()> {
    let time = world.current_local_filetime()?;
    header.words_18_1c[1] = time[1]; //5A26F7 high word FIRST.
    header.words_18_1c[0] = time[0];
    header.rules.mode = world.mode_10()?;
    header.rules.game_type = world.game_type_14()?;
    world.copy_rules_18(&mut header.rules.rules)?;
    *rule_extra = world.rule_extra_24()?;
    world.capture_parameters_510eb0(&mut header.parameters)?;
    let width = world.extent_3c()?;
    header
        .display_extents_bits
        .as_mut()
        .ok_or("constructed film header display extent storage is absent")?[0] = width;
    let height = world.extent_40()?;
    header
        .display_extents_bits
        .as_mut()
        .ok_or("constructed film header display extent storage is absent")?[1] = height;
    let current = world.current_campaign_4f1310()?;
    let looked_up = world.lookup_campaign_411790(current.is_some())?;
    if let (Some(current), Some(looked_up)) = (current, looked_up) {
        header.string_3c = world.campaign_name_08(current)?;
        header.word_40 = Some(world.campaign_index_14(looked_up)?);
    } else {
        header.string_3c.clear();
        header.word_40 = Some(u32::MAX);
    }
    world.capture_entries_5a17d0(&mut header.parameters, &mut header.entries)
}
