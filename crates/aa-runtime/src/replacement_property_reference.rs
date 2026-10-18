//!Captured source identity transport for the reached4CEF10 child only.
//!No lookup, registration, Property instance or gameplay state is created here.
use crate::{definitions::Bank,map_replacement_queue::DefinitionRef};
use aa_body::credit::Object;
pub(super) fn token(bank:&Bank,definition:&DefinitionRef)->Result<Object,String>{
    let entry=bank.require_usable(definition.token)?;
    if entry.key.domain!="property"||!entry.key.name.eq_ignore_ascii_case(&definition.canonical_ids){
        return Err("4CEF10 captured Property Bank token/domain/IDS differ".into());
    }
    Ok(Object(definition.token))
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn captured_property_rejects_foreign_domain_ids_and_missing_token(){
        let mut bank=Bank::default();
        let property=bank.intern("Property","MiXeD_Requirement").unwrap();
        let actor=bank.intern("ActorDefinition","MiXeD_Requirement").unwrap();
        let input=DefinitionRef{token:property,canonical_ids:"MIXED_REQUIREMENT".into()};
        assert_eq!(token(&bank,&input).unwrap(),Object(property));
        assert!(token(&bank,&DefinitionRef{token:actor,..input.clone()}).is_err());
        assert!(token(&bank,&DefinitionRef{canonical_ids:"another_property".into(),..input.clone()}).is_err());
        assert!(token(&bank,&DefinitionRef{token:u32::MAX,..input}).is_err());
        assert_eq!(bank.token("Property","mixed_requirement"),Some(property));
    }
}
