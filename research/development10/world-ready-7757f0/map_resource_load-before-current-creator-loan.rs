//! Move-only companion for the ONE actual MapResource+A8 allocation.
//!
//! Fresh factory5A28B0 always constructs5A27A0;5A283C stores+A8=0.
//! LoadMap5A2BDF..5A2BF1 saves that prior word and writes its load argument.
//! Its normal return5A3110..5A311A restores the retained prior word before
//! LoadGameTask5A3ED0 calls StartMission. Only the concrete fresh World factory
//! can create this owner; no public raw-word or "ready" constructor exists.
use aa_rules::resources::SourceIdentity;

#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Loading,
    StreamsClosed,
    Returned,
}

/// The mutable mode itself remains in the root World's stable Box. This
/// companion owns the captured prior word and lifecycle, never a second mode.
/// It has no Clone/Default implementation.
pub struct Owner {
    address: usize,
    prior: u32,
    load_mode: u32,
    source_files: Vec<SourceIdentity>,
    archive: Option<crate::map_resource_streams::ArchiveWitness>,
    /// Fresh memory-backed resource's actual+08 word. Other backend/request
    /// mutations require their own integration; this is not a native depot.
    flags: u32,
    /// Actual MapResource+88. None is explicitly UNBOUND, not native null:
    ///5A2854 clones the ctor's selected Creator and5A2859 stores it here.
    ///5A2A40 later replaces it with the appended source object.
    creator: Option<Box<crate::resource_creator::Creator>>,
    creator_constructor_bound: bool,
    selected_creator: Option<crate::resource_creator::CreatorWitness>,
    creator_header_loaded: bool,
    creator_load_completed: bool,
    phase: Phase,
}

impl Owner {
    pub(crate) fn fresh_loading(
        load_mode: u32,
        source_files: &[SourceIdentity],
    ) -> Result<(Self, Box<Option<u32>>), String> {
        Self::fresh_loading_archive(load_mode, source_files, None)
    }

    pub(crate) fn fresh_loading_archive(
        load_mode: u32,
        source_files: &[SourceIdentity],
        archive: Option<crate::map_resource_streams::ArchiveWitness>,
    ) -> Result<(Self, Box<Option<u32>>), String> {
        if !matches!(load_mode, 1 | 2 | 4) {
            return Err("managed MapResource loading requires its actual task mode1/2/4".into());
        }
        if source_files.is_empty() {
            return Err("managed MapResource requires its actual source identity universe".into());
        }
        // Exact fresh5A27A0 store, not an inferred existing-resource default.
        let mut word = Box::new(Some(0));
        let prior = word.ok_or("fresh MapResource constructor did not initialize+A8")?;
        let address = std::ptr::from_ref(word.as_ref()).addr();
        *word = Some(load_mode);
        // Base5AA890 sets0x10; derived5A2848 ORs8. This is the private fresh
        // read-only memory backend; no unported depot/request mutation runs.
        Ok((
            Self {
                address,
                prior,
                load_mode,
                source_files: source_files.to_vec(),
                archive,
                flags: 0x18,
                creator: None,
                creator_constructor_bound: false,
                selected_creator: None,
                creator_header_loaded: false,
                creator_load_completed: false,
                phase: Phase::Loading,
            },
            word,
        ))
    }

    fn identity(&self, word: &Option<u32>, sources: &[SourceIdentity]) -> Result<(), String> {
        if std::ptr::from_ref(word).addr() != self.address {
            return Err("MapResource companion belongs to another actual mode allocation".into());
        }
        if sources != self.source_files {
            return Err("MapResource companion belongs to another source universe".into());
        }
        Ok(())
    }

    pub(crate) fn validate_loading(
        &self,
        word: &Option<u32>,
        sources: &[SourceIdentity],
    ) -> Result<(), String> {
        self.identity(word, sources)?;
        if self.phase != Phase::Loading || *word != Some(self.load_mode) {
            return Err(
                "MapResource load interval is not the retained current loading mode".into(),
            );
        }
        Ok(())
    }

    /// Only call after the real resource-load callbacks and4FC600 conversion
    /// tail returned. Failed/unreached loads retain their owner and current word.
    pub(crate) fn finish_normal_return(
        &mut self,
        word: &mut Option<u32>,
        sources: &[SourceIdentity],
    ) -> Result<(), String> {
        self.identity(word, sources)?;
        if self.phase != Phase::StreamsClosed || *word != Some(self.load_mode) {
            return Err(
                "MapResource normal return requires its actual completed stream closes".into(),
            );
        }
        *word = Some(self.prior);
        self.phase = Phase::Returned;
        Ok(())
    }

