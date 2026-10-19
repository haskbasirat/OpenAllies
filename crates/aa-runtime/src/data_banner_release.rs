//!Banner 46C7D0 / 46C800 (Plane 46C860/46C890, Decal 46C920/46C950) over the
//!SAME selected Banner source that 46C7A0 requests. Kernel/proof:
//!data_release_kernels.
use super::*;
use crate::data_pre_request::release_kernels::{self as k, ResourceRelease};
fn planes(core: &fields::Core<Handle>) -> Vec<Vec<Vec<Handle>>> {
    core.planes.iter().map(|p| p.decals.iter().map(|d| d.textures.clone()).collect()).collect()
}
impl State {
    fn completed_banner(&self, catalog: &Catalog<'_>, bank: &Bank, r: &Reference) -> Result<&Banner> {
        let source = self.banner(catalog, bank, r)?;
        if !source.complete {
            return Err(source.failed.clone().unwrap_or_else(|| "Banner selected source fields unfinished".into()));
        }
        Ok(source)
    }
    ///46C7D0
    pub(in crate::data_pre_request) fn release_banner(
        &self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        r: &Reference,
        delta: u32,
        res: &mut impl ResourceRelease,
    ) -> Result<()> {
        let planes = planes(&self.completed_banner(catalog, bank, r)?.core);
        k::banner_release(&planes, delta, |h, d| res.release_14(h, d))
    }
    ///46C800
    pub(in crate::data_pre_request) fn banner_ready(
        &self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        r: &Reference,
        res: &mut impl ResourceRelease,
    ) -> Result<bool> {
        let planes = planes(&self.completed_banner(catalog, bank, r)?.core);
        k::banner_ready(&planes, |h| res.loaded(h))
    }
}
