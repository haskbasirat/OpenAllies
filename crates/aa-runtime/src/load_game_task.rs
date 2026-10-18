//!59E6F0 task+1C retains the SAME selected Creator, not a clone. Kind+20
//!controls the native5A4280 path; only kind1 copies selected+50 into+24.
//!This opaque input is neither queued execution nor map/task completion.
use crate::{
    catalog::Catalog,
    definitions::Bank,
    map_resource_mode::LoadGameTaskKind,
    resource_creator::{Creator, CreatorWitness, Kind, SourceResource},
};
use std::sync::Arc;
pub struct Inputs {
    selected: CreatorWitness,
    source: Arc<SourceResource>,
    kind: LoadGameTaskKind,
}
impl Inputs {
    pub fn from_selected_creator(
        catalog: &Catalog<'_>,
        bank: &Bank,
        creator: &Creator,
        kind: LoadGameTaskKind,
    ) -> Result<Self, String> {
        creator.validate(catalog, bank)?;
        if creator.kind() != Kind::Load {
            return Err("59E6F0 requires actual selected LoadCreator".into());
        }
        let source = Arc::clone(
            creator
                .source_resource()
                .ok_or("59E6F0 selected Creator has no actual source+50")?,
        );
        Ok(Self {
            selected: creator.witness(),
            source,
            kind,
        })
    }
    pub fn kind(&self) -> LoadGameTaskKind {
        self.kind
    }
    pub fn source(&self) -> &Arc<SourceResource> {
        &self.source
    }
    pub fn validate_creator(
        &self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        selected: &Creator,
    ) -> Result<(), String> {
        selected.validate(catalog, bank)?;
        if !self.selected.matches(selected)
            || !selected
                .source_resource()
                .is_some_and(|s| Arc::ptr_eq(s, &self.source))
        {
            return Err(
                "task input does not refer to SAME selected Creator/source allocation".into(),
            );
        }
        Ok(())
    }
    pub(crate) fn selected_witness(&self) -> &CreatorWitness {
        &self.selected
    }
}
