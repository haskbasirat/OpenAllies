//! Completed ActorData+17C recruitment backreferences. Constructor4624B0
//! initializes the list empty; deferred460310 and460000 populate it after
//! all definitions and Translate roots have loaded (473AD0/475230).
//! This is not transport_actor+184. Canonical getters do not translate again.
use crate::{
    actors::{ActorCatalog, ResolvedNode},
    native::{
        CATEGORIES, Error, Result, TypedValue,
        layout_data::{NativeLayoutData, RootPosition},
    },
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn invalid(message: impl Into<String>) -> Error {
    Error::Unsupported(message.into())
}
fn text(node: &ResolvedNode<'_>, field: &str) -> Result<String> {
    match node.plain_string_field(field)? {
        Some(TypedValue::String { value, .. }) => Ok(value),
        _ => Err(invalid(format!(
            "recruit backreference required string {field}"
        ))),
    }
}
fn lookup_key(value: &str) -> String {
    value
        .split('\0')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Only the native layout fields consumed by460000 and460310.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayoutInput {
    pub properties: Vec<u32>,
    pub numbers: Vec<i32>,
    /// +28/+2C expanded slot-group indices; duplicates are intentional.
    pub groups: Vec<u32>,
    /// Deferred element references in creation order (slot, authored IDS).
    pub elements: Vec<(u32, String)>,
}
/// Typed source fields after successful ActorData LoadData, before resolution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActorInput {
    pub id: String,
    /// +174 bits7,8,13,19,21,30; no transport proxy participates.
    pub flags: u32,
    pub properties: Vec<u32>,
    pub research_items: bool,
    pub layout: Option<LayoutInput>,
    pub recruit_targets: Vec<String>,
    #[serde(default, skip_deserializing)]
    pub sources: Vec<Provenance>,
}
/// Native registration order and the final translated identity lookup for
/// deferred references. Owners remain registered identities, never aliases.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inputs {
    pub actors: Vec<ActorInput>,
    pub lookups: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct RecruitBackrefs {
    pub id: String,
    /// Completed+17C list, head first.460310 may retain duplicates.
    pub ids: Vec<String>,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct NativeRecruitBackrefs {
    pub definitions: BTreeMap<String, RecruitBackrefs>,
    /// Callback execution order: native address, owner canonical IDS, target IDS.
    pub resolution_order: Vec<(u32, String, String)>,
}
impl NativeRecruitBackrefs {
    /// Input must be an existing canonical registered identity. This never
    /// performs another Thing Find/Translate on a definition already obtained.
    pub fn definition(&self, canonical_id: &str) -> Result<&RecruitBackrefs> {
        self.definitions
            .get(&canonical_id.to_ascii_lowercase())
            .ok_or_else(|| {
                invalid(format!(
                    "canonical ActorData+17C identity unavailable: {canonical_id}"
                ))
            })
    }
    pub fn present(&self, canonical_id: &str) -> Result<bool> {
        Ok(!self.definition(canonical_id)?.ids.is_empty())
    }
}

#[derive(Clone)]
enum Deferred {
    Recruit {
        owner: usize,
        target: String,
    },
    Element {
        owner: usize,
        slot: u32,
        target: String,
    },
}
impl Inputs {
    /// Successful completed-data projection of460310/460000. Invalid source
    /// or unbound facts return errors, never an empty-list substitute. Partial
    /// native fault-side effects are not exposed by this completed-data API.
    pub fn complete(&self) -> Result<NativeRecruitBackrefs> {
        let mut index = BTreeMap::new();
        let mut deferred = Vec::new();
        let mut parameters: Vec<Option<Vec<Option<usize>>>> = Vec::new();
        let mut lists: Vec<Vec<usize>> = vec![vec![]; self.actors.len()];
        for (i, actor) in self.actors.iter().enumerate() {
            if index.insert(actor.id.to_ascii_lowercase(), i).is_some() {
                return Err(invalid("duplicate canonical recruitment definition"));
            }
            parameters.push(
                actor
                    .layout
                    .as_ref()
                    .map(|l| vec![None; l.properties.len()]),
            );
            if let Some(layout) = &actor.layout {
                if layout.properties.len() != layout.numbers.len() {
                    return Err(invalid("460000 layout entry metadata length mismatch"));
                }
                //464BD0 creates Organization references before Recruit refs.
                for (slot, target) in &layout.elements {
                    deferred.push(Deferred::Element {
                        owner: i,
                        slot: *slot,
                        target: target.clone(),
                    });
                }
            }
            if !actor.recruit_targets.is_empty() && actor.flags & (1 << 13) == 0 {
                return Err(invalid(format!(
                    "467D26 Recruit targets require StructureComponent: {}",
                    actor.id
                )));
            }
            for target in &actor.recruit_targets {
                deferred.push(Deferred::Recruit {
                    owner: i,
                    target: target.clone(),
                });
            }
        }
        let resolve = |authored: &str| -> Result<usize> {
            let key = lookup_key(authored);
            self.lookups
                .get(&key)
                .and_then(|canonical| index.get(&canonical.to_ascii_lowercase()))
                .copied()
                .ok_or_else(|| invalid(format!("4BDBA0 unknown ActorData reference {authored:?}")))
        };
        let mut order = Vec::new();
        for reference in deferred.into_iter().rev() {
            match reference {
                Deferred::Recruit { owner, target } => {
                    let target = resolve(&target)?;
                    order.push((
                        0x460310,
                        self.actors[owner].id.clone(),
                        self.actors[target].id.clone(),
                    ));
                    lists[target].insert(0, owner);
                    if self.actors[target].flags & (1 << 7) != 0 {
                        let layout = self.actors[target]
                            .layout
                            .as_ref()
                            .ok_or_else(|| invalid("460310 Organization+2F0 is unbound"))?;
                        let slots = parameters[target]
                            .as_ref()
                            .ok_or_else(|| invalid("460310 Organization parameters unbound"))?;
                        for &group in &layout.groups {
                            let element = slots
                                .get(group as usize)
                                .ok_or_else(|| invalid("460310 group index outside layout"))?;
                            if let Some(element) = *element {
                                //Native compares the list value to the element
                                //itself, not to the recruiter (4603F0).
                                if !lists[element].contains(&element) {
                                    lists[element].insert(0, owner);
                                }
                            }
                        }
                    }
                    //The non-Organization tail only logs unit-limit warnings;
                    //it cannot alter either recruitment list.
                }
                Deferred::Element {
                    owner,
                    slot,
                    target,
                } => {
                    let target = resolve(&target)?;
                    order.push((
                        0x460000,
                        self.actors[owner].id.clone(),
                        self.actors[target].id.clone(),
                    ));
                    let layout = self.actors[owner]
                        .layout
                        .as_ref()
                        .ok_or_else(|| invalid("460000 owner layout is unbound"))?;
                    let property = *layout
                        .properties
                        .get(slot as usize)
                        .ok_or_else(|| invalid("460000 slot outside layout"))?;
                    let has_property = self.actors[target].properties.contains(&property);
                    if has_property {
                        parameters[owner]
                            .as_mut()
                            .ok_or_else(|| invalid("460000 layout parameter owner"))?
                            [slot as usize] = Some(target);
                    }
                    if self.actors[target].flags & (1 << 8) == 0 {
                        return Err(invalid("460000 target is not an Element"));
                    }
                    if !has_property {
                        return Err(invalid("460000 target lacks slot Property"));
                    }
                    if self.actors[target].flags & (1 << 19) != 0
                        && layout.numbers[slot as usize] > 1
                    {
                        return Err(invalid("460000 packable Element must occupy its own slot"));
                    }
                    //Native walks the current owner head-first and prepends
                    //only references absent from the current target list.
                    for recruiter in lists[owner].clone() {
                        if !lists[target].contains(&recruiter) {
                            lists[target].insert(0, recruiter);
                        }
                    }
                    if self.actors[owner].flags & (1 << 30) != 0
                        && self.actors[target].flags & (1 << 21) != 0
                        && self.actors[target].research_items
                    {
                        return Err(invalid("460000 HQ Element has Research techs"));
                    }
                }
            }
        }
        let definitions = self
            .actors
            .iter()
            .enumerate()
            .map(|(i, a)| {
                (
                    a.id.to_ascii_lowercase(),
                    RecruitBackrefs {
                        id: a.id.clone(),
                        ids: lists[i]
                            .iter()
                            .map(|&j| self.actors[j].id.clone())
                            .collect(),
                        sources: a.sources.clone(),
                    },
                )
            })
            .collect();
        Ok(NativeRecruitBackrefs {
            definitions,
            resolution_order: order,
        })
    }
}

impl ActorCatalog {
    /// Read only native-order registered ActorData and provenance-bound roots.
    /// Final translations apply to references at consolidation, not to owners
    /// or to the earlier Property/Layout lookups made during LoadData.
    pub fn native_recruit_backref_inputs(&self, pc: Precision) -> Result<Inputs> {
        if self.native_data_registry("NameList").is_none() {
            return Err(invalid(
                "recruit backreferences require native registration order",
            ));
        }
        let mut roots = Vec::new();
        for tag in crate::actors::native_load::THING_TAGS {
            roots.extend(self.resolved_definitions(tag)?);
        }
        let mut data: Option<NativeLayoutData> = None;
        let mut actors = Vec::new();
        let mut lookups = BTreeMap::new();
        for registered in &self.actors {
            let node = roots
                .iter()
                .find(|n| n.sources().first() == Some(&registered.source))
                .ok_or_else(|| {
                    invalid(format!(
                        "canonical source root unavailable: {}",
                        registered.id
                    ))
                })?;
            let category = text(node, "category")?;
            if !CATEGORIES
                .iter()
                .any(|c| c.name.eq_ignore_ascii_case(&category) && c.native_kind == 2)
            {
                continue;
            }
            let at = RootPosition {
                file: self
                    .sources
                    .iter()
                    .position(|s| s.path.eq_ignore_ascii_case(&registered.source.path))
                    .ok_or_else(|| invalid("registered source not in load order"))?,
                offset: registered.source.span.start,
            };
            let org = node.child("OrganizationComponent")?;
            let props = node.child("PropertiesComponent")?;
            if data.is_none() && (org.is_some() || props.is_some()) {
                data = Some(
                    self.native_layout_data(pc)
                        .map_err(|e| invalid(e.to_string()))?,
                );
            }
            let mut properties = Vec::new();
            if let Some(props) = props {
                let data = data.as_ref().unwrap();
                for property in props.children("Property")? {
                    let ids = text(&property, "property_IDS")?;
                    let handle = data
                        .find_property(&ids, Some(at))
                        .ok_or_else(|| {
                            invalid(format!(
                                "unknown Property {ids:?} during {} LoadData",
                                registered.id
                            ))
                        })?
                        .handle()
                        .map_err(|e| invalid(e.to_string()))?;
                    properties.insert(0, handle);
                }
            }
            let layout = if let Some(org) = org {
                let data = data.as_ref().unwrap();
                let input = data
                    .organization_component_fields(&org, &registered.id, Some(at), pc)
                    .map_err(|e| invalid(e.to_string()))?;
                let layout = data
                    .layouts
                    .get(input.layout as usize)
                    .ok_or_else(|| invalid("registered OrganizationLayout ordinal"))?;
                if layout.error.is_some() {
                    return Err(invalid(
                        "completed OrganizationLayout has a native load error",
                    ));
                }
                Some(LayoutInput {
                    properties: layout
                        .entries
                        .iter()
                        .map(|entry| {
                            entry
                                .property
                                .ok_or_else(|| invalid("460000 layout entry Property is null"))?
                                .handle()
                                .map_err(|e| invalid(e.to_string()))
                        })
                        .collect::<Result<_>>()?,
                    numbers: layout.entries.iter().map(|e| e.number).collect(),
                    groups: layout.slot_groups.clone(),
                    elements: input.elements,
                })
            } else {
                None
            };
            let mut flags = 0;
            for (name, bit) in [
                ("OrganizationComponent", 7),
                ("ElementComponent", 8),
                ("StructureComponent", 13),
                ("MobileDeploymentComponent", 19),
                ("ResearchComponent", 21),
                ("HQComponent", 30),
            ] {
                if node.child(name)?.is_some() {
                    flags |= 1 << bit;
                }
            }
            let recruit_targets = match node.child("RecruitComponent")? {
                Some(recruit) => recruit
                    .children("RecruitActor")?
                    .iter()
                    .map(|r| text(r, "actor_ids"))
                    .collect::<Result<Vec<_>>>()?,
                None => vec![],
            };
            for reference in recruit_targets.iter().chain(
                layout
                    .iter()
                    .flat_map(|l| l.elements.iter().map(|(_, s)| s)),
            ) {
                if let Some(found) = self.find(reference.split('\0').next().unwrap_or_default()) {
                    lookups.insert(lookup_key(reference), found.id.clone());
                }
            }
            let research_items = match node.child("ResearchComponent")? {
                Some(r) => !r.children("ResearchItem")?.is_empty(),
                None => false,
            };
            actors.push(ActorInput {
                id: registered.id.clone(),
                flags,
                properties,
                research_items,
                layout,
                recruit_targets,
                sources: node.sources(),
            });
        }
        Ok(Inputs { actors, lookups })
    }
    /// Completed+17C arena. Call/cache this lazily when native501730 actually
    /// reaches the backreference read; unrelated scalar getters need no arena.
    pub fn native_recruit_backrefs(&self, pc: Precision) -> Result<NativeRecruitBackrefs> {
        self.native_recruit_backref_inputs(pc)?.complete()
    }
}

pub fn actor_recruit_backref_present(
    catalog: &ActorCatalog,
    canonical_id: &str,
    pc: Precision,
) -> Result<bool> {
    catalog.native_recruit_backrefs(pc)?.present(canonical_id)
}
