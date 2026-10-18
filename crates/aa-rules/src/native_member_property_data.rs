//! ActorData+2D8 support-property list, read by45EC60.
//! Constructor4624B0 clears the list. ActorData464BD0 (466A79..466AA7)
//! calls4C4CF0 over ElementComponent/SupportProperty, resolving property_IDS
//! immediately at the owner's load position and prepending each result.
//! This source projection does not execute unrelated ActorData loading or
//! deferred callbacks, and does not create/apply runtime property instances.
use crate::{
    actors::{ActorCatalog, ResolvedNode},
    native::{
        CATEGORIES, TypedValue,
        layout_data::{
            Arg, LayoutError, NativeLayoutData, NativeThrow, PropertyRef, Result, RootPosition,
        },
    },
    resources::Provenance,
};
use serde::Serialize;

fn unsupported(message: impl Into<String>) -> LayoutError {
    LayoutError::Unsupported(message.into())
}

/// The typed source services4C4CF0 reads, in their native order. The count is
/// captured once; each selected child and lookup is read only when reached.
pub trait Source {
    fn child_count(&mut self) -> Result<i32>;
    fn property_ids(&mut self, index: i32) -> Result<String>;
    fn find_property(&mut self, ids: &str) -> Result<Option<PropertyRef>>;
}

///4C4CF0 successful-allocation domain, with immediate committed prepends.
/// A late typed-source/lookup error leaves earlier list entries intact.
/// The growable Vec is head first; its last value is the native tail value.
/// Allocator reentry and native null-allocation faults are not modeled.
pub fn load_list(source: &mut impl Source, head_first: &mut Vec<PropertyRef>) -> Result<()> {
    let count = source.child_count()?;
    for index in 0..count {
        let ids = source.property_ids(index)?;
        let property = source.find_property(&ids)?.ok_or_else(|| {
            LayoutError::Native(NativeThrow::Data {
                format: "Unknown property '%s'",
                args: vec![Arg::Text(ids)],
            })
        })?;
        head_first
            .try_reserve(1)
            .map_err(|e| unsupported(format!("4C4CF0 property-list allocation failed: {e}")))?;
        head_first.insert(0, property);
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
pub struct MemberPropertyData {
    /// Canonical registered owner, never translated again by the getter.
    pub id: String,
    pub element_present: bool,
    /// ActorData+2D8 head-first identities in the supplied layout registry.
    pub properties: Vec<PropertyRef>,
    pub sources: Vec<Provenance>,
}

fn text(node: &ResolvedNode<'_>, field: &str) -> Result<String> {
    match node.plain_string_field(field)? {
        Some(TypedValue::String { value, .. }) => Ok(value),
        None => Err(LayoutError::Native(NativeThrow::MissingField {
            field: field.into(),
        })),
        _ => Err(unsupported(format!("4C4CF0 requires string {field}"))),
    }
}

struct BoundSource<'a, 'b> {
    children: Vec<ResolvedNode<'a>>,
    layout: &'b NativeLayoutData,
    position: RootPosition,
}
impl Source for BoundSource<'_, '_> {
    fn child_count(&mut self) -> Result<i32> {
        i32::try_from(self.children.len())
            .map_err(|_| unsupported("SupportProperty count outside signed native domain"))
    }
    fn property_ids(&mut self, index: i32) -> Result<String> {
        text(
            self.children
                .get(index as usize)
                .ok_or_else(|| unsupported("SupportProperty child index unavailable"))?,
            "property_IDS",
        )
    }
    fn find_property(&mut self, ids: &str) -> Result<Option<PropertyRef>> {
        Ok(self.layout.find_property(ids, Some(self.position)))
    }
}

impl ActorCatalog {
    /// The completed2D8 source subsequence for one canonical ActorData owner.
    /// `layout` must be the same catalog's source registry used by all other
    /// PropertyRef consumers. String/identity operations are precision-independent;
    /// this API does not substitute restore precision for source loading.
    pub fn native_member_property_data(
        &self,
        layout: &NativeLayoutData,
        canonical_owner: &str,
    ) -> Result<MemberPropertyData> {
        if self.native_data_registry("NameList").is_none() {
            return Err(unsupported(
                "member properties require native registration order",
            ));
        }
        let registered = self
            .actors
            .iter()
            .find(|actor| actor.id.eq_ignore_ascii_case(canonical_owner))
            .ok_or_else(|| {
                unsupported(format!(
                    "canonical ActorData identity unavailable: {canonical_owner}"
                ))
            })?;
        // resolved_actor performs Thing Find/Translate. Match the registered
        // root provenance directly so a canonical owner is not translated twice.
        let mut roots = Vec::new();
        for tag in crate::actors::native_load::THING_TAGS {
            roots.extend(self.resolved_definitions(tag)?);
        }
        let actor = roots
            .into_iter()
            .find(|node| node.sources().first() == Some(&registered.source))
            .ok_or_else(|| unsupported("canonical member-property source root unavailable"))?;
        let category = text(&actor, "category")?;
        if !CATEGORIES.iter().any(|entry| {
            entry.name.eq_ignore_ascii_case(&category) && matches!(entry.native_kind, 2 | 19)
        }) {
            return Err(unsupported(
                "45EC60 requires ActorData or derived RandomActorData",
            ));
        }
        let source_file = self
            .sources
            .iter()
            .position(|file| file.path.eq_ignore_ascii_case(&registered.source.path))
            .ok_or_else(|| unsupported("member-property owner source file unavailable"))?;
        let element = actor.child("ElementComponent")?;
        let mut source = BoundSource {
            children: match &element {
                Some(element) => element.children("SupportProperty")?,
                None => Vec::new(),
            },
            layout,
            position: RootPosition {
                file: source_file,
                offset: registered.source.span.start,
            },
        };
        let mut properties = Vec::new();
        load_list(&mut source, &mut properties)?;
        Ok(MemberPropertyData {
            id: registered.id.clone(),
            element_present: element.is_some(),
            properties,
            sources: actor.sources(),
        })
    }
}
