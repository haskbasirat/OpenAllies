//! Scoped access to the sole native Admin+284 owner. Portable OrderHost
//! fixtures remain supported; production callbacks must consume this live slot.
use super::{CurrentClient, RequestEmission};
use crate::{
    Result,
    flow::{OrderHost, SyncEntry, TryOrderHost, Validity},
    payload::Field,
};

/// All callbacks are required. The slot is the actual Admin client.field_284,
/// never a snapshot, and remains borrowed for synchronous nested gameplay.
/// Zero is native null; nonzero values are SessionPlayer identities whose
/// resolution/security rules are the concrete host's required responsibility.
pub trait OrderingHost {
    ///Direct766CF0 ->751FC0, outside the request/decree template. Existing
    ///portable hosts have no actual client6C owner and must refuse it.
    fn acknowledge_order_version(&mut self,_slot:&mut u32,_client:&CurrentClient)->Result<()> {
        Err(crate::Error::UnsupportedNativeDescriptor(crate::native::VERSION_ACK_REQUEST_ID))
    }
    fn has_ordering_client(&mut self, slot: &mut u32, client: &CurrentClient) -> Result<bool>;
    fn version_acknowledged(&mut self, slot: &mut u32, client: &CurrentClient) -> Result<bool>;
    fn film_session(&mut self, slot: &mut u32, client: &CurrentClient) -> Result<bool>;
    fn valid(
        &mut self,
        slot: &mut u32,
        client: &CurrentClient,
        validity: Validity,
        request: usize,
        fields: &[Field],
    ) -> Result<bool>;
    fn sync(&mut self, slot: &mut u32, client: &CurrentClient, entry: SyncEntry) -> Result<()>;
    fn process(
        &mut self,
        slot: &mut u32,
        client: &CurrentClient,
        decree: usize,
        fields: &mut Vec<Field>,
    ) -> Result<()>;
    ///Additive live callback form. Existing hosts keep their exact process
    ///behavior; concrete reentrant emitters override this method explicitly.
    fn process_with_emission(
        &mut self,
        slot: &mut u32,
        client: &CurrentClient,
        decree: usize,
        fields: &mut Vec<Field>,
        _emission: &mut RequestEmission<'_>,
    ) -> Result<()> {
        self.process(slot, client, decree, fields)
    }
    fn mirror_enabled(&mut self, slot: &mut u32, client: &CurrentClient) -> Result<bool>;
}

pub(super) enum Host<'a> {
    Portable(&'a mut dyn OrderHost),
    Live(&'a mut dyn OrderingHost),
}
pub(super) struct ScopedHost<'a, 'b> {
    pub host: &'a mut Host<'b>,
    pub slot: &'a mut u32,
    pub client: &'a CurrentClient,
    pub request_stream: &'a mut crate::commands::WireWriter,
    pub last_request_player: &'a mut Option<u32>,
}
impl ScopedHost<'_, '_> {
    pub(super) fn acknowledge_order_version(&mut self)->Result<()> {
        match &mut self.host {
            Host::Live(h)=>h.acknowledge_order_version(self.slot,self.client),
            Host::Portable(_)=>Err(crate::Error::UnsupportedNativeDescriptor(crate::native::VERSION_ACK_REQUEST_ID)),
        }
    }
}
impl TryOrderHost for ScopedHost<'_, '_> {
    fn has_ordering_client(&mut self) -> Result<bool> {
        match &mut self.host {
            Host::Portable(h) => Ok(h.has_ordering_client()),
            Host::Live(h) => h.has_ordering_client(self.slot, self.client),
        }
    }
    fn version_acknowledged(&mut self) -> Result<bool> {
        match &mut self.host {
            Host::Portable(h) => Ok(h.version_acknowledged()),
            Host::Live(h) => h.version_acknowledged(self.slot, self.client),
        }
    }
    fn film_session(&mut self) -> Result<bool> {
        match &mut self.host {
            Host::Portable(h) => Ok(h.film_session()),
            Host::Live(h) => h.film_session(self.slot, self.client),
        }
    }
    fn valid(&mut self, v: Validity, r: usize, f: &[Field]) -> Result<bool> {
        match &mut self.host {
            Host::Portable(h) => h.valid(v, r, f),
            Host::Live(h) => h.valid(self.slot, self.client, v, r, f),
        }
    }
    fn sync(&mut self, e: SyncEntry) -> Result<()> {
        match &mut self.host {
            Host::Portable(h) => h.sync(e),
            Host::Live(h) => h.sync(self.slot, self.client, e),
        }
    }
    fn process(&mut self, d: usize, f: &mut Vec<Field>) -> Result<()> {
        match &mut self.host {
            Host::Portable(h) => h.process(d, f),
            Host::Live(h) => h.process_with_emission(
                self.slot,
                self.client,
                d,
                f,
                &mut RequestEmission {
                    stream: self.request_stream,
                    last_player: self.last_request_player,
                },
            ),
        }
    }
    fn mirror_enabled(&mut self) -> Result<bool> {
        match &mut self.host {
            Host::Portable(h) => Ok(h.mirror_enabled()),
            Host::Live(h) => h.mirror_enabled(self.slot, self.client),
        }
    }
}
