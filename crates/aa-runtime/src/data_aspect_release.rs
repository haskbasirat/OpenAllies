//!Aspect 469BB0 (release) / 469C50 (ready) over the SAME Aspect rows that
//!46AEC0 requests. Kernel and proof: data_release_kernels.
use super::*;
use crate::data_pre_request::release_kernels::{self as k, ResourceRelease};
struct Bound<'a, R> {
    row: &'a mut Row,
    res: &'a mut R,
}
const EXTRA: &str = "469BB0/469C50 +54 loaded-Model extra textures need the unported 46AE20 +48 owner";
impl<R: ResourceRelease> k::AspectRelease for Bound<'_, R> {
    type Resource = Handle;
    fn count(&mut self) -> Result<u32> {
        Ok(self.row.count)
    }
    fn store_count(&mut self, value: u32) -> Result<()> {
        self.row.count = value;
        Ok(())
    }
    fn model(&mut self) -> Result<Handle> {
        self.row
            .model
            .clone()
            .ok_or_else(|| "469BB0/469C50 Aspect Model28 is null (native null dereference)".into())
    }
    fn release(&mut self, r: &Handle, delta: u32) -> Result<()> {
        self.res.release_14(r, delta)
    }
    fn loaded(&mut self, r: &Handle) -> Result<bool> {
        self.res.loaded(r)
    }
    fn release_animations(&mut self, delta: u32) -> Result<()> {
        let Self { row, res } = self;
        match &row.animations {
            Some(set) => set.release(delta, |h, d| res.release_14(h, d)),
            None => Ok(()),
        }
    }
    fn animations_ready(&mut self) -> Result<bool> {
        let Self { row, res } = self;
        match &row.animations {
            Some(set) => set.ready(|h| res.loaded(h)),
            None => Ok(true),
        }
    }
    fn skin(&mut self, index: usize) -> Result<Option<Handle>> {
        Ok(self.row.skins[index].clone())
    }
    fn extra_requested(&mut self) -> Result<bool> {
        Ok(self.row.extra_requested)
    }
    fn clear_extra_requested(&mut self) -> Result<()> {
        self.row.extra_requested = false;
        Ok(())
    }
    fn extra_count(&mut self, _: &Handle) -> Result<i32> {
        Err(EXTRA.into())
    }
    fn extra(&mut self, _: usize) -> Result<Handle> {
        Err(EXTRA.into())
    }
}
impl State {
    fn completed_aspect_row(&mut self, catalog: &Catalog<'_>, bank: &Bank, aspect: &Aspect) -> Result<&mut Row> {
        self.entry(&aspect.data)?;
        let object = self
            .entries
            .get_mut(&aspect.data.token)
            .unwrap()
            .aspects
            .as_mut()
            .ok_or("Aspect allocation owner absent")?;
        crate::data_pre_request::validate_object_data(catalog, bank, &object.definition)?;
        if !object.complete {
            return Err("Aspect source fields have not completed".into());
        }
        object.rows.get_mut(aspect.index).ok_or_else(|| "Aspect index absent".into())
    }
    ///469BB0
    pub(in crate::data_pre_request) fn release_aspect(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        aspect: &Aspect,
        delta: u32,
        res: &mut impl ResourceRelease,
    ) -> Result<()> {
        let row = self.completed_aspect_row(catalog, bank, aspect)?;
        k::aspect_release(&mut Bound { row, res }, delta)
    }
    ///469C50
    pub(in crate::data_pre_request) fn aspect_ready(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        aspect: &Aspect,
        res: &mut impl ResourceRelease,
    ) -> Result<bool> {
        let row = self.completed_aspect_row(catalog, bank, aspect)?;
        k::aspect_ready(&mut Bound { row, res })
    }
}
