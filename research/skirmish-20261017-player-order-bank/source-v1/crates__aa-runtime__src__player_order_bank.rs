//!A scoped loan of the SAME moved definition Bank. Reads release before an
//!actor/Mind callback, so reentrant state/property effects may borrow it again.
use super::{Error,Result};
use crate::definitions::Bank;
use std::cell::{Ref,RefCell,RefMut};

///Owns only a temporary mutable reference, never another Bank/cache/arena.
///The actual Frame supplies its retained Bank; legacy Catalog APIs keep their
///original lazy per-operation borrow and never substitute for a moved owner.
pub struct DefinitionBankLoan<'bank>{cell:RefCell<&'bank mut Bank>}
impl<'bank> DefinitionBankLoan<'bank>{
    pub fn new(bank:&'bank mut Bank)->Self{Self{cell:RefCell::new(bank)}}
    pub fn with_bank<R>(&self,call:impl FnOnce(&Bank)->std::result::Result<R,String>)->std::result::Result<R,String>{
        let bank=self.cell.try_borrow().map_err(|e|format!("actual command Bank loan: {e}"))?;
        call(&bank)
    }
    pub fn with_bank_mut<R>(&self,call:impl FnOnce(&mut Bank)->std::result::Result<R,String>)->std::result::Result<R,String>{
        let mut bank=self.cell.try_borrow_mut().map_err(|e|format!("actual command Bank mutation: {e}"))?;
        call(&mut bank)
    }
}
pub(crate) trait Banks{
    fn borrow_bank(&self)->Result<Ref<'_,Bank>>;
    fn borrow_bank_mut(&self)->Result<RefMut<'_,Bank>>;
}
impl Banks for DefinitionBankLoan<'_>{
    fn borrow_bank(&self)->Result<Ref<'_,Bank>>{
        self.cell.try_borrow().map(|v|Ref::map(v,|bank|&**bank)).map_err(|e|Error::Required(format!("actual command Bank read: {e}")))
    }
    fn borrow_bank_mut(&self)->Result<RefMut<'_,Bank>>{
        self.cell.try_borrow_mut().map(|v|RefMut::map(v,|bank|&mut **bank)).map_err(|e|Error::Required(format!("actual command Bank write: {e}")))
    }
}
impl Banks for RefCell<Bank>{
    fn borrow_bank(&self)->Result<Ref<'_,Bank>>{self.try_borrow().map_err(|e|Error::Required(e.to_string()))}
    fn borrow_bank_mut(&self)->Result<RefMut<'_,Bank>>{self.try_borrow_mut().map_err(|e|Error::Required(e.to_string()))}
}
