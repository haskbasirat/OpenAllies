//! Source-bound Character Animation rules. Gameplay seconds remain independent
//! of the KF controller duration. Resource loading and full FX compilation are
//! separate gates; retaining an authored KF expression does not prove it loaded.
use crate::runtime::Animation;
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use aa_rules::{
    actors::{ActorCatalog, ResolvedNode},
    native::{ScalarType, TypedValue},
    resources::Provenance,
};
use serde::Serialize;
use std::{cmp::Ordering, collections::BTreeMap};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Rules(#[from] aa_rules::native::Error),
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("unsupported source animation: {0}")]
    Unsupported(String),
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Debug, Serialize)]
pub struct BoundAnimation {
    pub id: String,
    pub timing: Animation,
    ///Unexpanded original source expression; never a guessed resource filename.
    pub kf_expression: String,
    pub fields: BTreeMap<String, TypedValue>,
    pub cue_time_fields: Vec<TypedValue>,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct CharacterDefinitions {
    pub actor_id: String,
    pub resource_prefix: TypedValue,
    pub base: Vec<BoundAnimation>,
    pub aspect_ids: Vec<String>,
    pub aspects: BTreeMap<String, Vec<BoundAnimation>>,
    pub sources: Vec<Provenance>,
}
impl CharacterDefinitions {
    ///45E690 falls back to the final/base set for -1 or a missing aspect set.
    pub fn for_aspect(&self, aspect: Option<&str>) -> &[BoundAnimation] {
        aspect
            .and_then(|id| self.aspects.get(&id.to_ascii_lowercase()))
            .map(Vec::as_slice)
            .unwrap_or(&self.base)
    }
    ///45EBB0 first case-insensitive authored name match.
    pub fn find(&self, aspect: Option<&str>, id: &str) -> Option<&BoundAnimation> {
        self.for_aspect(aspect)
            .iter()
            .find(|a| a.id.eq_ignore_ascii_case(id))
    }
    pub fn for_variant(&self, variant: u32) -> Result<&[BoundAnimation]> {
        if variant as usize == self.aspect_ids.len() {
            return Ok(&self.base);
        }
        let aspect = self.aspect_ids.get(variant as usize).ok_or_else(|| {
            Error::Unsupported(format!(
                "native animation variant {variant} outside aspect list"
            ))
        })?;
        Ok(self.for_aspect(Some(aspect)))
    }
    ///ActorData+1C0 is aspect count plus the final base/default Character set.
    pub fn set_count(&self) -> usize {
        self.aspect_ids.len() + 1
    }
    ///45EC00 searches only the requested native kind, preserving list order.
    pub fn lookup(
        &self,
        set: u32,
        name: &str,
        expected_kind: u32,
    ) -> Result<Option<&BoundAnimation>> {
        Ok(self
            .for_variant(set)?
            .iter()
            .find(|a| a.timing.kind == expected_kind && a.id.eq_ignore_ascii_case(name)))
    }
}
impl aa_rules::abilities::AnimationResolver for CharacterDefinitions {
    fn set_count(&self) -> u32 {
        //Original model-set indices/counts are u32. Source construction checks
        //this representability; this does not impose an asset/memory budget.
        self.set_count() as u32
    }
    fn lookup(
        &self,
        set: u32,
        name: &str,
        expected_kind: u32,
    ) -> aa_rules::native::Result<Option<aa_rules::abilities::AnimationBinding>> {
        self.lookup(set, name, expected_kind)
            .map_err(|error| aa_rules::native::Error::Unsupported(error.to_string()))
            .map(|binding| {
                binding.map(|animation| aa_rules::abilities::AnimationBinding {
                    name: animation.id.clone(),
                    length_bits: animation.timing.length.0,
                    use_fraction_bits: animation.timing.use_fraction.0,
                    sources: animation.sources.clone(),
                })
            })
    }
}
fn required(node: &ResolvedNode<'_>, name: &str, p: Precision) -> Result<TypedValue> {
    node.field(name, p)?
        .ok_or_else(|| Error::Unsupported(format!("required animation field {name} missing")))
}
fn text(v: &TypedValue) -> Result<String> {
    if let TypedValue::String { value, .. } = v {
        Ok(value.clone())
    } else {
        Err(Error::Unsupported("expected native string".into()))
    }
}
fn fixed(v: &TypedValue) -> Result<F32> {
    if let TypedValue::Scalar(v) = v
        && v.field_type == ScalarType::Fixed
    {
        Binary::from_f32_bits(v.bits)?;
        Ok(F32(v.bits))
    } else {
        Err(Error::Unsupported(
            "expected native fixed animation field".into(),
        ))
    }
}
///Canonical type rows in native registry499FF0. Unknown/custom aliases remain
///explicit errors here rather than being guessed from clip/actor names.
fn canonical_kind(name: &str) -> Result<u32> {
    ["idle", "fidget", "move", "die", "ability", "summon", "hit"]
        .iter()
        .position(|s| name.eq_ignore_ascii_case(s))
        .map(|i| i as u32)
        .ok_or_else(|| {
            Error::Unsupported(format!(
                "animation type {name} is not a supported canonical native type"
            ))
        })
}
pub fn animation(node: &ResolvedNode<'_>, p: Precision) -> Result<BoundAnimation> {
    let mut fields = BTreeMap::new();
    for key in [
        "IDS",
        "type",
        "kf",
        "length",
        "key_ratio",
        "walk_distance",
        "animation_blend",
        "sister_anim_type",
        "weight",
        "damage_threshold",
        "damage_filter",
    ] {
        fields.insert(key.into(), required(node, key, p)?);
    }
    let length = fixed(&fields["length"])?;
    let use_fraction = fixed(&fields["key_ratio"])?;
    //A positive desired duration is a portable consumer precondition. The original
    //loader diagnoses ratio bounds but does not itself assert positive length.
    if Binary::from_f32_bits(length.0)?.compare(&Binary::from_u32(0)) != Ordering::Greater {
        return Err(Error::Unsupported(
            "nonpositive desired animation length".into(),
        ));
    }
    let ratio = Binary::from_f32_bits(use_fraction.0)?;
    if ratio.compare(&Binary::from_u32(0)) == Ordering::Less
        || ratio.compare(&Binary::from_u32(1)) == Ordering::Greater
    {
        return Err(Error::Unsupported("native key_ratio outside0..1".into()));
    }
    let mut cue_time_fields = Vec::new();
    for cue in node.children("WhenDoAtTime")? {
        cue_time_fields.push(required(&cue, "time", p)?);
    }
    //461F70 inserts every parsed cue at the head; Performer/Character visit that
    //linked order. Sorting by time would change native same-tick dispatch order.
    cue_time_fields.reverse();
    let cues = cue_time_fields
        .iter()
        .map(fixed)
        .collect::<Result<Vec<_>>>()?;
    Ok(BoundAnimation {
        id: text(&fields["IDS"])?,
        kf_expression: text(&fields["kf"])?,
        timing: Animation {
            kind: canonical_kind(&text(&fields["type"])?)?,
            length,
            use_fraction,
            walk_distance: fixed(&fields["walk_distance"])?,
            cues,
        },
        fields,
        cue_time_fields,
        sources: node.sources(),
    })
}
pub fn character(
    catalog: &ActorCatalog,
    id: &str,
    p: Precision,
) -> Result<Option<CharacterDefinitions>> {
    let actor = catalog.resolved_actor(id)?;
    let Some(character) = actor.child("CharacterComponent")? else {
        return Ok(None);
    };
    let view = actor
        .child("View")?
        .ok_or_else(|| Error::Unsupported("Character actor has no View".into()))?;
    let base = character
        .children("Animation")?
        .iter()
        .map(|n| animation(n, p))
        .collect::<Result<Vec<_>>>()?;
    let creation = catalog.native_creation_definition(id, p)?;
    let aspect_ids = creation
        .aspects
        .iter()
        .map(|a| text(&a.id))
        .collect::<Result<Vec<_>>>()?;
    if aspect_ids.len() >= u32::MAX as usize {
        return Err(Error::Unsupported(
            "native animation set count exceeds u32".into(),
        ));
    }
    let mut aspects = BTreeMap::new();
    for set in character.children("AspectAnimationSet")? {
        let aspect = text(&required(&set, "aspect_ids", p)?)?.to_ascii_lowercase();
        if !aspect_ids.iter().any(|v| v.eq_ignore_ascii_case(&aspect)) {
            return Err(Error::Unsupported(format!(
                "animation set references unknown aspect {aspect}"
            )));
        }
        let mut rows = set
            .children("Animation")?
            .iter()
            .map(|n| animation(n, p))
            .collect::<Result<Vec<_>>>()?;
        for fallback in &base {
            if !rows.iter().any(|r| r.id.eq_ignore_ascii_case(&fallback.id)) {
                rows.push(fallback.clone());
            }
        }
        if aspects.insert(aspect.clone(), rows).is_some() {
            return Err(Error::Unsupported(format!(
                "duplicate animation aspect {aspect}"
            )));
        }
    }
    Ok(Some(CharacterDefinitions {
        actor_id: id.into(),
        resource_prefix: required(&view, "resource_prefix", p)?,
        base,
        aspect_ids,
        aspects,
        sources: character.sources(),
    }))
}
