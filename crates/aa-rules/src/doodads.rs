//! DynamicDoodadData definition inputs, with a separate two-slot component mask.
//! FX actions remain opaque; this snapshot does not authorize component updates.
use crate::{
    actors::ActorCatalog,
    native::{Error, Result, TypedValue},
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize)]
pub struct Component {
    pub kind: u32,
    pub sources: Vec<Provenance>,
    pub fields: BTreeMap<String, TypedValue>,
    pub action_sources: Vec<Vec<Provenance>>,
    /// Native loader reads ToppleAction, although the template declares
    /// ToppledAction. Preserve the executable's actual field spelling.
    pub topple_action_sources: Vec<Vec<Provenance>>,
}
#[derive(Clone, Debug, Serialize)]
pub struct DynamicDoodadDefinition {
    pub id: String,
    /// Definition+170; unrelated to the 31-slot Actor component mask.
    pub component_mask: u32,
    /// Definition+174/+178. Repeated authored types replace the earlier slot.
    pub components: [Option<Component>; 2],
}
impl ActorCatalog {
    pub fn native_dynamic_doodad(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<DynamicDoodadDefinition> {
        if self.native_category(id)?.native_kind != 21 {
            return Err(Error::Unsupported(
                "DynamicDoodadData requires native kind21".into(),
            ));
        }
        let node = self.resolved_actor(id)?;
        let mut components: [Option<Component>; 2] = [None, None];
        let mut component_mask = 0;
        for child in node.children("Component")? {
            let ty = child
                .field("type", pc)?
                .ok_or_else(|| Error::Unsupported("doodad Component.type missing".into()))?;
            let kind = match &ty {
                TypedValue::String { value, .. } if value.eq_ignore_ascii_case("DISTURBABLE") => 0,
                TypedValue::String { value, .. } if value.eq_ignore_ascii_case("BREAKABLE") => 1,
                _ => return Err(Error::Unsupported("unknown doodad component type".into())),
            };
            let mut fields = BTreeMap::new();
            fields.insert("type".into(), ty);
            let scalars: &[&str] = if kind == 0 {
                &[
                    "update_time",
                    "affect_radius",
                    "recharge_time_min",
                    "recharge_time_max",
                ]
            } else {
                &[
                    "update_time",
                    "break_impulse",
                    "topple_duration",
                    "bounce_duration",
                    "bounce_percentage",
                    "regrow_dead_duration",
                    "regrow_sink_duration",
                    "regrow_grow_duration",
                ]
            };
            for name in scalars {
                let value = child
                    .field(name, pc)?
                    .ok_or_else(|| Error::Unsupported(format!("doodad {name} missing")))?;
                value.fixed_bits()?;
                fields.insert((*name).into(), value);
            }
            if kind == 1 {
                for name in ["toppled", "regrow"] {
                    let value = child
                        .field(name, pc)?
                        .ok_or_else(|| Error::Unsupported(format!("doodad {name} missing")))?;
                    value.boolean()?;
                    fields.insert(name.into(), value);
                }
                let value = child.field("trampled_aspect_ids", pc)?.ok_or_else(|| {
                    Error::Unsupported("doodad trampled_aspect_ids missing".into())
                })?;
                if !matches!(value, TypedValue::String { .. }) {
                    return Err(Error::Unsupported(
                        "doodad trampled_aspect_ids must be string".into(),
                    ));
                }
                fields.insert("trampled_aspect_ids".into(), value);
            }
            validate(kind, &fields)?;
            let component = Component {
                kind,
                sources: child.sources(),
                fields,
                action_sources: child
                    .children("Action")?
                    .iter()
                    .map(|n| n.sources())
                    .collect(),
                topple_action_sources: if kind == 1 {
                    child
                        .children("ToppleAction")?
                        .iter()
                        .map(|n| n.sources())
                        .collect()
                } else {
                    Vec::new()
                },
            };
            components[kind as usize] = Some(component);
            component_mask |= 1 << kind;
        }
        if component_mask == 0 {
            return Err(Error::Unsupported(
                "native dynamic doodad requires at least one Component".into(),
            ));
        }
        Ok(DynamicDoodadDefinition {
            id: self
                .find(id)
                .ok_or_else(|| Error::Unsupported("doodad identity missing".into()))?
                .id
                .clone(),
            component_mask,
            components,
        })
    }
}
fn validate(kind: u32, fields: &BTreeMap<String, TypedValue>) -> Result<()> {
    let fixed = |name: &str| -> Result<f32> { Ok(f32::from_bits(fields[name].fixed_bits()?)) };
    if kind == 0 {
        if fixed("affect_radius")? <= 0.0
            || fixed("recharge_time_max")? < fixed("recharge_time_min")?
        {
            return Err(Error::Unsupported(
                "native disturbable radius/recharge validation failed".into(),
            ));
        }
    } else {
        let toppled = fields["toppled"].boolean()?;
        let regrow = fields["regrow"].boolean()?;
        let TypedValue::String { value: aspect, .. } = &fields["trampled_aspect_ids"] else {
            unreachable!()
        };
        if !toppled && (aspect.is_empty() || regrow) || toppled && fixed("topple_duration")? <= 0.0
        {
            return Err(Error::Unsupported(
                "native breakable topple/aspect validation failed".into(),
            ));
        }
        if regrow
            && [
                "regrow_dead_duration",
                "regrow_sink_duration",
                "regrow_grow_duration",
            ]
            .iter()
            .any(|n| fixed(n).is_ok_and(|v| v <= 0.0))
        {
            return Err(Error::Unsupported(
                "native breakable regrow duration validation failed".into(),
            ));
        }
    }
    Ok(())
}
