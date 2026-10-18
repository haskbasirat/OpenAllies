//! Native4FE590/4FE260 numeric consequence dispatch over the same held fields.
//! Modifier/Properties remains borrowed in place; positive unbound children
//! retain their exact reached boundary rather than becoming absent components.
use crate::{catalog::Catalog, load_property_instances::Fields, load_view_variables::State};
use aa_actors::components::ComponentSlot as Slot;
use aa_body::{
    Error, F32, Result,
    properties::PropertiesState,
    property_effects::{Instances, NumericKind, NumericModifier},
};
use aa_mapworld::restore::RestoredComponent as Component;
use aa_numeric::Precision;
fn error(e: impl std::fmt::Display) -> Error {
    Error::Callback(e.to_string())
}

///Actual Fixed declaration after controlled reset, never a formula fallback.
///The existing10CV cells retain ClampAdd(+1A4); this is Multiply(+1A8).
pub(crate) fn multiply_source(
    catalog: &Catalog<'_>,
    pc: Precision,
) -> std::result::Result<u32, String> {
    let name = "ModifierAttributeClampMultiply";
    let registry = catalog
        .actors
        .native_data_registry("WorldRule")
        .ok_or("modifier WorldRule registry absent")?;
    if registry.find(name).is_some() {
        return Err("ModifierAttributeClampMultiply WorldRule publication is unbound".into());
    }
    let settings = catalog
        .settings
        .as_ref()
        .ok_or("modifier Settings source owner missing")?;
    let (bits, source) = settings.unique_fixed(name, pc).map_err(|e| e.to_string())?;
    if !catalog
        .actors
        .sources
        .iter()
        .any(|s| s.path.eq_ignore_ascii_case(&source.path) && s.sha256 == source.sha256)
    {
        return Err("modifier Multiply source identity differs".into());
    }
    let mut count = 0;
    for node in catalog
        .actors
        .resolved_definitions("SVars")
        .map_err(|e| e.to_string())?
    {
        if let Some(aa_rules::native::TypedValue::Scalar(v)) =
            node.field(name, pc).map_err(|e| e.to_string())?
            && v.source == *source
            && v.field_type == aa_rules::native::ScalarType::Fixed
            && v.bits == bits
        {
            count += 1
        }
    }
    if count != 1 {
        return Err("modifier Multiply declaration/value/span differs".into());
    }
    Ok(bits)
}
fn source_pc(catalog: &Catalog<'_>, fields: &Fields<'_>) -> Result<Precision> {
    catalog.require_source_owner().map_err(error)?;
    let bank = catalog.definition_bank.try_borrow().map_err(error)?;
    let source = bank.combat_source().map_err(error)?;
    if source.source_identities() != catalog.actors.sources.as_slice() {
        return Err(error("numeric source Bank differs"));
    }
    let pc = source.source_precision();
    drop(bank);
    let id = &fields.definition.proof.id;
    catalog
        .require_untranslated_actor_identity(id)
        .map_err(error)?;
    let actor = catalog
        .actors
        .find(id)
        .ok_or_else(|| error("numeric canonical ActorData absent"))?;
    if actor.source != fields.definition.proof.source
        || actor.template_chain != fields.definition.proof.template_chain
        || catalog.actors.native_category(id).map_err(error)? != fields.definition.proof.category
    {
        return Err(error("numeric current definition identity differs"));
    }
    Ok(pc)
}
fn fixed(catalog: &Catalog<'_>, fields: &Fields<'_>, path: &str) -> Result<F32> {
    let pc = source_pc(catalog, fields)?;
    match catalog
        .actors
        .native_field(&fields.definition.proof.id, path, pc)
        .map_err(error)?
    {
        Some(aa_rules::native::TypedValue::Scalar(v))
            if v.field_type == aa_rules::native::ScalarType::Fixed =>
        {
            Ok(F32(v.bits))
        }
        _ => Err(error(format!(
            "numeric ActorData {path} fixed field is required"
        ))),
    }
}
fn additive(variables: &State, catalog: &Catalog<'_>) -> Result<F32> {
    variables.validate_catalog(catalog).map_err(error)?;
    Ok(F32(variables.combat_values().bits(
        crate::load_combat_variables::Field::ModifierClampAdd,
    )))
}
fn present(fields: &Fields<'_>, slot: Slot) -> bool {
    fields.components.present(slot)
}
fn abilities(
    catalog: &Catalog<'_>,
    fields: &mut Fields<'_>,
    properties: &PropertiesState,
    variables: &State,
) -> Result<()> {
    let pc = fields.precision;
    for slot in [Slot::Performer, Slot::Defender] {
        let count = match fields.components.get(slot) {
            None => 0,
            Some(Component::Performer(p)) => p.state.abilities.len(),
            Some(Component::Defender(d)) => d.abilities.len(),
            _ => return Err(error("numeric Ability component payload differs")),
        };
        for i in 0..count {
            let handle = match fields.components.get(slot) {
                Some(Component::Performer(p)) => p.state.abilities[i],
                Some(Component::Defender(d)) => d.abilities[i],
                _ => return Err(error("numeric Ability allocation changed")),
            };
            let local = crate::load_performer_value_refresh::local_ability(
                catalog,
                handle,
                source_pc(catalog, fields)?,
            )
            .map_err(error)?;
            let value = properties.modifier.power(
                local.definition.projectile_3c,
                local.definition.power_64,
                additive(variables, catalog)?,
                pc,
            )?;
            catalog
                .ability_arena
                .try_borrow_mut()
                .map_err(error)?
                .get_mut(handle)
                .map_err(error)?
                .power = value;
        }
    }
    Ok(())
}
///No successful placeholder callbacks. Absent components and native default
///jump-table cases are genuine completed leaves; reached unbound bodies fail.
pub fn changed(
    catalog: &Catalog<'_>,
    fields: &mut Fields<'_>,
    modifier: &NumericModifier,
    properties: &mut PropertiesState,
    _instances: &mut Instances,
    variables: &mut State,
) -> Result<()> {
    let pc = fields.precision;
    match (modifier.kind, modifier.index) {
        (NumericKind::Add, 0..=2) => abilities(catalog, fields, properties, variables),
        (NumericKind::Add, 3) if present(fields, Slot::Body) => {
            let base = fixed(catalog, fields, "BodyComponent.defense")?;
            let minimum = additive(variables, catalog)?;
            let Some(Component::Body(body)) = fields.components.get_mut(Slot::Body) else {
                return Err(error("numeric actual Body payload differs"));
            };
            if body.header.owner != fields.owner {
                return Err(error("numeric Body owner differs"));
            }
            body.body
                .refresh_defense(base, Some(&properties.modifier), minimum, pc)
        }
        (NumericKind::Add, 5) if present(fields, Slot::Supply) => {
            let base = fixed(catalog, fields, "SupplyComponent.supply_range")?;
            let value = properties.modifier.adjust_with_floor(
                0x18,
                base,
                additive(variables, catalog)?,
                pc,
            )?;
            let Some(Component::Supply(s)) = fields.components.get_mut(Slot::Supply) else {
                return Err(error("numeric actual Supply payload differs"));
            };
            if s.header.owner != fields.owner {
                return Err(error("numeric Supply owner differs"));
            }
            s.supply_range = value;
            s.byte_14 = 1;
            Ok(())
        }
        (NumericKind::Add, 6) if present(fields, Slot::Detection) => {
            //46889E finalization may raise +1CC above authored detection_range.
            let source = catalog
                .actors
                .native_ground_definition(&fields.definition.proof.id, source_pc(catalog, fields)?)
                .map_err(error)?;
            let bits = source
                .detection
                .as_ref()
                .ok_or_else(|| error("numeric finalized Detection source absent"))?
                .finalized_range_bits;
            if fields.definition.ground.detection_radius_bits != Some(bits) {
                return Err(error("numeric finalized Detection source owner differs"));
            }
            let base = F32(bits);
            let value = properties.modifier.adjust_with_floor(
                0x1c,
                base,
                additive(variables, catalog)?,
                pc,
            )?;
            let Some(Component::Detection(d)) = fields.components.get_mut(Slot::Detection) else {
                return Err(error("numeric actual Detection payload differs"));
            };
            if d.header.owner != fields.owner {
                return Err(error("numeric Detection owner differs"));
            }
            d.radius = value;
            Ok(())
        }
        (NumericKind::Add, 8) if present(fields, Slot::Organization) => {
            let base = fixed(catalog, fields, "OrganizationComponent.morale_breakpoint")?;
            let value = properties.modifier.adjust_with_floor(
                0x24,
                base,
                additive(variables, catalog)?,
                pc,
            )?;
            let Some(Component::Organization(o)) = fields.components.get_mut(Slot::Organization)
            else {
                return Err(error("numeric actual Organization payload differs"));
            };
            if o.header.owner != fields.owner {
                return Err(error("numeric Organization owner differs"));
            }
            o.definition_value_308 = value;
            Ok(())
        }
        (NumericKind::Add, 7) => {
            if present(fields, Slot::Organization) {
                return Err(error("578300 real Organization morale refresh is required"));
            }
            match fields.components.get(Slot::Element) {
                None => Ok(()),
                Some(Component::Element(e)) if e.organization.is_none() => Ok(()),
                Some(Component::Element(_)) => Err(error(
                    "578300 current Element parent Organization refresh is required",
                )),
                _ => Err(error("numeric actual Element payload differs")),
            }
        }
        (NumericKind::Multiply, 14) if present(fields, Slot::Element) => {
            let base = fixed(catalog, fields, "ElementComponent.resupply_rate")?;
            let value = F32(properties
                .modifier
                .multiply(0x3c, base, pc)?
                .store_f32()
                .map_err(error)?);
            let Some(Component::Element(e)) = fields.components.get_mut(Slot::Element) else {
                return Err(error("numeric actual Element payload differs"));
            };
            if e.header.owner != fields.owner {
                return Err(error("numeric Element owner differs"));
            }
            e.word_20 = value.0;
            Ok(())
        }
        (NumericKind::Multiply, 13) if present(fields, Slot::Mover) => Err(error(
            "576C70 actual Mover speed/group/Mind refresh is required",
        )),
        (NumericKind::Multiply, 19) if present(fields, Slot::Body) => {
            let ratio = match fields.components.get(Slot::Body) {
                Some(Component::Body(b)) if b.header.owner == fields.owner => {
                    b.body.capture_maximum_ratio(pc)?
                }
                _ => return Err(error("numeric actual Body owner/payload differs")),
            };
            let linked = *fields
                .words
                .get(&0x168)
                .ok_or_else(|| error("561D40 actual Actor+168 link is uninitialized"))?;
            if linked != 0 {
                let id = aa_actors::ActorId::from_raw(linked);
                if let Some(entry) = fields.registry.get(id) {
                    use aa_mapworld::restore::RestoredRecord;
                    let actor = match &entry.value {
                        RestoredRecord::Actor(e) => e.shell.actor.as_ref(),
                        RestoredRecord::Kingdom(e) => {
                            e.runtime.as_ref().and_then(|e| e.base.actor.as_ref())
                        }
                        RestoredRecord::Loading => {
                            return Err(error(
                                "561D40 reached held linked Body allocation is required",
                            ));
                        }
                        _ => None,
                    };
                    if actor.is_some_and(|a| a.components.present(Slot::Body)) {
                        return Err(error("561D40 reached linked Body recursion is required"));
                    }
                }
            }
            let base = fixed(catalog, fields, "BodyComponent.health")?;
            let maximum = properties.modifier.multiply(0x50, base, pc)?;
            let Some(Component::Body(b)) = fields.components.get_mut(Slot::Body) else {
                return Err(error("numeric actual Body allocation changed"));
            };
            b.body.apply_extended_maximum(ratio, maximum, true, pc)
        }
        (NumericKind::Multiply, 20) => {
            if present(fields, Slot::Defender) {
                let mask = fields
                    .definition
                    .component_mask
                    .ok_or_else(|| error("numeric ActorData+174 source is absent"))?;
                let base = fixed(
                    catalog,
                    fields,
                    if mask & (1 << 7) != 0 {
                        "OrganizationComponent.control_range"
                    } else {
                        "DefenderComponent.control_range"
                    },
                )?;
                let value = F32(properties
                    .modifier
                    .multiply(0x54, base, pc)?
                    .store_f32()
                    .map_err(error)?);
                let Some(Component::Defender(d)) = fields.components.get_mut(Slot::Defender) else {
                    return Err(error("numeric actual Defender payload differs"));
                };
                if d.header.owner != fields.owner {
                    return Err(error("numeric Defender owner differs"));
                }
                d.word_2c = value.0;
                d.byte_30 = 1;
            }
            if present(fields, Slot::Organization) {
                let base = fixed(catalog, fields, "OrganizationComponent.control_range")?;
                let value = F32(properties
                    .modifier
                    .multiply(0x54, base, pc)?
                    .store_f32()
                    .map_err(error)?);
                let Some(Component::Organization(o)) =
                    fields.components.get_mut(Slot::Organization)
                else {
                    return Err(error("numeric actual Organization payload differs"));
                };
                if o.header.owner != fields.owner {
                    return Err(error("numeric Organization owner differs"));
                }
                o.words.insert(0xac, value.0);
                o.bytes.insert(0xb0, 1);
            }
            Ok(())
        }
        (NumericKind::Multiply, 21 | 22) if present(fields, Slot::Organization) => {
            let value = properties.modifier.offense_factor()?;
            let Some(Component::Organization(o)) = fields.components.get_mut(Slot::Organization)
            else {
                return Err(error("numeric actual Organization payload differs"));
            };
            if o.header.owner != fields.owner {
                return Err(error("numeric Organization owner differs"));
            }
            o.words.insert(0x88, value.0);
            Ok(())
        }
        _ => Ok(()),
    }
}
