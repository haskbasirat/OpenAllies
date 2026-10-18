//! Bounded Construction constructor parameters, with a proved absent upgrade
//! graph. Nonempty source graphs require native callback order and remain errors.
use crate::{
    actors::{ActorCatalog, ConstructionUpgradeCensus},
    economy_values::ResourceValues,
    native::{Error, Result},
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct ConstructionInputs {
    pub id: String,
    pub sources: Vec<Provenance>,
    /// None without an EconomyComponent: 5657B0 assigns the cost vector only
    /// when component-mask bit 9 is set, so the Construction costs stay empty.
    pub resources: Option<ResourceValues>,
    /// Original4C0 outgoing Upgrade lists and4D0 predecessor lists remain empty
    /// because the complete supplied source census contains no Upgrade instance.
    pub upgrade_census: ConstructionUpgradeCensus,
    pub has_structure: bool,
}
impl ActorCatalog {
    pub fn native_construction_inputs(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<Option<ConstructionInputs>> {
        let actor = self.resolved_actor(id)?;
        let Some(construction) = actor.child("ConstructionComponent")? else {
            return Ok(None);
        };
        if self.native_category(id)?.native_kind != 2 {
            return Err(Error::Unsupported(
                "Construction inputs require ActorData".into(),
            ));
        }
        let upgrade_census = self.construction_upgrade_census()?;
        if !upgrade_census.explicit_instances.is_empty() {
            return Err(Error::Unsupported("Construction predecessor callbacks are unresolved for a nonempty Upgrade source graph".into()));
        }
        let resources = self.native_economy_inputs(id, pc)?.resources;
        Ok(Some(ConstructionInputs {
            id: self
                .find(id)
                .ok_or_else(|| Error::Unsupported("Construction identity missing".into()))?
                .id
                .clone(),
            sources: construction.sources(),
            resources,
            upgrade_census,
            has_structure: actor.child("StructureComponent")?.is_some(),
        }))
    }
}
/// ActorData Construction command inputs read by 565140/565090.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ConstructionFlags {
    /// +4E4 `can_sell`.
    pub can_sell: bool,
    /// +4E5 `can_raze`, cleared by 467C52..467C5C when `can_sell` is set.
    pub can_raze: bool,
    /// +4C0 outgoing Upgrade count; zero is proven by the complete source census.
    pub upgrade_count: i32,
}
impl ActorCatalog {
    /// Original464BD0 ConstructionComponent tail (467C16..467C63).
    pub fn native_construction_flags(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<Option<ConstructionFlags>> {
        let actor = self.resolved_actor(id)?;
        let Some(construction) = actor.child("ConstructionComponent")? else {
            return Ok(None);
        };
        let flag = |name: &str| -> Result<bool> {
            construction
                .field(name, pc)?
                .ok_or_else(|| {
                    Error::Unsupported(format!("Construction field {name} missing (native throws)"))
                })?
                .boolean()
        };
        let can_sell = flag("can_sell")?;
        let can_raze = flag("can_raze")? && !can_sell;
        if !self.construction_upgrade_census()?.explicit_instances.is_empty() {
            return Err(Error::Unsupported(
                "ActorData+4C0 outgoing Upgrade count needs the native Upgrade registration order"
                    .into(),
            ));
        }
        Ok(Some(ConstructionFlags {
            can_sell,
            can_raze,
            upgrade_count: 0,
        }))
    }
}
/// ActorData SupplyComponent fields (464BD0, 46783C..4678BB).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SupplyFields {
    /// +420 `supply_range` (fixed getter, binary32 store).
    pub supply_range_bits: u32,
    /// +424 `is_relay`.
    pub is_relay: bool,
    /// +428 `stockpile_max`, read only when `is_relay`; a negative value
    /// throws (`fcomp 0; test ah,5; jp ok`: ordered less-than fails).
    pub stockpile_max_bits: Option<u32>,
}
impl ActorCatalog {
    pub fn native_supply_fields(&self, id: &str, pc: Precision) -> Result<Option<SupplyFields>> {
        let actor = self.resolved_actor(id)?;
        let Some(supply) = actor.child("SupplyComponent")? else {
            return Ok(None);
        };
        let field = |name: &str| {
            supply.field(name, pc)?.ok_or_else(|| {
                Error::Unsupported(format!("Supply field {name} missing (native throws)"))
            })
        };
        let supply_range_bits = field("supply_range")?.fixed_bits()?;
        let is_relay = field("is_relay")?.boolean()?;
        let stockpile_max_bits = if is_relay {
            let bits = field("stockpile_max")?.fixed_bits()?;
            let nan = bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x7f_ffff != 0;
            if !nan && bits >> 31 != 0 && bits & 0x7fff_ffff != 0 {
                return Err(Error::Unsupported(format!(
                    "Maximum supply stockpile cannot be negative for actor '{id}' (native throws)"
                )));
            }
            Some(bits)
        } else {
            None
        };
        Ok(Some(SupplyFields {
            supply_range_bits,
            is_relay,
            stockpile_max_bits,
        }))
    }
}
impl ActorCatalog {
    /// ActorData+558 ResearchComponent items: each `property_IDS` resolved to
    /// its registered Property definition's canonical IDS, in list order.
    pub fn native_research_property_ids(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<Option<Vec<String>>> {
        let actor = self.resolved_actor(id)?;
        let Some(research) = actor.child("ResearchComponent")? else {
            return Ok(None);
        };
        let mut out = Vec::new();
        for item in research.children("ResearchItem")? {
            let Some(crate::native::TypedValue::String { value, .. }) =
                item.field("property_IDS", pc)?
            else {
                return Err(Error::Unsupported(
                    "ResearchItem property_IDS missing (native throws)".into(),
                ));
            };
            let property = self.resolved_definition("Property", &value, pc)?;
            let Some(crate::native::TypedValue::String { value: canonical, .. }) =
                property.field("IDS", pc)?
            else {
                return Err(Error::Unsupported("Property definition lacks IDS".into()));
            };
            out.push(canonical);
        }
        Ok(Some(out))
    }
}
impl ActorCatalog {
    /// 464BD0 CityComponent (4683F0..4684DE): at least one Layout child,
    /// then `zone_radius` (+5B4); an ordered negative value throws.
    pub fn native_city_zone_radius(&self, id: &str, pc: Precision) -> Result<Option<u32>> {
        let actor = self.resolved_actor(id)?;
        let Some(city) = actor.child("CityComponent")? else {
            return Ok(None);
        };
        if city.children("Layout")?.is_empty() {
            return Err(Error::Unsupported(format!(
                "No layouts specified for city '{id}' (native throws)"
            )));
        }
        let bits = city
            .field("zone_radius", pc)?
            .ok_or_else(|| Error::Unsupported("City zone_radius missing (native throws)".into()))?
            .fixed_bits()?;
        let nan = bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x7f_ffff != 0;
        if !nan && bits >> 31 != 0 && bits & 0x7fff_ffff != 0 {
            return Err(Error::Unsupported(format!(
                "City zone radius must be positive for '{id}' (native throws)"
            )));
        }
        Ok(Some(bits))
    }
}
