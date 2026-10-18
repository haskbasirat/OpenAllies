    pub(crate) fn verified_placeholder_flags(
        &self,id:&str,fields:&aa_rules::native_source_requirements::NativeSourceRequirements,
    )->Result<u32,String>{
        let mut bank=self.definition_bank.try_borrow_mut().map_err(|e|e.to_string())?;
        if bank.replacement_flags.is_none(){
            bank.replacement_flags=Some(crate::replacement_flags_source::State::bind(self,&mut bank,fields)?);
        }
        self.verified_placeholder_flags_in_bank(&bank,id,fields)
    }

    pub(crate) fn verified_placeholder_flags_in_bank(
        &self,bank:&crate::definitions::Bank,id:&str,
        fields:&aa_rules::native_source_requirements::NativeSourceRequirements,
    )->Result<u32,String>{
        self.require_source_owner()?;
        let source=bank.combat_source()?;
        if fields.receipt().sources!=self.actors.sources
            ||fields.source_precision()!=source.source_precision()
            ||source.source_identities()!=self.actors.sources.as_slice(){
            return Err("Placeholder cached read source-field writer universe or PC differs".into());
        }
        bank.replacement_flags.as_ref().ok_or("actual Bank has no completed Placeholder fixed-point storage")?.flags(self,bank,id)
    }
