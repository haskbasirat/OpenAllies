//! Source-bound ObjectData shapes and dimensions used by live actor getters.
use crate::{
    actors::ActorCatalog,
    native::{Error, Result, ScalarType, TypedScalar, TypedValue},
    resources::Provenance,
};
use aa_numeric::{Binary, Precision};
use serde::Serialize;
use std::{cmp::Ordering, collections::BTreeMap};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Shape {
    /// Native shape+0 kind,+4 radius,+8/+C minima,+10/+14 maxima.
    pub words: [u32; 6],
}

/// Original46E060 initialization plus46D8D0/46D8E0/46EB60/46E0E0 setter.
/// POINT/CIRCLE retain zero-initialized bounds; per-object visual scale is not
/// part of the shape getter contract. Inputs are already native binary32.
pub fn shape(selector: u32, dimensions: &[u32], precision: Precision) -> Result<Shape> {
    let mut words = [0; 6];
    let expected = match selector {
        0 => 0,
        1 | 2 => 1,
        3 => 2,
        _ => return Err(Error::Unsupported("unknown native shape selector".into())),
    };
    if dimensions.len() != expected {
        return Err(Error::Unsupported(
            "incorrect native shape dimensions".into(),
        ));
    }
    if selector == 0 {
        return Ok(Shape { words });
    }
    let width = Binary::from_f32_bits(dimensions[0])?;
    if selector == 1 {
        words[..2].copy_from_slice(&[1, dimensions[0]]);
        return Ok(Shape { words });
    }
    let height = Binary::from_f32_bits(dimensions[if selector == 2 { 0 } else { 1 }])?;
    words[0] = if width.compare(&height) == Ordering::Equal {
        2
    } else {
        3
    };
    let half = Binary::from_f32_bits(0x3f000000)?;
    let min_x = width.mul(&half.clone().negated(), precision)?;
    let max_y = height.mul(&half, precision)?;
    words[2] = min_x.store_f32()?;
    words[3] = height
        .mul(&half.clone().negated(), precision)?
        .store_f32()?;
    words[4] = width.mul(&half, precision)?.store_f32()?;
    words[5] = max_y.store_f32()?;
    //FST keeps both half-extents extended for the subsequent squares/sqrt.
    words[1] = max_y
        .mul(&max_y, precision)?
        .add(&min_x.mul(&min_x, precision)?, precision)?
        .sqrt(precision)?
        .store_f32()?;
    Ok(Shape { words })
}

#[derive(Clone, Debug, Serialize)]
pub struct ObjectDefinition {
    pub id: String,
    pub source: Provenance,
    pub height: TypedScalar,
    pub scale_min: TypedScalar,
    pub scale_max: TypedScalar,
    pub blocking: Shape,
    pub soft: Shape,
    pub target: Shape,
    pub has_explicit_target: bool,
    ///Winning values and declarations retain their separate source provenance.
    pub shape_inputs: BTreeMap<String, TypedValue>,
}

