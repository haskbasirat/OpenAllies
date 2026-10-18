//!5A26E0 over actual fresh header fields. Unlike the bounded decoded-DTO
//!diagnostic adapter, this needs no MapHeader/chunk SourceIdentity and can
//!retain typed definition references and the complete localized parameter owner.
pub type Result<T> = std::result::Result<T, String>;
pub trait Destination {
    type GameType;
    type Rules;
    type Parameters;
    type Entries;
    fn set_time_high(&mut self, value: u32);
    fn set_time_low(&mut self, value: u32);
    fn set_rule_mode(&mut self, value: u32);
    fn set_game_type(&mut self, value: Self::GameType);
    fn rules(&mut self) -> &mut Self::Rules;
    fn set_rule_extra(&mut self, value: u32);
    fn parameters(&mut self) -> &mut Self::Parameters;
    fn set_width(&mut self, value: u32);
    fn set_height(&mut self, value: u32);
    fn set_campaign_name_3c(&mut self, value: Vec<u16>);
    fn set_campaign_index_40(&mut self, value: u32);
    fn parameters_and_entries(&mut self) -> (&mut Self::Parameters, &mut Self::Entries);
}
pub trait Source<D: Destination> {
    type Campaign;
    fn current_local_filetime(&mut self) -> Result<[u32; 2]>;
    fn mode_10(&mut self) -> Result<u32>;
    fn game_type_14(&mut self) -> Result<D::GameType>;
    fn copy_rules_18(&mut self, target: &mut D::Rules) -> Result<()>;
    fn rule_extra_24(&mut self) -> Result<u32>;
    fn capture_parameters_510eb0(&mut self, target: &mut D::Parameters) -> Result<()>;
    fn extent_3c(&mut self) -> Result<u32>;
    fn extent_40(&mut self) -> Result<u32>;
    fn current_campaign_4f1310(&mut self) -> Result<Option<Self::Campaign>>;
    fn lookup_campaign_411790(&mut self, create: bool) -> Result<Option<Self::Campaign>>;
    fn campaign_name_08(&mut self, current: &Self::Campaign) -> Result<Vec<u16>>;
    fn campaign_index_14(&mut self, looked_up: &Self::Campaign) -> Result<u32>;
    fn capture_entries_5a17d0(
        &mut self,
        parameters: &mut D::Parameters,
        entries: &mut D::Entries,
    ) -> Result<()>;
}
///Each destination method stores immediately in the actual retained owner.
///No destination/store clone or final commit is made; a child failure retains
///all earlier stores. Complete510EB0 and Campaign services remain mandatory.
pub fn capture<D: Destination, S: Source<D>>(target: &mut D, source: &mut S) -> Result<()> {
    let time = source.current_local_filetime()?;
    target.set_time_high(time[1]);
    target.set_time_low(time[0]);
    target.set_rule_mode(source.mode_10()?);
    target.set_game_type(source.game_type_14()?);
    source.copy_rules_18(target.rules())?;
    target.set_rule_extra(source.rule_extra_24()?);
    source.capture_parameters_510eb0(target.parameters())?;
    target.set_width(source.extent_3c()?);
    target.set_height(source.extent_40()?);
    let current = source.current_campaign_4f1310()?;
    let looked_up = source.lookup_campaign_411790(current.is_some())?;
    if let (Some(current), Some(looked_up)) = (current, looked_up) {
        target.set_campaign_name_3c(source.campaign_name_08(&current)?);
        target.set_campaign_index_40(source.campaign_index_14(&looked_up)?);
    } else {
        target.set_campaign_name_3c(Vec::new());
        target.set_campaign_index_40(u32::MAX);
    }
    let (parameters, entries) = target.parameters_and_entries();
    source.capture_entries_5a17d0(parameters, entries)
}