    pub(crate) fn validate_archive(&self, map: &aa_mapworld::MapArchive) -> Result<(), String> {
        if let Some(archive) = &self.archive {
            archive.validate_map(map)?;
        }
        Ok(())
    }

    /// Source5A2A40 installs+88, then5A2FCF..5A3013 replaces a non-Load
    /// Creator ONLY in mode1 by cloning the actual selected CreatorLoad.
    /// The selected descriptor is supplied by its launch-state owner; neither
    /// a MAP_HEADER parameters projection nor absent settings substitute for it.
    /// If selection is missing/invalid, the parsed+88 owner remains installed.
    pub(crate) fn prepare_creator_header(
        &mut self,
        catalog: &crate::catalog::Catalog<'_>,
        map: &aa_mapworld::MapArchive,
        selected: Option<&crate::resource_creator::Creator>,
    ) -> Result<(), String> {
        if self.phase != Phase::Loading || self.creator.is_some() {
            return Err("MapResource Creator source load is repeated or outside loading".into());
        }
        self.archive
            .as_ref()
            .ok_or("actual Creator load requires retained resource archive")?
            .validate_map(map)?;
        if let Some(selected) = selected {
            let bank = catalog
                .definition_bank
                .try_borrow()
                .map_err(|_| "source Bank already borrowed")?;
            //5A2831..5A2859: ctor arg is the actual Creator itself, not its+8.
            self.creator = Some(selected.clone_for_resource(catalog, &bank)?);
            self.creator_constructor_bound = true;
            self.selected_creator = Some(selected.witness());
            if selected.kind() == crate::resource_creator::Kind::Load {
                //5A2D09..5A2D12 dereferences param2+50 before reading the
                //source header. A valid unselected ctor cannot stand in for it.
                let source = selected
                    .source_resource()
                    .ok_or("5A2D09 actual selected CreatorLoad+50 source MapResource is absent")?;
                if !self
                    .archive
                    .as_ref()
                    .is_some_and(|archive| archive.same_owner(source.archive_witness()))
                {
                    return Err(
                        "selected CreatorLoad source differs from the actual retained map archive"
                            .into(),
                    );
                }
            }
        }
        self.creator = Some(crate::resource_creator::Creator::load_appended(
            catalog, map,
        )?);
        if !self.creator_constructor_bound {
            return Err("MapResource actual constructor selected Creator clone is unbound; parsed+88 retained".into());
        }
        self.creator_header_loaded = true;
        Ok(())
    }

    ///5A2FCF..5A3013 executes AFTER the actual GWorld constructor returned,
    ///before4FC600. It clones the current SAME selected Creator allocation.
    pub(crate) fn select_creator_before_world_load(
        &mut self,
        catalog: &crate::catalog::Catalog<'_>,
        selected: &crate::resource_creator::Creator,
    ) -> Result<(), String> {
        if self.phase != Phase::Loading
            || !self.creator_constructor_bound
            || !self.creator_header_loaded
            || self.creator_load_completed
            || !self
                .selected_creator
                .as_ref()
                .is_some_and(|w| w.matches(selected))
        {
            return Err("MapResource final Creator selection requires the same actual selected constructor owner and parsed header".into());
        }
        if self.load_mode == 1
            && self
                .creator
                .as_ref()
                .is_some_and(|v| v.kind() != crate::resource_creator::Kind::Load)
        {
            if selected.kind() != crate::resource_creator::Kind::Load {
                return Err("selected map launch requires the actual CreatorLoad subclass".into());
            }
            let bank = catalog
                .definition_bank
                .try_borrow()
                .map_err(|_| "source Bank already borrowed")?;
            selected.validate(catalog, &bank)?;
            //All preflight above precedes the native deletion/copy operations.
            //No allocator/SEH parity is claimed for this safe value transfer.
            self.creator = None;
            self.creator = Some(selected.clone_for_resource(catalog, &bank)?);
        }
        self.creator_load_completed = true;
        Ok(())
    }

    /// A bounded composition for owner tests/diagnostics. Real launch uses the
    /// two methods separately around its actual GWorld constructor callback.
    pub(crate) fn load_creator_before_world(
        &mut self,
        catalog: &crate::catalog::Catalog<'_>,
        map: &aa_mapworld::MapArchive,
        selected: Option<&crate::resource_creator::Creator>,
    ) -> Result<(), String> {
        self.prepare_creator_header(catalog, map, selected)?;
        self.select_creator_before_world_load(
            catalog,
            selected.ok_or("actual selected Creator is absent")?,
        )
    }

    pub fn creator(&self) -> Result<&crate::resource_creator::Creator, String> {
        self.creator
            .as_deref()
            .ok_or_else(|| "actual current MapResource+88 Creator is absent".into())
    }

