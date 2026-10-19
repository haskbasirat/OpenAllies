//! ObjectData's six source FX-list inputs, before any FXC compilation.
//!
//!49C760 constructs six zero head/tail pairs at+BC;49DC40 loads View lists
//! in4979D0 enum order through42BF10. In the single-declaration domain an
//! absent View or zero source count proves a null list. A nonempty authored
//! list is only preserved input: it requires the actual FXC loader/owner.
//! Shared mode1 reload/appends are explicitly refused here.
use crate::catalog::Catalog;
use aa_mapworld::restore::RuntimeDefinition;
use aa_numeric::Precision;
use aa_rules::resources::Provenance;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum ListKind {
    Created = 0,
    Destroyed = 1,
    Selected = 2,
    Commanded = 3,
    CommandedAggressive = 4,
    Constructed = 5,
}
impl ListKind {
    pub const ALL: [Self; 6] = [
        Self::Created,
        Self::Destroyed,
        Self::Selected,
        Self::Commanded,
        Self::CommandedAggressive,
        Self::Constructed,
    ];
    pub fn offset(self) -> u32 {
        0xbc + (self as u32) * 8
    }
    pub fn node_name(self) -> &'static str {
        match self {
            Self::Created => "WhenCreated",
            Self::Destroyed => "WhenDestroyed",
            Self::Selected => "WhenSelected",
            Self::Commanded => "WhenCommanded",
            Self::CommandedAggressive => "WhenCommandedAggressive",
            Self::Constructed => "WhenConstructed",
        }
    }
}
/// Source inputs only. Empty is certified by this private producer; positive
/// entries are not advertised as completed native FXC graph nodes.
#[derive(Clone, Debug)]
pub struct InputList {
    kind: ListKind,
    entries: Vec<Vec<Provenance>>,
}
impl InputList {
    pub fn kind(&self) -> ListKind {
        self.kind
    }
    pub fn native_offset(&self) -> u32 {
        self.kind.offset()
    }
    pub fn is_certified_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn authored_entries(&self) -> &[Vec<Provenance>] {
        &self.entries
    }
}
/// Immutable source owner; safe to share behind Arc. No native renderer/queue
/// object or mutable head pointer is duplicated by this value.
#[derive(Clone, Debug)]
pub struct Source {
    canonical_id: String,
    source: Provenance,
    source_precision: Precision,
    view_present: bool,
    lists: [InputList; 6],
}
impl Source {
    pub fn canonical_id(&self) -> &str {
        &self.canonical_id
    }
    pub fn definition_source(&self) -> &Provenance {
        &self.source
    }
    pub fn source_precision(&self) -> Precision {
        self.source_precision
    }
    pub fn view_present(&self) -> bool {
        self.view_present
    }
    pub fn list(&self, kind: ListKind) -> &InputList {
        &self.lists[kind as usize]
    }
    pub fn bind(catalog: &Catalog<'_>, definition: &RuntimeDefinition) -> Result<Self, String> {
        let source_precision = {
            let bank = catalog.definition_bank.try_borrow()
                .map_err(|_| "nested ObjectData FX source Bank")?;
            Self::bank_precision(catalog,&bank)?
        };
        Self::bind_at(catalog,definition,source_precision,false)
    }
    ///Same immutable source producer after the canonical Bank moves to Staged.
    ///It never temporarily reinstalls a Bank into the post-staging Catalog.
    pub fn bind_in_bank(catalog:&Catalog<'_>,bank:&crate::definitions::Bank,definition:&RuntimeDefinition)->Result<Self,String>{
        let source_precision=Self::bank_precision(catalog,bank)?;
        let portrait=definition.native_class==0;
        if portrait {crate::data_pre_request::validate_object_data(catalog,bank,definition)?;}
        Self::bind_at(catalog,definition,source_precision,portrait)
    }
    fn bank_precision(catalog:&Catalog<'_>,bank:&crate::definitions::Bank)->Result<Precision,String>{
        catalog.require_source_owner()?;
        let state=bank.combat_source()?;
        if !crate::catalog_lookups::same_sources(catalog,state.source_identities()){return Err("ObjectData FX source FP owner differs".into())}
        Ok(state.source_precision())
    }
    fn bind_at(catalog:&Catalog<'_>,definition:&RuntimeDefinition,source_precision:Precision,portrait:bool)->Result<Self,String>{
        catalog.require_source_owner()?;
        //ObjectData49DC40 reads the View FX lists for every ObjectData-derived
        //data kind; the runtime class created later (GROUP is class8) is not
        //an input to this source producer.
        let proved_class = matches!(definition.native_class, 2 | 3 | 4 | 6 | 7 | 9);
        let object_data = matches!(definition.proof.category.native_kind, 1 | 2 | 20 | 21)
            && definition.native_class != 0;
        if !(proved_class || object_data || portrait) {
            return Err("ObjectData FX source requires ObjectData-derived definition data".into());
        }
        let proof = &definition.proof;
        let canonical = catalog
            .actors
            .actors
            .iter()
            .find(|a| a.id.eq_ignore_ascii_case(&proof.id))
            .ok_or("ObjectData FX canonical identity is absent")?;
        //4BDC50 passes the registered allocation to vf0 LoadData, so a later
        //Thing translation of this IDS does not change the source read here.
        let thing = aa_rules::actors::ThingRef::registered(&proof.id);
        let found = catalog
            .actors
            .select(thing)
            .ok_or("ObjectData FX canonical lookup is absent")?;
        if !found.id.eq_ignore_ascii_case(&canonical.id) || found.source != canonical.source {
            return Err("ObjectData FX registered allocation differs from its canonical entry".into());
        }
        if canonical.source != proof.source
            || canonical.template_chain != proof.template_chain
            || catalog
                .actors
                .native_category(thing)
                .map_err(|e| e.to_string())?
                != proof.category
            || proof.category.runtime_class != definition.native_class
        {
            return Err("ObjectData FX canonical definition/source differs".into());
        }
        // Constructor-empty is not the correct proof for a reloaded shared list.
        let declarations = catalog
            .actors
            .thing_ids_declarations(&proof.id, source_precision)
            .map_err(|e| format!("FX reload census: {e}"))?;
        if declarations != 1 {
            return Err("ObjectData FX requires actual shared reload/list ownership".into());
        }
        let actor = catalog
            .actors
            .resolved_actor(thing)
            .map_err(|e| e.to_string())?;
        let view = actor.child("View").map_err(|e| e.to_string())?;
        let mut lists = Vec::new();
        for kind in ListKind::ALL {
            let entries = match &view {
                Some(view) => view
                    .children(kind.node_name())
                    .map_err(|e| e.to_string())?
                    .iter()
                    .map(|n| n.sources())
                    .collect(),
                None => Vec::new(),
            };
            lists.push(InputList { kind, entries });
        }
        Ok(Self {
            canonical_id: canonical.id.clone(),
            source: canonical.source.clone(),
            source_precision,
            view_present: view.is_some(),
            lists: lists
                .try_into()
                .map_err(|_| "six native ObjectData FX lists")?,
        })
    }
}
