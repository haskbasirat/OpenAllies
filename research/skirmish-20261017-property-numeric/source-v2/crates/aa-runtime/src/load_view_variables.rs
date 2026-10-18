//! One mutable current SVar owner for the controlled fresh single-World path.
//! 474C50 applies registered defaults before4902B1 snapshots them;4F2826
//! resets from that snapshot, then4FC859/512200 publishes standalone RULES.
//! This is not an imported profile or a second per-view scalar snapshot.
use crate::{
    catalog::Catalog, combat_value_source_context::SourceFpContext, saved_action_settings,
};
use aa_mapworld::WorldInventory;
use aa_numeric::Precision;
use aa_rules::{native::TypedValue, resources::SourceIdentity};

#[cfg(test)]
#[path = "load_view_variables_tests.rs"]
mod tests;

/// Explicit scope only. Unknown/profile overrides must bind their real owner;
/// this enum does not itself claim that native application startup was traced.
pub enum NativeOverrides {
    Unbound,
    ControlledNone,
}

struct Current {
    fog_of_war: i32,
    cheat_fog: u8,
    fast_detection_zones: u8,
}
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
    current: Current,
    combat: crate::load_combat_variables::Values,
    modifier_clamp_multiply: u32,
    stockpile_default110: u32,
    stockpile_maximum1b8: u32,
    city_victory_ratio130: i32,
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

fn declared_integer(catalog: &Catalog<'_>, name: &str, pc: Precision) -> Result<i32, String> {
    let settings = catalog
        .settings
        .as_ref()
        .ok_or("view SVar Settings owner missing")?;
    let rows = settings
        .declarations
        .get(&name.to_ascii_lowercase())
        .ok_or("view SVar declaration missing")?;
    let [row] = rows.as_slice() else {
        return Err(format!("view SVar {name} override order is unbound"));
    };
    if !row
        .field_type
        .as_deref()
        .is_some_and(|v| v.eq_ignore_ascii_case("int"))
    {
        return Err(format!("view SVar {name} requires native Integer type"));
    }
    let raw = row.raw.trim().parse::<i32>().map_err(text_error)?;
    let mut matches = 0;
    for node in catalog
        .actors
        .resolved_definitions("SVars")
        .map_err(text_error)?
    {
        if let Some(TypedValue::Integer { value, source, .. }) =
            node.field(name, pc).map_err(text_error)?
            && source == row.source
            && value == raw
        {
            matches += 1;
        }
    }
    if matches != 1 {
        return Err(format!(
            "view SVar {name} differs from private registered source value/span"
        ));
    }
    Ok(raw)
}

