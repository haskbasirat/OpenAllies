//! Native target-definition grammar and the source-bound A&A target registry.
use crate::{
    actors::ActorCatalog,
    native::{Error, Result, TypedValue},
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct TargetFilter {
    pub required: u32,
    pub forbidden: u32,
}
/// Original4C5A80 target flag word. Unlike the Ability filter parser, tokens
/// are literal registry IDs; '+', '-' and '!' are not interpreted as prefixes.
pub fn parse_flag_word(
    raw: &str,
    mut initial: u32,
    mut lookup: impl FnMut(&str) -> Result<u32>,
) -> Result<u32> {
    for token in raw.split([',', ' ']).filter(|s| !s.is_empty()) {
        initial |= 1u32.wrapping_shl(lookup(token)?);
    }
    Ok(initial)
}
impl TargetFilter {
    pub const EMPTY: Self = Self {
        required: 0,
        forbidden: 0,
    };
    /// Original4C5BF0. Delimiters are exactly comma and ASCII space; the x86
    /// shift masks its index to five bits, including native indices above31.
    pub fn parse(
        raw: &str,
        mut initial: Self,
        mut lookup: impl FnMut(&str) -> Result<u32>,
    ) -> Result<Self> {
        for token in raw.split([',', ' ']).filter(|s| !s.is_empty()) {
            let (name, exclude) = if let Some(s) = token.strip_prefix(['!', '-']) {
                (s, true)
            } else {
                (token.strip_prefix('+').unwrap_or(token), false)
            };
            let mask = 1u32.wrapping_shl(lookup(name)?);
            if exclude {
                initial.forbidden |= mask;
            } else {
                initial.required |= mask;
            }
        }
        Ok(initial)
    }
    /// Native AbilityDefinition4BF411 defaults; effect-specific target filters
    /// do not receive these implicit constraints.
    pub fn for_ability(mut self) -> Self {
        if self.required & ((1 << 9) | (1 << 13)) == 0 {
            self.required |= 1 << 12;
        }
        if self.required & (1 << 14) == 0 {
            self.forbidden |= 1 << 14;
        }
        self
    }
}
pub const PREDEFINED_TARGETS: [&str; 16] = [
    "friend",
    "enemy",
    "neutral",
    "independent",
    "own",
    "self",
    "same_organization",
    "constructable",
    "cannot_build",
    "organization",
    "under_construction",
    "capturable",
    "body",
    "actor",
    "invulnerable",
    "upgrading",
];
#[derive(Clone, Debug, Serialize)]
pub struct TargetDomain {
    pub indices: BTreeMap<String, u32>,
    pub declarations: BTreeMap<String, Vec<Provenance>>,
}
impl TargetDomain {
    pub fn flag_word(&self, value: &TypedValue) -> Result<u32> {
        let TypedValue::String { value, .. } = value else {
            return Err(Error::Unsupported("target flags must be string".into()));
        };
        parse_flag_word(value, 0, |id| self.index(id))
    }
    pub fn index(&self, id: &str) -> Result<u32> {
        self.indices
            .get(&id.to_ascii_lowercase())
            .copied()
            .ok_or_else(|| Error::Unsupported(format!("unknown native TargetType {id}")))
    }
    pub fn parse(&self, value: &TypedValue, ability: bool) -> Result<TargetFilter> {
        let TypedValue::String { value, .. } = value else {
            return Err(Error::Unsupported("targets must be string".into()));
        };
        let filter = TargetFilter::parse(value, TargetFilter::EMPTY, |id| self.index(id))?;
        Ok(if ability {
            filter.for_ability()
        } else {
            filter
        })
    }
}
impl ActorCatalog {
    /// Data48DB90 inserts16 fixed IDs before source loading; policy1 reuses
    /// those entries. A&A's additional IDs occupy one source file, so their
    /// within-file order is proven without assigning guessed cross-file order.
    pub fn native_target_domain(&self, pc: Precision) -> Result<TargetDomain> {
        let mut indices: BTreeMap<_, _> = PREDEFINED_TARGETS
            .iter()
            .enumerate()
            .map(|(i, s)| (s.to_string(), i as u32))
            .collect();
        let mut declarations = BTreeMap::new();
        let mut dynamic_path = None;
        let mut next = 16;
        for node in self.resolved_definitions("TargetType")? {
            let Some(TypedValue::String { value, .. }) = node.field("IDS", pc)? else {
                return Err(Error::Unsupported("TargetType IDS not typed string".into()));
            };
            let id = value.to_ascii_lowercase();
            let sources = node.sources();
            let path = &sources[0].path;
            if declarations.contains_key(&id) {
                return Err(Error::Unsupported(format!(
                    "duplicate authored TargetType {id}"
                )));
            }
            if !indices.contains_key(&id) {
                if let Some(prior) = &dynamic_path {
                    if prior != path {
                        return Err(Error::Unsupported("TargetType extensions span multiple files; native index order unresolved".into()));
                    }
                } else {
                    dynamic_path = Some(path.clone());
                }
                indices.insert(id.clone(), next);
                next += 1;
            }
            declarations.insert(id, sources);
        }
        Ok(TargetDomain {
            indices,
            declarations,
        })
    }
}