    pub(crate) fn validate_creator(
        &self,
        catalog: &crate::catalog::Catalog<'_>,
        bank: &crate::definitions::Bank,
    ) -> Result<(), String> {
        if !self.creator_constructor_bound || !self.creator_load_completed {
            return Err(
                "MapResource constructor/source Creator interval is unbound or incomplete".into(),
            );
        }
        self.creator()?.validate(catalog, bank)
    }

    pub(crate) fn close_owned_streams(
        &mut self,
        streams: &mut crate::map_resource_streams::Owner,
    ) -> Result<(), String> {
        if self.phase != Phase::Loading {
            return Err("MapResource stream-close boundary is repeated or unavailable".into());
        }
        let archive = self
            .archive
            .as_ref()
            .ok_or("borrowed map diagnostics cannot certify an owned resource return")?;
        streams.validate_open_for(archive)?;
        streams.close_input()?;
        streams.close_embedded()?;
        // Actual5AA6F0 then5A3101, no callback between these scalar writes.
        let previous = self.flags;
        self.flags = previous | 4;
        self.flags = previous | 6;
        self.flags |= 1;
        self.phase = Phase::StreamsClosed;
        Ok(())
    }

    pub(crate) fn validate_returned(
        &self,
        word: &Option<u32>,
        sources: &[SourceIdentity],
    ) -> Result<(), String> {
        self.identity(word, sources)?;
        if self.phase != Phase::Returned || *word != Some(self.prior) {
            return Err("MapResource normal-return prior word is not current".into());
        }
        Ok(())
    }

    /// Constructor-captured native word, exposed for a diagnostic receipt only.
    pub fn prior_word(&self) -> u32 {
        self.prior
    }
    pub fn load_mode(&self) -> u32 {
        self.load_mode
    }
    pub fn flags(&self) -> u32 {
        self.flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_v14_factory_and_same_allocation_restore() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-mission-start/native-map-resource.json"
        ))
        .unwrap();
        assert_eq!(fixture["cases"].as_array().unwrap().len(), 6);
        let sources = vec![SourceIdentity {
            path: "explicit-owner-identity-fixture".into(),
            sha256: "fixture".into(),
            origin: aa_rules::sources::Origin::V14Loose,
        }];
        for row in fixture["cases"].as_array().unwrap() {
            assert_eq!(row["factory"]["mode_a8"], 0);
            assert_eq!(row["factory"]["registered_same_allocation"], true);
            assert_eq!(row["world_constructor_same_resource"], true);
            assert_eq!(row["factory"]["flags_08"], 0x18);
            for item in row["mode_intervals"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| v["initial"] == 0)
            {
                let mode = item["load_mode"].as_u64().unwrap() as u32;
                let mut bytes = b"TGCK".to_vec();
                bytes.extend(2u32.to_le_bytes());
                bytes.extend(0u32.to_le_bytes());
                let mut streams = crate::map_resource_streams::Owner::open(
                    aa_mapworld::MapArchive::from_bytes(bytes).unwrap(),
                );
                let (mut actual, mut word) =
                    Owner::fresh_loading_archive(mode, &sources, Some(streams.witness())).unwrap();
                actual.validate_archive(streams.archive().unwrap()).unwrap();
                assert_eq!(
                    actual.prior_word(),
                    item["saved_prior"].as_u64().unwrap() as u32
                );
                assert_eq!(*word, Some(item["during"].as_u64().unwrap() as u32));
                actual.validate_loading(word.as_ref(), &sources).unwrap();
                let mut foreign = Box::new(*word);
                assert!(
                    actual
                        .finish_normal_return(foreign.as_mut(), &sources)
                        .is_err()
                );
                assert!(actual.finish_normal_return(word.as_mut(), &[]).is_err());
                assert!(
                    actual
                        .finish_normal_return(word.as_mut(), &sources)
                        .is_err()
                );
                actual.close_owned_streams(&mut streams).unwrap();
                assert!(streams.is_closed());
                assert_eq!(actual.flags(), 0x1f);
                actual
                    .finish_normal_return(word.as_mut(), &sources)
                    .unwrap();
                assert_eq!(*word, Some(item["after"].as_u64().unwrap() as u32));
                actual.validate_returned(word.as_ref(), &sources).unwrap();
                assert!(
                    actual
                        .finish_normal_return(word.as_mut(), &sources)
                        .is_err()
                );
                let moved = (actual, word);
                moved
                    .0
                    .validate_returned(moved.1.as_ref(), &sources)
                    .unwrap();
            }
        }
    }
}
