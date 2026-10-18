    ///43F0F1 reads the Biome pointer already retained by this Creator+44 and
    ///GMap+24. Resolve that exact registration, never Find its IDS again.
    pub(crate) fn retained_biome_registration<'a>(
        &self,
        catalog: &'a Catalog<'_>,
        bank: &Bank,
    ) -> Result<&'a aa_rules::actors::native_load::NativeDataEntry, String> {
        catalog.require_source_owner()?;
        self.validate(catalog, bank)?;
        let source = bank.combat_source()?;
        if source.source_identities() != catalog.actors.sources.as_slice() {
            return Err("Creator retained Biome belongs to another completed source Bank".into());
        }
        let named = self.biome.as_ref().ok_or("VWorld retained Biome pointer is null")?;
        let registry = catalog.actors.native_data_registry("Biome")
            .ok_or("Creator retained Biome registry absent")?;
        let entry = registry.entries.iter().find(|entry| {
            entry.ids == named.ids && entry.path == named.path
                && entry.source == named.source && entry.reloads.is_empty()
        }).ok_or("Creator retained canonical Biome differs")?;
        catalog.actors.native_data_node(entry).map_err(err)?;
        Ok(entry)
    }
