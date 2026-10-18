    /// Post-staging callers borrow the same moved lifecycle owner directly.
    /// This read remains lazy at the native SAI+20 gate.
    pub(crate) fn saved_observer_active_in_load(
        &self,
        load: &crate::restore_lifecycle::InactiveLoad,
    ) -> Result<bool, String> {
        self.require_managed_mode(load)?;
        load.observer_active()
    }
