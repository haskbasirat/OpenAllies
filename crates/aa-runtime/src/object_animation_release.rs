//!49B820 (release) / 49B860 (ready) over the SAME ObjectAnimationSet entry
//!+1C Resources that 49B7E0 requests. Kernel/proof: data_release_kernels.
use super::*;
use crate::data_pre_request::release_kernels as k;
type Result<T> = std::result::Result<T, String>;
impl Set {
    fn entries(&self) -> Result<Vec<Option<Handle>>> {
        self.data.require_live()?;
        if let Some(error) = &self.failed {
            return Err(format!("ObjectAnimationSet source load failed: {error}"));
        }
        Ok(self.core.animations.iter().map(|a| a.resource.clone()).collect())
    }
    pub(crate) fn release(&self, delta: u32, release: impl FnMut(&Handle, u32) -> Result<()>) -> Result<()> {
        k::set_release(&self.entries()?, delta, release)
    }
    pub(crate) fn ready(&self, loaded: impl FnMut(&Handle) -> Result<bool>) -> Result<bool> {
        k::set_ready(&self.entries()?, loaded)
    }
}
