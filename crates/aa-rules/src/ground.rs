//! Source-bound ground component inputs, with native derived values kept explicit.
use crate::{
    actors::{ActorCatalog, ResolvedNode},
    native::{Error, Result, ScalarType, TypedScalar, TypedValue},
    resources::Provenance,
};
use aa_numeric::{Binary, Precision};
use serde::Serialize;
use std::collections::BTreeMap;

fn required(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<TypedValue> {
    node.field(name, pc)?
        .ok_or_else(|| Error::Unsupported(format!("missing field {name}")))
}
fn fixed(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<TypedScalar> {
    match required(node, name, pc)? {
        TypedValue::Scalar(s) if s.field_type == ScalarType::Fixed => Ok(s),
        _ => Err(Error::Unsupported(format!("{name} must be fixed"))),
    }
}
fn text(value: &TypedValue) -> Result<&str> {
    match value {
        TypedValue::String { value, .. } => Ok(value),
        _ => Err(Error::Unsupported("expected string".into())),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RegistryDomain {
    pub ids: Vec<String>,
    pub declarations: Vec<Vec<Provenance>>,
}
impl RegistryDomain {
    fn require_single_file(&self, expected: &str) -> Result<()> {
        if self.declarations.iter().any(|sources| {
            sources
                .first()
                .is_none_or(|p| !p.path.eq_ignore_ascii_case(expected))
        }) {
            return Err(Error::Unsupported(format!(
                "native registry ordering requires its verified single source file {expected}"
            )));
        }
        Ok(())
    }
    pub fn count(&self) -> Result<u32> {
        u32::try_from(self.ids.len()).map_err(|_| Error::Unsupported("registry exceeds u32".into()))
    }
}
impl ActorCatalog {
    /// Original AbilityType registry starts empty; no capacity slots are entries.
    /// This returns distinct valid authored definitions, not an invented capacity.
    pub fn native_ability_type_domain(&self, pc: Precision) -> Result<RegistryDomain> {
        self.native_registry_domain("AbilityType", pc)
    }
    pub fn native_resource_domain(&self, pc: Precision) -> Result<RegistryDomain> {
        self.native_registry_domain("Resource", pc)
    }
    /// Original Resource registry starts empty and appends definitions in
    /// source node order. The checked A&A corpus declares every Resource in
    /// one indexed file, so no cross-file ordering assumption is needed.
    pub fn native_resource_registry(&self, pc: Precision) -> Result<RegistryDomain> {
        let domain = self.native_resource_domain(pc)?;
        domain.require_single_file("Game/resources.tgi")?;
        Ok(domain)
    }
    pub fn native_damage_type_registry(&self, pc: Precision) -> Result<RegistryDomain> {
        let domain = self.native_registry_domain("DamageType", pc)?;
        domain.require_single_file("Game/damage_types.tgi")?;
        Ok(domain)
    }
    fn native_registry_domain(&self, kind: &str, pc: Precision) -> Result<RegistryDomain> {
        let mut ids = Vec::<String>::new();
        let mut declarations = Vec::new();
        for node in self.resolved_definitions(kind)? {
            let value = required(&node, "IDS", pc)?;
            let id = text(&value)?;
            if id.is_empty() || ids.iter().any(|prior| prior.eq_ignore_ascii_case(id)) {
                return Err(Error::Unsupported(format!(
                    "invalid or duplicate {kind} IDS {id}"
                )));
            }
            ids.push(id.to_owned());
            declarations.push(node.sources());
        }
        Ok(RegistryDomain { ids, declarations })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct LayoutSlot {
    pub property_id: TypedValue,
    pub number: TypedValue,
    pub primary: TypedValue,
    pub required: TypedValue,
    pub locked: TypedValue,
    pub morale_line: TypedValue,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct OrganizationLayout {
    pub id: String,
    pub sources: Vec<Provenance>,
    pub slots: Vec<LayoutSlot>,
    /// Definition+28 array: each logical group repeated `number` times.
    pub expanded_groups: Vec<u32>,
    pub primary_index: i32,
    pub morale_count: i32,
}
/// Original4C3794..4C385E. No formation or live organization state is implied.
pub fn expand_slots(counts: &[(i32, bool, bool)]) -> Result<(Vec<u32>, i32, i32)> {
    let mut expanded = Vec::new();
    let mut primary = -1;
    let mut morale_count = 0i32;
    for (index, &(count, is_primary, morale)) in counts.iter().enumerate() {
        if count < 0 {
            return Err(Error::Unsupported(
                "negative layout multiplicity outside supported domain".into(),
            ));
        }
        if morale {
            morale_count = morale_count
                .checked_add(count)
                .ok_or_else(|| Error::Unsupported("layout morale count overflow".into()))?;
        }
        let count = usize::try_from(count)
            .map_err(|_| Error::Unsupported("layout multiplicity overflow".into()))?;
        let total = expanded
            .len()
            .checked_add(count)
            .filter(|&n| n <= i32::MAX as usize)
            .ok_or_else(|| Error::Unsupported("layout slot count overflow".into()))?;
        expanded
            .try_reserve(count)
            .map_err(|_| Error::Unsupported("layout allocation failed".into()))?;
        expanded.resize(
            total,
            u32::try_from(index)
                .map_err(|_| Error::Unsupported("layout group count overflow".into()))?,
        );
        if is_primary {
            if primary >= 0 {
                return Err(Error::Unsupported("multiple primary layout slots".into()));
            }
            primary = total as i32 - 1;
        }
    }
    Ok((expanded, primary, morale_count))
}
impl ActorCatalog {
    pub fn native_organization_layout(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<OrganizationLayout> {
        let node = self.resolved_definition("OrganizationLayout", id, pc)?;
        let mut slots = Vec::new();
        let mut counts = Vec::new();
        for slot in node.children("Slot")? {
            let number = required(&slot, "number", pc)?;
            let primary = required(&slot, "primary", pc)?;
            let morale_line = required(&slot, "morale_line", pc)?;
            counts.push((
                number.integer()?,
                primary.boolean()?,
                morale_line.boolean()?,
            ));
            slots.push(LayoutSlot {
                property_id: required(&slot, "property_IDS", pc)?,
                number,
                primary,
                required: required(&slot, "required", pc)?,
                locked: required(&slot, "locked", pc)?,
                morale_line,
                sources: slot.sources(),
            });
        }
        let (expanded_groups, primary_index, morale_count) = expand_slots(&counts)?;
        Ok(OrganizationLayout {
            id: id.to_owned(),
            sources: node.sources(),
            slots,
            expanded_groups,
            primary_index,
            morale_count,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct OrganizationDefinition {
    pub layout_id: TypedValue,
    pub layout: OrganizationLayout,
    /// Layout instance+0C: default ActorData pointers, one per logical group.
    /// The preserved typed IDs are verified against Element/property inputs;
    /// they are distinct from LayoutData Slot property references.
    pub parameters: Vec<Option<TypedValue>>,
    pub morale_breakpoint: TypedScalar,
    pub control_range: TypedScalar,
    pub operational_range: TypedScalar,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct EconomyDefinition {
    pub supply_required: TypedValue,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct DetectionDefinition {
    pub authored_range: TypedScalar,
    pub remote_range: TypedScalar,
    /// Final ActorData+1CC after original468B16..468BD9.
    pub finalized_range_bits: u32,
    pub finalized_remote_range_bits: u32,
    pub contributing_ranges: BTreeMap<String, TypedScalar>,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct GroundDefinition {
    pub id: String,
    pub attributes: crate::native::ActorAttributes,
    pub body_shortfall_damage_rate: Option<TypedScalar>,
    pub economy: Option<EconomyDefinition>,
    pub detection: Option<DetectionDefinition>,
    pub organization: Option<OrganizationDefinition>,
    pub denizen_guards: Option<crate::denizen::DenizenGuards>,
}
/// Original finite Detection finalizer. Values are Detection, Organization,
/// Supply, Denizen guard and Denizen operational range in that order.
pub fn finalized_detection_range(active: bool, values: [u32; 5]) -> Result<u32> {
    let mut max = values[0];
    let mut value = Binary::from_f32_bits(max)?;
    for bits in values.into_iter().skip(1) {
        let next = Binary::from_f32_bits(bits)?;
        if active && next.compare(&value) == std::cmp::Ordering::Greater {
            max = bits;
            value = next;
        }
    }
    Ok(max)
}
/// Original46889E remote Detection range write. The native condition is
/// nonnegative, including positive authored values; it is not a missing-value
/// fallback and is preserved exactly.
pub fn finalized_remote_detection_range(mask: u32, remote: u32, control: u32) -> Result<u32> {
    let remote_value = Binary::from_f32_bits(remote)?;
    Binary::from_f32_bits(control)?;
    Ok(
        if mask & (1 << 22) != 0
            && mask & (1 << 7) != 0
            && remote_value.compare(&Binary::from_u32(0)) != std::cmp::Ordering::Less
        {
            control
        } else {
            remote
        },
    )
}
impl ActorCatalog {
    pub fn native_ground_definition(&self, id: &str, pc: Precision) -> Result<GroundDefinition> {
        if self.native_category(id)?.native_kind != 2 {
            return Err(Error::Unsupported(
                "ground definition requires standard ActorData".into(),
            ));
        }
        let actor = self.resolved_actor(id)?;
        let attributes = self.native_attributes(id, pc)?;
        let body_shortfall_damage_rate = actor
            .child("BodyComponent")?
            .map(|n| fixed(&n, "unit_shortfall_damage_rate", pc))
            .transpose()?;
        let economy = actor
            .child("EconomyComponent")?
            .map(|n| {
                Ok::<_, Error>(EconomyDefinition {
                    supply_required: required(&n, "supply_required", pc)?,
                    sources: n.sources(),
                })
            })
            .transpose()?;
        let organization = actor
            .child("OrganizationComponent")?
            .map(|n| {
                let layout_id = required(&n, "layout_IDS", pc)?;
                let layout = self.native_organization_layout(text(&layout_id)?, pc)?;
                let parameters = self.native_organization_parameters(&n, &layout, pc)?;
                Ok::<_, Error>(OrganizationDefinition {
                    layout_id,
                    layout,
                    parameters,
                    morale_breakpoint: fixed(&n, "morale_breakpoint", pc)?,
                    control_range: fixed(&n, "control_range", pc)?,
                    operational_range: fixed(&n, "operational_range", pc)?,
                    sources: n.sources(),
                })
            })
            .transpose()?;
        let denizen_guards = self.native_denizen_guards(id, pc)?;
        let detection = actor
            .child("DetectionComponent")?
            .map(|n| {
                let authored_range = fixed(&n, "detection_range", pc)?;
                let remote_range = fixed(&n, "remote_detection_range", pc)?;
                let mut ranges = BTreeMap::new();
                let mut values = [authored_range.bits, 0, 0, 0, 0];
                if let Some(org) = &organization {
                    values[1] = org.control_range.bits;
                    ranges.insert(
                        "OrganizationComponent.control_range".into(),
                        org.control_range.clone(),
                    );
                }
                if let Some(supply) = actor.child("SupplyComponent")? {
                    let range = fixed(&supply, "supply_range", pc)?;
                    values[2] = range.bits;
                    ranges.insert("SupplyComponent.supply_range".into(), range);
                }
                if let Some(denizen) = &denizen_guards {
                    values[3] = denizen.finalized_bits[0];
                    values[4] = denizen.finalized_bits[1];
                    ranges.insert(
                        "DenizenComponent.guard_range".into(),
                        denizen.guard_range.clone(),
                    );
                    ranges.insert(
                        "DenizenComponent.guard_operational_range".into(),
                        denizen.operational_range.clone(),
                    );
                }
                // Native ActorData constructor explicitly initializes absent range slots to
                // zero. The remote range is replaced by control range only for Organization
                // and a nonnegative authored remote range (original468898..4688D4).
                let finalized_remote_range_bits = finalized_remote_detection_range(
                    (1 << 22) | if organization.is_some() { 1 << 7 } else { 0 },
                    remote_range.bits,
                    values[1],
                )?;
                Ok::<_, Error>(DetectionDefinition {
                    authored_range,
                    remote_range,
                    finalized_range_bits: finalized_detection_range(true, values)?,
                    finalized_remote_range_bits,
                    contributing_ranges: ranges,
                    sources: n.sources(),
                })
            })
            .transpose()?;
        Ok(GroundDefinition {
            id: id.to_owned(),
            attributes,
            body_shortfall_damage_rate,
            economy,
            detection,
            organization,
            denizen_guards,
        })
    }
}

/// Original460000 post-link checks for a resolved ordinary element definition.
pub fn validate_organization_element(
    is_element: bool,
    has_property: bool,
    mobile_deployment: bool,
    multiplicity: i32,
) -> Result<()> {
    if !is_element {
        return Err(Error::Unsupported(
            "organization member lacks ElementComponent".into(),
        ));
    }
    if !has_property {
        return Err(Error::Unsupported(
            "organization member lacks required layout property".into(),
        ));
    }
    if mobile_deployment && multiplicity > 1 {
        return Err(Error::Unsupported(
            "packable organization element must occupy its own group".into(),
        ));
    }
    Ok(())
}
impl ActorCatalog {
    fn native_organization_parameters(
        &self,
        node: &ResolvedNode<'_>,
        layout: &OrganizationLayout,
        pc: Precision,
    ) -> Result<Vec<Option<TypedValue>>> {
        let authored = node.children("Slot")?;
        if authored.len() > layout.slots.len() {
            return Err(Error::Unsupported(
                "too many Organization Slot elements".into(),
            ));
        }
        let mut parameters = Vec::new();
        for (index, slot) in layout.slots.iter().enumerate() {
            if slot.locked.boolean()? && !slot.required.boolean()? {
                return Err(Error::Unsupported(
                    "locked organization group must also be required".into(),
                ));
            }
            let value = authored
                .get(index)
                .map(|n| required(n, "element_IDS", pc))
                .transpose()?;
            let value = match value {
                Some(v) if !text(&v)?.is_empty() => Some(v),
                _ => None,
            };
            let Some(value) = value else {
                if slot.required.boolean()? {
                    return Err(Error::Unsupported(
                        "required organization group lacks default element".into(),
                    ));
                }
                parameters.push(None);
                continue;
            };
            let id = text(&value)?;
            if self.native_category(id)?.native_kind != 2 {
                return Err(Error::Unsupported(
                    "organization element requires standard ActorData".into(),
                ));
            }
            let actor = self.resolved_actor(id)?;
            let property = text(&slot.property_id)?;
            self.resolved_definition("Property", property, pc)?;
            let mut has_property = false;
            if let Some(properties) = actor.child("PropertiesComponent")? {
                for reference in properties.children("Property")? {
                    if text(&required(&reference, "property_IDS", pc)?)?
                        .eq_ignore_ascii_case(property)
                    {
                        has_property = true;
                    }
                }
            }
            validate_organization_element(
                actor.child("ElementComponent")?.is_some(),
                has_property,
                actor.child("MobileDeploymentComponent")?.is_some(),
                slot.number.integer()?,
            )?;
            parameters.push(Some(value));
        }
        Ok(parameters)
    }
}
