    pub fn or_registration_flags(&mut self, object:ObjectKey, bits:u32)->Result<()> {
        let key=self.key(object)?;
        self.index.or_registration_flags(key,bits)?;
        Ok(())
    }
