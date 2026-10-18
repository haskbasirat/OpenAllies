//!Callback-scoped loans of the actual Session/client/log owners. Native
//!request normalization runs between callbacks and may read these same cells.
//!Every callback delegates the existing World kernel without copied state.
use super::*;

pub struct CellWorld<'a,'owner,E>{
    pub players:&'a RefCell<PlayersBorrow<'owner>>,
    pub local_client:&'a RefCell<LocalClientBorrow<'owner>>,
    pub sync_a:&'owner RefCell<Synchronizer>,
    pub sync_s:&'a RefCell<&'owner mut Synchronizer>,
    pub effects:&'a mut E,
}
impl<'owner,E:Effects> CellWorld<'_,'owner,E>{
    fn with<R>(&mut self,call:impl FnOnce(&mut BorrowedWorld<'_,'owner,'_,E>)->Result<R>)->Result<R>{
        let mut players=self.players.try_borrow_mut().map_err(|_|Error::SchemaContext("same ordering Session callback loan"))?;
        let mut local=self.local_client.try_borrow_mut().map_err(|_|Error::SchemaContext("same ordering client callback loan"))?;
        let mut s=self.sync_s.try_borrow_mut().map_err(|_|Error::SchemaContext("same ordering S callback loan"))?;
        call(&mut BorrowedWorld{players:&mut *players,local_client:&mut *local,sync_a:self.sync_a,sync_s:&mut **s,effects:&mut *self.effects})
    }
}
impl<E:Effects> OrderingHost for CellWorld<'_,'_,E>{
    fn acknowledge_order_version(&mut self,slot:&mut u32,c:&CurrentClient)->Result<()>{self.with(|w|w.acknowledge_order_version(slot,c))}
    fn has_ordering_client(&mut self,slot:&mut u32,c:&CurrentClient)->Result<bool>{self.with(|w|w.has_ordering_client(slot,c))}
    fn version_acknowledged(&mut self,slot:&mut u32,c:&CurrentClient)->Result<bool>{self.with(|w|w.version_acknowledged(slot,c))}
    fn film_session(&mut self,slot:&mut u32,c:&CurrentClient)->Result<bool>{self.with(|w|w.film_session(slot,c))}
    fn valid(&mut self,slot:&mut u32,c:&CurrentClient,v:Validity,r:usize,f:&[Field])->Result<bool>{self.with(|w|w.valid(slot,c,v,r,f))}
    fn sync(&mut self,slot:&mut u32,c:&CurrentClient,e:SyncEntry)->Result<()>{self.with(|w|w.sync(slot,c,e))}
    fn process(&mut self,slot:&mut u32,c:&CurrentClient,d:usize,f:&mut Vec<Field>)->Result<()>{self.with(|w|w.process(slot,c,d,f))}
    fn process_with_emission(&mut self,slot:&mut u32,c:&CurrentClient,d:usize,f:&mut Vec<Field>,e:&mut aa_orders::net::admin::RequestEmission<'_>)->Result<()>{self.with(|w|w.process_with_emission(slot,c,d,f,e))}
    fn mirror_enabled(&mut self,slot:&mut u32,c:&CurrentClient)->Result<bool>{self.with(|w|w.mirror_enabled(slot,c))}
}
