//! One mutable current SVar owner for the controlled fresh single-World path.
//! 474C50 applies registered defaults before4902B1 snapshots them;4F2826
//! resets from that snapshot, then4FC859/512200 publishes standalone RULES.
//! This is not an imported profile or a second per-view scalar snapshot.
#[cfg(test)]
use crate::saved_action_settings;
use crate::{catalog::Catalog, combat_value_source_context::SourceFpContext};
use aa_mapworld::WorldInventory;
use aa_numeric::Precision;
use aa_rules::resources::SourceIdentity;

#[cfg(test)]
#[path = "load_view_variables_tests.rs"]
mod tests;

/// Explicit scope only. Unknown/profile overrides must bind their real owner;
/// this enum does not itself claim that native application startup was traced.
pub enum NativeOverrides {
    Unbound,
    ControlledNone,
}

#[cfg(test)]
struct Current {
    fog_of_war: i32,
    cheat_fog: u8,
    fast_detection_zones: u8,
}
#[cfg(test)]
impl Current {
    fn source_reset(fog_default: i32, fast_source: u8) -> Self {
        //474C50/40E110 use the registered enum bounds before snapshot/reset.
        Self {
            fog_of_war: fog_default.clamp(0, 2),
            cheat_fog: 0,
            fast_detection_zones: fast_source,
        }
    }
    fn assign_fog(&mut self, value: i32) {
        self.fog_of_war = value.clamp(0, 2)
    }
    fn assign_cheat(&mut self, value: u8) {
        self.cheat_fog = value
    }
    fn assign_fast(&mut self, value: u8) {
        self.fast_detection_zones = value
    }
}

/// The same owner moves into the active World. No Clone/Default/Deserialize or
/// raw-word constructor is provided. The caller must retain it across all views
/// and native SVar mutations. Simultaneous Worlds must share this process owner.
pub struct State {
    store: crate::canonical_svars::Store,
    sources: Vec<SourceIdentity>,
    source_context: SourceFpContext,
    rules_source: Option<aa_mapworld::SourceIdentity>,
}

