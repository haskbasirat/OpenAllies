//! Owned read-only memory-backed map resource streams.
//!
//! Both views share the SAME MapArchive allocation. Normal return releases the
//! input view first and its embedded chunk-stream view second. This certifies
//! this backend's read/close lifetime, not native Win32 file locks/destructors.
use aa_mapworld::{MapArchive,SourceIdentity};
use std::sync::{Arc,Weak};

/// Immutable capability, not another stream owner. Weak retains the allocation
/// identity after close, so allocator address reuse cannot forge a match.
#[derive(Clone)]
pub struct ArchiveWitness {owner:Weak<MapArchive>,source:SourceIdentity}
impl ArchiveWitness {
    pub(crate) fn validate_map(&self,map:&MapArchive)->Result<(),String> {
        let actual=self.owner.upgrade().ok_or("map resource archive was already released")?;
        if !std::ptr::eq(actual.as_ref(),map) || map.source()!=&self.source {
            return Err("map resource archive is not its actual retained allocation/source".into());
        }
        Ok(())
    }
    pub(crate) fn same_owner(&self,other:&Self)->bool {
        self.owner.ptr_eq(&other.owner) && self.source==other.source
    }
    pub fn source(&self)->&SourceIdentity {&self.source}
}

#[derive(PartialEq,Eq)]
enum Phase {Open,InputClosed,Closed}

/// Move-only: no map bytes or mutable cursor are cloned. Arc cloning creates
/// the two required views of one immutable backend allocation.
pub struct Owner {
    input:Option<Arc<MapArchive>>,
    embedded:Option<Arc<MapArchive>>,
    witness:ArchiveWitness,
    phase:Phase,
}
impl Owner {
    pub fn open(map:MapArchive)->Self {
        let archive=Arc::new(map);
        let witness=ArchiveWitness{owner:Arc::downgrade(&archive),source:archive.source().clone()};
        Self{input:Some(Arc::clone(&archive)),embedded:Some(archive),witness,phase:Phase::Open}
    }
    pub fn archive(&self)->Result<&MapArchive,String> {
        if self.phase!=Phase::Open {return Err("map resource streams no longer admit reads".into())}
        self.embedded.as_deref().ok_or_else(||"map resource embedded view is absent".into())
    }
    pub fn witness(&self)->ArchiveWitness {self.witness.clone()}
    ///A second real OpenAs over the same retained immutable resource depot.
    ///The views have their own close lifetime; no map bytes/cursors are copied.
    pub fn reopen_shared(&self)->Result<Self,String> {
        self.witness.validate_map(self.archive()?)?;
        let archive=Arc::clone(self.embedded.as_ref().ok_or("actual embedded map archive is absent")?);
        Ok(Self {input:Some(Arc::clone(&archive)),embedded:Some(archive),witness:self.witness.clone(),phase:Phase::Open})
    }
    pub(crate) fn validate_open_for(&self,expected:&ArchiveWitness)->Result<(),String> {
        if !expected.same_owner(&self.witness) {return Err("map close owner differs from actual resource archive".into())}
        expected.validate_map(self.archive()?)
    }
    pub(crate) fn close_input(&mut self)->Result<(),String> {
        if self.phase!=Phase::Open {return Err("input map stream close is not at its native boundary".into())}
        let actual=self.input.take().ok_or("actual input map view is absent")?;
        drop(actual);self.phase=Phase::InputClosed;Ok(())
    }
    pub(crate) fn close_embedded(&mut self)->Result<(),String> {
        if self.phase!=Phase::InputClosed {return Err("embedded map stream close precedes input close or repeats".into())}
        let actual=self.embedded.take().ok_or("actual embedded map view is absent")?;
        drop(actual);self.phase=Phase::Closed;Ok(())
    }
    pub fn is_closed(&self)->bool {self.phase==Phase::Closed}
}

#[cfg(test)]
mod tests {
    use super::*;
    fn empty_archive()->MapArchive {
        let mut data=b"TGCK".to_vec();data.extend(2u32.to_le_bytes());data.extend(0u32.to_le_bytes());
        MapArchive::from_bytes(data).unwrap()
    }
    #[test]
    fn same_archive_views_close_once_and_foreign_equal_bytes_are_not_same_owner() {
        let mut actual=Owner::open(empty_archive());let witness=actual.witness();
        let foreign=Owner::open(empty_archive());
        assert_eq!(actual.archive().unwrap().source(),foreign.archive().unwrap().source());
        assert!(actual.validate_open_for(&foreign.witness()).is_err());
        actual.validate_open_for(&witness).unwrap();
        assert!(actual.close_embedded().is_err());
        actual.close_input().unwrap();assert!(actual.archive().is_err());
        assert!(actual.close_input().is_err());actual.close_embedded().unwrap();
        assert!(actual.is_closed());assert!(witness.owner.upgrade().is_none());
        assert!(actual.close_embedded().is_err());
        let later=Owner::open(empty_archive());assert!(!witness.same_owner(&later.witness()));
    }
    #[test]
    fn reopened_depot_views_retain_same_archive_and_independent_close_lifetimes() {
        let actual=Owner::open(empty_archive());let witness=actual.witness();
        let mut reopened=actual.reopen_shared().unwrap();
        reopened.validate_open_for(&witness).unwrap();
        reopened.close_input().unwrap();reopened.close_embedded().unwrap();
        assert!(reopened.reopen_shared().is_err());
        actual.validate_open_for(&witness).unwrap();
        assert!(witness.owner.upgrade().is_some());
        drop(actual);assert!(witness.owner.upgrade().is_none());
    }
}
