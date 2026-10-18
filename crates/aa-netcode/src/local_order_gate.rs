//! Safe aliases to the SAME Session24/Node2E5 bytes used by74D710.
//! Arc/Weak is lifetime metadata; each native byte has one atomic storage cell.
use super::*;
use std::sync::atomic::AtomicU8;
pub(crate) struct Byte {
    word: Arc<AtomicU8>,
}
impl Byte {
    ///Native76FAE7 and75D1BA both store byte0 after the real constructors.
    pub(crate) fn from_native_constructor_zero() -> Self {
        Self {
            word: Arc::new(AtomicU8::new(0)),
        }
    }
    pub(crate) fn read(&self) -> u8 {
        self.word.load(Ordering::SeqCst)
    }
}
///No raw/default constructor. Both aliases refer to the exact currently lent
///Session/Node owner, and expire when either native allocation is destroyed.
pub struct Borrow {
    session: Weak<AtomicU8>,
    admin: Weak<AtomicU8>,
    owner: Weak<()>,
}
///Write-only capability to the SAME Node2E5 byte for the native missing-film
///abort callback. It cannot replace the owning Node or keep it alive.
pub struct AdminWriter {
    word: Weak<AtomicU8>,
    owner: Weak<()>,
}
impl AdminWriter {
    pub(crate) fn store(&self, value: u8) -> aa_orders::Result<()> {
        if self.owner.strong_count() == 0 {
            return Err(aa_orders::Error::SchemaContext(
                "actual abort Node has retired",
            ));
        }
        self.word
            .upgrade()
            .ok_or(aa_orders::Error::SchemaContext(
                "actual Node2E5 has retired",
            ))?
            .store(value, Ordering::SeqCst);
        Ok(())
    }
}
impl Borrow {
    pub fn admin_writer(&self) -> AdminWriter {
        AdminWriter {
            word: self.admin.clone(),
            owner: self.owner.clone(),
        }
    }
    pub(super) fn from_current(session: &Byte, admin: &Byte, owner: &Arc<()>) -> Self {
        Self {
            session: Arc::downgrade(&session.word),
            admin: Arc::downgrade(&admin.word),
            owner: Arc::downgrade(owner),
        }
    }
    pub fn require_current(
        &self,
        players: &PlayersBorrow<'_>,
        local: &LocalClientBorrow<'_>,
    ) -> Result<()> {
        let owner = self
            .owner
            .upgrade()
            .ok_or_else(|| missing("order gate Session owner has retired"))?;
        if !Arc::ptr_eq(&owner, players.identity) || !Arc::ptr_eq(&owner, local.owner_identity) {
            return Err(missing("order gate belongs to another current Host"));
        }
        Ok(())
    }
    fn load(&self, word: &Weak<AtomicU8>) -> aa_orders::Result<u8> {
        if self.owner.strong_count() == 0 {
            return Err(aa_orders::Error::SchemaContext(
                "actual order gate Session has retired",
            ));
        }
        Ok(word
            .upgrade()
            .ok_or(aa_orders::Error::SchemaContext(
                "actual order gate field owner has retired",
            ))?
            .load(Ordering::SeqCst))
    }
    pub(crate) fn store_admin_byte(&mut self, value: u8) -> aa_orders::Result<()> {
        self.load(&self.admin)?;
        self.admin
            .upgrade()
            .ok_or(aa_orders::Error::SchemaContext(
                "actual Node2E5 has retired",
            ))?
            .store(value, Ordering::SeqCst);
        Ok(())
    }
}
impl aa_orders::GateSource for Borrow {
    fn session_byte(&mut self) -> aa_orders::Result<u8> {
        self.load(&self.session)
    }
    fn admin_byte(&mut self) -> aa_orders::Result<u8> {
        self.load(&self.admin)
    }
    fn store_session_byte(&mut self, value: u8) -> aa_orders::Result<()> {
        self.load(&self.session)?;
        self.session
            .upgrade()
            .ok_or(aa_orders::Error::SchemaContext(
                "actual Session24 has retired",
            ))?
            .store(value, Ordering::SeqCst);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use aa_orders::GateSource;
    fn owner() -> LocalBootstrap {
        let mut owner = LocalBootstrap::from_native_constructors();
        owner.host_node = Some(crate::local_host::Node::before_session_reset().unwrap());
        owner
    }
    #[test]
    fn same_owner_raw_bytes_are_lazy_aliases_and_retire_with_the_real_node() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/development10/live-frame-host/native-order-gate.json"
        ))
        .unwrap();
        for c in fixture["constructors"].as_array().unwrap() {
            assert_eq!(c["session"], 0);
            assert_eq!(c["admin"], 0);
            assert_eq!(c["ebx_input"], 0);
        }
        let mut a = owner();
        let mut b = owner();
        let mut gate = {
            let loans = a.runtime_borrow().unwrap();
            loans
                .order_gate
                .require_current(&loans.players, &loans.local_client)
                .unwrap();
            loans.order_gate
        };
        assert_eq!(gate.session_byte().unwrap(), 0);
        assert_eq!(gate.admin_byte().unwrap(), 0);
        let writer = gate.admin_writer();
        writer.store(128).unwrap();
        gate.store_session_byte(255).unwrap();
        assert_eq!(a.session.film_24.read(), 255);
        assert_eq!(a.host_node().unwrap().order_abort_2e5.read(), 128);
        {
            let loans = b.runtime_borrow().unwrap();
            assert!(
                gate.require_current(&loans.players, &loans.local_client)
                    .is_err()
            );
        }
        assert_eq!(b.session.film_24.read(), 0);
        assert_eq!(b.host_node().unwrap().order_abort_2e5.read(), 0);
        drop(a.host_node.take());
        assert_eq!(gate.session_byte().unwrap(), 255);
        assert!(gate.admin_byte().is_err());
        assert!(writer.store(1).is_err());
        drop(a);
        assert!(gate.session_byte().is_err());
    }
}