fn text_error(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn source_context(catalog: &Catalog<'_>) -> Result<SourceFpContext, String> {
    catalog.require_source_owner()?;
    let bank = catalog.definition_bank.try_borrow().map_err(text_error)?;
    let source = bank.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources {
        return Err("view SVar source Bank/catalog identities differ".into());
    }
    Ok(source.context().clone())
}

impl State {
    pub fn reset_from_fresh_source(
        catalog: &Catalog<'_>,
        overrides: NativeOverrides,
    ) -> Result<Self, String> {
        if !matches!(overrides, NativeOverrides::ControlledNone) {
            return Err("view SVar profile/console override owner is unbound".into());
        }
        let context = source_context(catalog)?;
        crate::combat_value_source_context::bind_numeric(catalog, &context)?;
        let pc = context.precision();
        let store = crate::canonical_svars::from_fresh_source(catalog, pc)?;
        // Validate closed gameplay accessors once, so these infallible views
        // cannot encounter a missing or wrongly typed current entry later.
        for (name, kind) in [
            ("FogOfWar", 3),
            ("CheatFog", 1),
            ("ZoneBuilderFastDetectionZones", 1),
            ("ModifierAttributeClampMultiply", 2),
            ("ResourceStockpileDefault", 2),
            ("GameSpeed", 2),
            ("EconomyStockpileResourceMax", 2),
            ("CityVictoryRatio", 3),
            ("StartingExperience", 2),
            ("SupplyProvideUnderSiege", 1),
            ("SupplyStockpileRechargeRate", 2),
        ] {
            store.require_kind(name, kind)?;
        }
        for field in crate::load_combat_variables::FIELDS {
            store.require_kind(field.name(), 2)?;
        }
        Ok(Self {
            store,
            sources: catalog.actors.sources.clone(),
            source_context: context,
            rules_source: None,
        })
    }

    pub fn validate_catalog(&self, catalog: &Catalog<'_>) -> Result<(), String> {
        let bank = catalog.definition_bank.try_borrow().map_err(text_error)?;
        self.validate_in_bank(catalog, &bank)
    }

    /// The actual Bank moves into Staged; validate that owner directly rather
    /// than reconstructing a Catalog Bank or making a second source cache.
    pub fn validate_in_bank(
        &self,
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
    ) -> Result<(), String> {
        catalog.require_source_owner()?;
        let source = bank.combat_source()?;
        source.completed_means()?;
        let actual = source.context();
        if self.sources != catalog.actors.sources
            || source.source_identities() != self.sources
            || actual.precision() != self.source_context.precision()
            || actual.reference_startup() != self.source_context.reference_startup()
        {
            return Err("view SVar source owner/precision/startup context differs".into());
        }
        crate::combat_value_source_context::bind_numeric(catalog, &self.source_context)?;
        Ok(())
    }

    /// The reset owner must already be retained by the caller. A failed rule
    /// load/publication leaves its previous current words intact.
    pub fn publish_world_rules(
        &mut self,
        catalog: &Catalog<'_>,
        inventory: &WorldInventory,
    ) -> Result<(), String> {
        let owner = crate::load_world_rules::Owner::load(catalog, inventory)?;
        self.publish_loaded_rules(catalog, &owner)
    }

    ///512200 publication from the SAME retained, registration-ordered native
    ///WorldRules owner. The inventory re-decoder above remains diagnostic.
    ///Only selected actual SVar cells are written; current CV/other words stay
    ///in this same owner and are never reconstructed from source snapshots.
    pub fn publish_loaded_rules(
        &mut self,
        catalog: &Catalog<'_>,
        owner: &crate::load_world_rules::Owner,
    ) -> Result<(), String> {
        self.validate_catalog(catalog)?;
        {
            let bank = catalog.definition_bank.try_borrow().map_err(text_error)?;
            owner.validate_in_bank(catalog, &bank)?;
        }
        if owner.source_precision() != self.source_context.precision() {
            return Err("current SVar and retained WorldRules source PC differ".into());
        }
        //512200 visits every retained descriptor in native registration order.
        //Each setter writes the SAME entry that gameplay and76D820 borrow.
        for row in owner.records() {
            self.store.publish_rule(row)?;
        }
        self.rules_source = Some(owner.rules_source().clone());
        Ok(())
    }
    ///4F8137 and4F2BFF copy the reset current SVar to World278 before map
    ///publication. The same current cell remains owned here afterward.
    pub(crate) fn constructor_game_speed(&self, catalog: &Catalog<'_>) -> Result<u32, String> {
        self.validate_catalog(catalog)?;
        if self.rules_source.is_some() {
            return Err("World278 constructor read occurred after Rule publication".into());
        }
        Ok(self.store.word("GameSpeed"))
    }
    pub fn require_published(&self) -> Result<(), String> {
        self.rules_source
            .as_ref()
            .map(|_| ())
            .ok_or_else(|| "view SVar WorldRules publication is incomplete".into())
    }
    #[cfg(test)]
    pub(crate) fn store_for_evidence(&self) -> &crate::canonical_svars::Store {
        &self.store
    }
    pub fn registrations(&self) -> impl Iterator<Item = Registration<'_>> {
        self.store.registrations()
    }
    pub fn current(&self, index: usize) -> Result<CurrentValue<'_>, String> {
        self.store.current(index)
    }
    pub fn fog_integer(&self) -> i32 {
        self.store.word("FogOfWar") as i32
    }
    pub fn combat_values(&self) -> crate::load_combat_variables::Values<'_> {
        crate::load_combat_variables::Values::borrow(&self.store)
    }
    pub fn assign_combat_bits(&mut self, f: crate::load_combat_variables::Field, v: u32) {
        self.store.assign_word_raw(f.name(), v)
    }
    pub fn modifier_clamp_multiply_bits(&self) -> u32 {
        self.store.word("ModifierAttributeClampMultiply")
    }
    pub fn assign_modifier_clamp_multiply_bits(&mut self, v: u32) {
        self.store
            .assign_word_raw("ModifierAttributeClampMultiply", v)
    }
    pub fn stockpile_default_bits(&self) -> u32 {
        self.store.word("ResourceStockpileDefault")
    }
    pub fn stockpile_maximum_bits(&self) -> u32 {
        self.store.word("EconomyStockpileResourceMax")
    }
    pub fn assign_stockpile_maximum_bits(&mut self, v: u32) {
        self.store.assign_word_raw("EconomyStockpileResourceMax", v)
    }
    pub fn city_victory_ratio(&self) -> i32 {
        self.store.word("CityVictoryRatio") as i32
    }
    pub fn starting_experience_bits(&self) -> u32 {
        self.store.word("StartingExperience")
    }
    pub fn supply_provide_under_siege(&self) -> bool {
        *self.store.bool_ref("SupplyProvideUnderSiege") != 0
    }
    pub fn assign_supply_provide_under_siege(&mut self, v: u8) {
        self.store.assign_bool_raw("SupplyProvideUnderSiege", v)
    }
    pub fn supply_recharge_rate_bits(&self) -> u32 {
        self.store.word("SupplyStockpileRechargeRate")
    }
    pub fn assign_supply_recharge_rate_bits(&mut self, v: u32) {
        self.store.assign_word_raw("SupplyStockpileRechargeRate", v)
    }
    pub fn assign_starting_experience_bits(&mut self, v: u32) {
        self.store.assign_word_raw("StartingExperience", v)
    }
    pub fn assign_city_victory_ratio(&mut self, v: i32) {
        self.store.assign_word_raw("CityVictoryRatio", v as u32)
    }
    pub fn assign_stockpile_bits(&mut self, v: u32) {
        self.store.assign_word_raw("ResourceStockpileDefault", v)
    }
    pub fn fog_enabled(&self) -> bool {
        self.fog_integer() != 0
    }
    pub fn cheat_fog_byte(&self) -> u8 {
        *self.cheat_fog_word()
    }
    pub fn cheat_fog_word(&self) -> &u8 {
        self.store.bool_ref("CheatFog")
    }
    pub fn cheat_fog_enabled(&self) -> bool {
        self.cheat_fog_byte() != 0
    }
    pub fn fast_detection_zones_byte(&self) -> u8 {
        *self.store.bool_ref("ZoneBuilderFastDetectionZones")
    }
    pub fn fast_detection_zones(&self) -> bool {
        self.fast_detection_zones_byte() != 0
    }
    pub fn assign_fog_integer(&mut self, v: i32) {
        self.store.assign_word_raw("FogOfWar", v.clamp(0, 2) as u32)
    }
    pub fn assign_cheat_fog(&mut self, v: u8) {
        self.store.assign_bool_raw("CheatFog", v)
    }
    pub fn assign_fast_detection_zones(&mut self, v: u8) {
        self.store
            .assign_bool_raw("ZoneBuilderFastDetectionZones", v)
    }
}
///Borrowed current cells, read only when a native consumer reaches its getter.
pub enum CurrentValue<'a> {
    Bool(&'a u8),
    Fixed(&'a u32),
    Int(&'a u32),
    CString(&'a [u16]),
    Point3(&'a [u32; 3]),
}
pub struct Registration<'a> {
    pub(crate) ordinal: u32,
    pub(crate) name: &'a str,
    pub(crate) kind: u32,
}
impl Registration<'_> {
    pub fn ordinal(&self) -> u32 {
        self.ordinal
    }
    pub fn name(&self) -> &str {
        self.name
    }
    pub fn kind(&self) -> u32 {
        self.kind
    }
}
