//! Which Character animation set a Character component binds to.
//!
//! Character constructor 563670 takes the first animation of the list that
//! 4D0720 returns for its owner actor: a stance word (Actor+15C) other than
//! -1 maps through 45E5E0 (ActorData+188 stance -> +8 -> +8 aspect index),
//! otherwise the Object variant (Object+40) is used; 45E690 then returns
//! ActorData+1BC[index], or the last set (+1C0 - 1, the base set) for -1 or
//! a null entry, and null when +1C0 is 0. The saved-object load constructor
//! 4D6D70 sets Actor+15C = -1 and the saved actor tail keeps it -1, so a
//! restored Character always selects by the loaded Object variant.
use crate::{
    actors::ActorCatalog,
    native::{Error, Result, TypedValue},
};
use aa_numeric::Precision;

/// Original 4D0720 + 45E5E0 + 45E690. `sets_present[i]` is whether
/// ActorData+1BC[i] is non-null (aspect sets, then the base set last).
/// `Ok(None)` is the native null list (+1C0 == 0). Indices outside the
/// native arrays are explicit errors (the original reads past them).
pub fn native_character_view_set(
    sets_present: &[bool],
    stance_aspects: &[i32],
    stance: i32,
    variant: i32,
) -> Result<Option<usize>> {
    if sets_present.is_empty() {
        return Ok(None);
    }
    let index = if stance != -1 {
        *usize::try_from(stance)
            .ok()
            .and_then(|s| stance_aspects.get(s))
            .ok_or_else(|| Error::Unsupported(format!("stance {stance} outside ActorData+188")))?
    } else {
        variant
    };
    let last = sets_present.len() - 1;
    if index == -1 {
        return Ok(Some(last));
    }
    let slot = usize::try_from(index)
        .ok()
        .filter(|&i| i < sets_present.len())
        .ok_or_else(|| Error::Unsupported(format!("animation set {index} outside ActorData+1BC")))?;
    Ok(Some(if sets_present[slot] { slot } else { last }))
}

impl ActorCatalog {
    /// ActorData+1BC presence for an actor with a CharacterComponent: one
    /// entry per creation aspect (View Aspect list, `native_creation_definition`)
    /// that has an AspectAnimationSet, then the base set. `Ok(None)` when the
    /// actor has no CharacterComponent.
    pub fn native_character_sets_present(&self, id: &str, pc: Precision) -> Result<Option<Vec<bool>>> {
        let actor = self.resolved_actor(id)?;
        let Some(character) = actor.child("CharacterComponent")? else {
            return Ok(None);
        };
        let creation = self.native_creation_definition(id, pc)?;
        let mut with_sets = Vec::new();
        for set in character.children("AspectAnimationSet")? {
            match set.field("aspect_ids", pc)? {
                Some(TypedValue::String { value, .. }) => with_sets.push(value.to_ascii_lowercase()),
                _ => {
                    return Err(Error::Unsupported(
                        "AspectAnimationSet aspect_ids must be a String".into(),
                    ));
                }
            }
        }
        let mut present = Vec::new();
        for aspect in &creation.aspects {
            let TypedValue::String { value, .. } = &aspect.id else {
                return Err(Error::Unsupported("Aspect IDS must be a String".into()));
            };
            present.push(with_sets.contains(&value.to_ascii_lowercase()));
        }
        present.push(true);
        Ok(Some(present))
    }
}