fn declared_boolean(catalog: &Catalog<'_>, name: &str, pc: Precision) -> Result<u8, String> {
    let settings = catalog
        .settings
        .as_ref()
        .ok_or("view SVar Settings owner missing")?;
    let rows = settings
        .declarations
        .get(&name.to_ascii_lowercase())
        .ok_or("view Bool SVar declaration missing")?;
    let [row] = rows.as_slice() else {
        return Err(format!("view SVar {name} override order is unbound"));
    };
    if !row
        .field_type
        .as_deref()
        .is_some_and(|v| v.eq_ignore_ascii_case("flag") || v.eq_ignore_ascii_case("bool"))
    {
        return Err(format!("view SVar {name} requires native Bool type"));
    }
    let raw = match row.raw.to_ascii_lowercase().as_str() {
        "true" => true,
        "false" => false,
        _ => return Err(format!("view SVar {name} literal Bool is required")),
    };
    let mut matches = 0;
    for node in catalog
        .actors
        .resolved_definitions("SVars")
        .map_err(text_error)?
    {
        if let Some(TypedValue::Boolean { value, source, .. }) =
            node.field(name, pc).map_err(text_error)?
            && source == row.source
            && value == raw
        {
            matches += 1;
        }
    }
    if matches != 1 {
        return Err(format!(
            "view SVar {name} differs from private registered source value/span"
        ));
    }
    Ok(u8::from(raw))
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
        // Validate the public Settings projection against immutable private nodes.
        declared_integer(catalog, "FogOfWar", pc)?;
        let fast = declared_boolean(catalog, "ZoneBuilderFastDetectionZones", pc)?;
        let settings = catalog.settings.as_ref().unwrap();
        if settings.declarations.contains_key("cheatfog") {
            return Err(
                "view CheatFog source override requires native assignment ownership".into(),
            );
        }
        for node in catalog
            .actors
            .resolved_definitions("SVars")
            .map_err(text_error)?
        {
            if node.field("CheatFog", pc).map_err(text_error)?.is_some() {
                return Err("view CheatFog private source override is unbound".into());
            }
        }
        let registry = catalog
            .actors
            .native_data_registry("WorldRule")
            .ok_or("view WorldRule source registry missing")?;
        for name in ["CheatFog", "ZoneBuilderFastDetectionZones"] {
            if registry.find(name).is_some() {
                return Err(format!(
                    "view {name} WorldRule default/publication is unbound"
                ));
            }
        }
        let (fog_default, _) = saved_action_settings::source_fog_definition(catalog, pc)?;
        let mission = crate::load_mission_variables::source_reset(catalog, pc)?;
        let combat = crate::load_combat_variables::Values::source_reset(catalog, &context)?;
        Ok(Self {
            current: Current::source_reset(fog_default, fast),
            combat,
            modifier_clamp_multiply: crate::load_property_numeric::multiply_source(catalog,pc)?,
            stockpile_default110: mission.stockpile_default110,
            stockpile_maximum1b8: mission.stockpile_maximum1b8,
            city_victory_ratio130: mission.city_victory_ratio130,
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
        self.validate_catalog(catalog)?;
        declared_integer(catalog, "FogOfWar", self.source_context.precision())?;
        let value = saved_action_settings::published_fog_integer(
            catalog,
            inventory,
            self.source_context.precision(),
        )?;
        let mission = crate::load_mission_variables::publication(
            catalog,
            inventory,
            self.source_context.precision(),
        )?;
        let sections: Vec<_> = inventory
            .opaque_initialization
            .iter()
            .filter(|s| s.name.eq_ignore_ascii_case("RULES"))
            .collect();
        let [section] = sections.as_slice() else {
            return Err("view SVar requires one actual standalone RULES source".into());
        };
        let registry = catalog
            .actors
            .native_data_registry("WorldRule")
            .ok_or("view WorldRule publication registry missing")?;
        let names = ["FogOfWar", "ResourceStockpileDefault", "CityVictoryRatio"];
        let mut seen = [0usize; 3];
        for entry in &registry.entries {
            for (slot, name) in names.iter().enumerate() {
                if entry.ids.eq_ignore_ascii_case(name) {
                    seen[slot] += 1;
                }
            }
        }
        if seen != [1, 1, 1] {
            return Err("view WorldRule native publication order is ambiguous".into());
        }
        //511CE0 prepares the WorldRules vector in manager registration order;
        //512420 changes matching values in place. These selected setters are
        //closed, flags0/20 paths with no fallible or outward mutable callback.
        for entry in &registry.entries {
            if entry.ids.eq_ignore_ascii_case("FogOfWar") {
                self.current.assign_fog(value);
            } else if entry.ids.eq_ignore_ascii_case("ResourceStockpileDefault") {
                self.stockpile_default110 = mission.stockpile_default110;
            } else if entry.ids.eq_ignore_ascii_case("CityVictoryRatio") {
                self.city_victory_ratio130 = mission.city_victory_ratio130;
            }
        }
        self.rules_source = Some(section.source.clone());
        Ok(())
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
            let bank=catalog.definition_bank.try_borrow().map_err(text_error)?;
            owner.validate_in_bank(catalog,&bank)?;
        }
        if owner.source_precision()!=self.source_context.precision() {
            return Err("current SVar and retained WorldRules source PC differ".into());
        }
        let mut seen=[0usize;3];
        for row in owner.records() {
            for (index,(name,kind)) in [("FogOfWar",3),("ResourceStockpileDefault",2),("CityVictoryRatio",1)].iter().enumerate() {
                if row.ids().eq_ignore_ascii_case(name) {
                    if row.kind()!=*kind {return Err("current SVar retained WorldRule type differs".into())}
                    seen[index]+=1;
                }
            }
        }
        if seen!=[1,1,1] {return Err("current SVar retained WorldRule identity/order is ambiguous".into())}
        //Closed native setters: type3 Fog maps to the actual signed Int SVar
        //with range0..2; type2 stockpile and type1 city have flags0/raw stores.
        for row in owner.records() {
            if row.ids().eq_ignore_ascii_case("FogOfWar") {self.current.assign_fog(row.value()as i32)}
            else if row.ids().eq_ignore_ascii_case("ResourceStockpileDefault") {self.stockpile_default110=row.value()}
            else if row.ids().eq_ignore_ascii_case("CityVictoryRatio") {self.city_victory_ratio130=row.value()as i32}
        }
        self.rules_source=Some(owner.rules_source().clone());
        Ok(())
    }
    pub fn require_published(&self) -> Result<(), String> {
        self.rules_source
            .as_ref()
            .map(|_| ())
            .ok_or_else(|| "view SVar WorldRules publication is incomplete".into())
    }
    pub fn fog_integer(&self) -> i32 {
        self.current.fog_of_war
    }
    pub fn combat_values(&self) -> &crate::load_combat_variables::Values {
        &self.combat
    }
    /// The same mutable current SVar cell used by Performer evaluation.
    pub fn assign_combat_bits(&mut self, field: crate::load_combat_variables::Field, value: u32) {
        self.combat.assign_bits(field, value);
    }
    ///Current Fixed SVar cell B39AB0+1A8, distinct from ClampAdd+1A4.
    pub fn modifier_clamp_multiply_bits(&self)->u32 {self.modifier_clamp_multiply}
    ///40E0A0 type2/flags0 raw assignment to the same retained cell.
    pub fn assign_modifier_clamp_multiply_bits(&mut self,bits:u32) {self.modifier_clamp_multiply=bits}
    pub fn stockpile_default_bits(&self) -> u32 {
        self.stockpile_default110
    }
    pub fn stockpile_maximum_bits(&self) -> u32 {
        self.stockpile_maximum1b8
    }
    pub fn assign_stockpile_maximum_bits(&mut self, value: u32) {
        self.stockpile_maximum1b8 = value
    }
    pub fn city_victory_ratio(&self) -> i32 {
        self.city_victory_ratio130
    }
    ///512160/40E110 on the actual type3/flags0 Var: no slider clamp.
    pub fn assign_city_victory_ratio(&mut self, value: i32) {
        self.city_victory_ratio130 = value;
    }
    ///5120F0/40E0A0 flags0 path stores the DWORD directly, including NaN bits.
    pub fn assign_stockpile_bits(&mut self, value: u32) {
        self.stockpile_default110 = value;
    }
    pub fn fog_enabled(&self) -> bool {
        self.current.fog_of_war != 0
    }
    pub fn cheat_fog_byte(&self) -> u8 {
        self.current.cheat_fog
    }
    pub fn cheat_fog_word(&self) -> &u8 {
        &self.current.cheat_fog
    }
    pub fn cheat_fog_enabled(&self) -> bool {
        self.current.cheat_fog != 0
    }
    pub fn fast_detection_zones_byte(&self) -> u8 {
        self.current.fast_detection_zones
    }
    pub fn fast_detection_zones(&self) -> bool {
        self.current.fast_detection_zones != 0
    }
    ///512160/40E110 on the known type3/flags20/range0..2 current Var.
    pub fn assign_fog_integer(&mut self, value: i32) {
        self.current.assign_fog(value)
    }
    ///41C670 on the actual type1/flags0 current Var; preserve all byte payloads.
    pub fn assign_cheat_fog(&mut self, value: u8) {
        self.current.assign_cheat(value)
    }
    pub fn assign_fast_detection_zones(&mut self, value: u8) {
        self.current.assign_fast(value)
    }
}
