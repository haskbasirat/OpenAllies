    ///43F0F1->5AE0A0 over this SAME manager. All currently admitted cache
    ///entries are typed MapResource/FilmResource allocations, including map
    ///Pending/Failed prefixes. Their class chains exclude NiAVObjResource,
    ///so5ACA50 skips each before reading flags/counts or unloading anything.
    ///A modern GPU cache may not be silently attached to this closed namespace.
    pub fn set_vworld_resource_context(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
        biome: &crate::vworld_resource_context::Biome,
    ) -> Result<crate::vworld_resource_context::Update> {
        let input = crate::vworld_resource_context::Input::bind(catalog, bank, biome)?;
        let mut manager = self.manager.lock().map_err(|_| "resource manager lock poisoned")?;
        let Manager { films, texture_subfolder_80, .. } = &mut *manager;
        texture_subfolder_80.assign_and_invalidate(&input.units, || {
            //These private owners have no generic insertion API. Native
            //Map B3D7DC->Resource B38E24 and Film B3D7F0->Map chains cannot
            //reach the unrelated NiAVObj descriptor B3D990. Bucket order
            //has no observable effect when EVERY entry takes that skip.
            let maps = self.maps.keys().count();
            let films = films.values().count();
            Ok((maps, films))
        })
    }
    ///A copied diagnostic value, never another mutable manager authority.
    pub fn vworld_texture_subfolder(&self) -> Result<Vec<u16>> {
        Ok(self.manager.lock().map_err(|_| "resource manager lock poisoned")?
            .texture_subfolder_80.current().to_vec())
    }