impl ActorCatalog {
    pub fn native_object_definition<'a>(
        &self,
        id: impl Into<crate::actors::ThingRef<'a>>,
        precision: Precision,
    ) -> Result<ObjectDefinition> {
        let id = id.into();
        // A successful result depends only on the selected root (query_memo),
        // apart from `id`, which keeps the caller's spelling.
        let root = self.selected_root(id).ok().flatten();
        let key = root.map(|root| (root, precision.significand_bits()));
        let memo = &self.resolution_cache.memo.object_definitions;
        if let Some(hit) = key.and_then(|key| crate::query_memo::lock(memo)?.get(&key).cloned()) {
            return Ok(ObjectDefinition {
                id: id.ids.into(),
                ..hit
            });
        }
        let result = self.native_object_definition_uncached(id, precision);
        if let (Some(key), Ok(value), Some(mut memo)) = (key, &result, crate::query_memo::lock(memo)) {
            memo.insert(key, value.clone());
        }
        result
    }
    fn native_object_definition_uncached(
        &self,
        id: crate::actors::ThingRef<'_>,
        precision: Precision,
    ) -> Result<ObjectDefinition> {
        if !matches!(self.native_category(id)?.native_kind, 1 | 2 | 20 | 21) {
            return Err(Error::Unsupported(
                "object shape loader requires ObjectData/ActorData/DynamicDoodadData".into(),
            ));
        }
        let source = self
            .select(id)
            .ok_or_else(|| Error::Unsupported(format!("missing actor {id}")))?
            .source
            .clone();
        let fixed = |path: &str| -> Result<TypedScalar> {
            match self.native_field(id, path, precision)? {
                Some(TypedValue::Scalar(s)) if s.field_type == ScalarType::Fixed => Ok(s),
                _ => Err(Error::Unsupported(format!(
                    "{path} requires an assigned fixed field"
                ))),
            }
        };
        let height = fixed("Size.height")?;
        let scale_min = fixed("Size.scale_min")?;
        let scale_max = fixed("Size.scale_max")?;
        if Binary::from_f32_bits(scale_max.bits)?.compare(&Binary::from_f32_bits(scale_min.bits)?)
            == Ordering::Less
        {
            return Err(Error::Unsupported(
                "native object scale_max is below scale_min".into(),
            ));
        }
        let mut inputs = BTreeMap::new();
        let mut get = |name: &str| -> Result<TypedValue> {
            let path = format!("Size.{name}");
            let value = self
                .native_field(id, &path, precision)?
                .ok_or_else(|| Error::Unsupported(format!("required field {path} absent")))?;
            inputs.insert(path, value.clone());
            Ok(value)
        };
        let selector = |value: &TypedValue| -> Result<Option<u32>> {
            let TypedValue::String { value, .. } = value else {
                return Err(Error::Unsupported(
                    "shape selector requires native string".into(),
                ));
            };
            if value.is_empty() {
                return Ok(None);
            }
            ["POINT", "CIRCLE", "SQUARE", "RECT"]
                .iter()
                .position(|s| value.eq_ignore_ascii_case(s))
                .map(|n| Some(n as u32))
                .ok_or_else(|| Error::Unsupported(format!("unknown native shape {value}")))
        };
        let dimensions = |kind: u32,
                          target: bool,
                          radius: &str,
                          get: &mut dyn FnMut(&str) -> Result<TypedValue>|
         -> Result<Vec<u32>> {
            let fields: &[&str] = match kind {
                0 => &[],
                1 => &[radius],
                2 if target => &["target_square_size"],
                2 => &["square_size"],
                3 if target => &["target_rect_width", "target_rect_height"],
                3 => &["rect_width", "rect_height"],
                _ => return Err(Error::Unsupported("unknown shape dimensions".into())),
            };
            fields
                .iter()
                .map(|field| get(field)?.fixed_bits())
                .collect()
        };
        let blocking_selector = selector(&get("blocking_shape")?)?
            .ok_or_else(|| Error::Unsupported("native blocking shape must be named".into()))?;
        let blocking = shape(
            blocking_selector,
            &dimensions(blocking_selector, false, "blocking_radius", &mut get)?,
            precision,
        )?;
        let soft_selector = selector(&get("soft_shape")?)?.unwrap_or(blocking_selector);
        let soft = shape(
            soft_selector,
            &dimensions(soft_selector, false, "soft_radius", &mut get)?,
            precision,
        )?;
        let target_selector = selector(&get("target_shape")?)?;
        let target = if let Some(kind) = target_selector {
            shape(
                kind,
                &dimensions(kind, true, "target_radius", &mut get)?,
                precision,
            )?
        } else {
            soft.clone()
        };
        Ok(ObjectDefinition {
            id: id.ids.into(),
            source,
            height,
            scale_min,
            scale_max,
            blocking,
            soft,
            target,
            has_explicit_target: target_selector.is_some(),
            shape_inputs: inputs,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct AspectDefinition {
    pub id: TypedValue,
    pub weight: TypedScalar,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct CreationDefinition {
    pub id: String,
    pub source: Provenance,
    pub variations_enabled: bool,
    pub variations_source: TypedValue,
    pub aspects: Vec<AspectDefinition>,
    pub variation_weights_bits: Vec<u32>,
    pub total_variation_weight_bits: u32,
    pub scale_min: TypedScalar,
    pub scale_max: TypedScalar,
}
impl ActorCatalog {
    /// Original ObjectData49DC40: when variations are disabled, AspectNormal
    /// loads first, followed by the ordered Aspect list.49C360 stores the total
    /// after each addition. This snapshot does not substitute model mesh counts.
    pub fn native_creation_definition<'a>(
        &self,
        id: impl Into<crate::actors::ThingRef<'a>>,
        precision: Precision,
    ) -> Result<CreationDefinition> {
        let id = id.into();
        let object = self.native_object_definition(id, precision)?;
        let view = self
            .resolved_actor(id)?
            .child("View")?
            .ok_or_else(|| Error::Unsupported("native ObjectData View node missing".into()))?;
        let variations_source = view
            .field("aspects_as_variations", precision)?
            .ok_or_else(|| Error::Unsupported("View.aspects_as_variations missing".into()))?;
        let variations_enabled = variations_source.boolean()?;
        let mut nodes = Vec::new();
        if !variations_enabled {
            nodes.push(view.child("AspectNormal")?.ok_or_else(|| {
                Error::Unsupported("nonvariation View requires AspectNormal".into())
            })?);
        }
        nodes.extend(view.children("Aspect")?);
        let mut aspects = Vec::new();
        let mut ids = std::collections::BTreeSet::new();
        let mut total = 0_u32;
        for node in nodes {
            let id = node
                .field("IDS", precision)?
                .ok_or_else(|| Error::Unsupported("Aspect IDS missing".into()))?;
            let TypedValue::String { value, .. } = &id else {
                return Err(Error::Unsupported(
                    "Aspect IDS must be native string".into(),
                ));
            };
            if !ids.insert(value.clone()) {
                return Err(Error::Unsupported(format!(
                    "duplicate native aspect IDS {value}"
                )));
            }
            let Some(TypedValue::Scalar(weight)) = node.field("weight", precision)? else {
                return Err(Error::Unsupported(
                    "Aspect weight must be assigned float".into(),
                ));
            };
            if weight.field_type != ScalarType::Float
                || Binary::from_f32_bits(weight.bits)?.compare(&Binary::from_u32(0))
                    != Ordering::Greater
            {
                return Err(Error::Unsupported(
                    "native Aspect weight must be positive float".into(),
                ));
            }
            total = Binary::from_f32_bits(total)?
                .add(&Binary::from_f32_bits(weight.bits)?, precision)?
                .store_f32()?;
            aspects.push(AspectDefinition {
                id,
                weight,
                sources: node.sources(),
            });
        }
        Ok(CreationDefinition {
            id: object.id,
            source: object.source,
            variations_enabled,
            variations_source,
            variation_weights_bits: aspects.iter().map(|a| a.weight.bits).collect(),
            aspects,
            total_variation_weight_bits: total,
            scale_min: object.scale_min,
            scale_max: object.scale_max,
        })
    }
}
