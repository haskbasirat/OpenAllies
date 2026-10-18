    pub fn or_registration_flags(&mut self, key: ActorKey, bits: u32) -> Result<()> {
        if !matches!(bits, 0x08000000 | 0x10000000 | 0x18000000) {
            return Err(Error::World("registration OR requires native bits27/28".into()));
        }
        self.member_mut(key)?.actor.flags |= bits;
        Ok(())
    }
