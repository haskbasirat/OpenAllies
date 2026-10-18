//! Concrete local player/client/security binding for native Admin order flow.
//! Admin streams/280/284, player roster, Node2C/6C/2C4/268 and both synchronizers
//! are the same retained owners. Gameplay/film/OS leaves remain required.
use crate::lobby::PeerLink;
use crate::local_bootstrap::{LocalClientBorrow, PlayerHandle, PlayersBorrow};
use aa_orders::{
    Error, Result,
    commands::{self, PlayerOutcome, TryPlayerContext},
    flow::{Log, SyncEntry, Validity},
    net::{
        admin::{ClientIdentity, CurrentClient, OrderingHost},
        sync::Synchronizer,
    },
    payload::Field,
};
use aa_runtime::load_world_globals::SessionPhase;
use std::cell::RefCell;
fn required<T>(name: &'static str) -> Result<T> {
    Err(Error::SchemaContext(name))
}
fn source<T>(r: crate::Result<T>) -> Result<T> {
    source_at(298, r)
}
fn source_at<T>(index: usize, r: crate::Result<T>) -> Result<T> {
    r.map_err(|e| Error::Handler {
        index,
        position: aa_orders::Position::PACKET_START,
        message: e.to_string(),
    })
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Diagnostic {
    NullOrderingPlayer,
    SecurityBreach,
    MissingFilmPlayer(u32),
    ///7710E0/76DA60's actual localized rejection, never a fabricated seed.
    SessionOrderRejectedCountingDown,
}
struct SeedEffects<'a, E>(&'a mut E);
impl<E: Effects> crate::local_bootstrap::launch::Effects for SeedEffects<'_, E> {
    fn mode_rejected_counting_down(&mut self) -> crate::Result<()> {
        self.0
            .diagnostic(Diagnostic::SessionOrderRejectedCountingDown)
            .map_err(crate::Error::Orders)
    }
    fn set_new_state_diagnostic(&mut self, _: u32) -> crate::Result<()> {
        Err(crate::Error::Unsupported(
            "7710E0 never sets a new Session state",
        ))
    }
}
/// These are reached external effects, never recording-only successful defaults.
/// sync_log_gate must read actual simulation-thread/application flags and is
/// called only after the actual enabled/suspend guards; it cannot mutate logs.
pub trait Effects {
    fn sync_log_gate(&mut self, log: Log) -> Result<bool>;
    fn diagnostic(&mut self, value: Diagnostic) -> Result<()>;
    fn stop_film(&mut self, players: &mut PlayersBorrow<'_>) -> Result<()>;
    fn set_admin_abort(&mut self) -> Result<()>;
    fn valid(
        &mut self,
        players: &mut PlayersBorrow<'_>,
        slot: &mut u32,
        client: &CurrentClient,
        validity: Validity,
        request: usize,
        fields: &[Field],
    ) -> Result<bool>;
    fn process(
        &mut self,
        players: &mut PlayersBorrow<'_>,
        local: &mut LocalClientBorrow<'_>,
        slot: &mut u32,
        client: &CurrentClient,
        decree: usize,
        fields: &mut Vec<Field>,
    ) -> Result<()>;
    fn process_with_emission<'owner>(
        &mut self,
        players: &mut PlayersBorrow<'owner>,
        local: &mut LocalClientBorrow<'owner>,
        slot: &mut u32,
        client: &CurrentClient,
        decree: usize,
        fields: &mut Vec<Field>,
        _emission: &mut aa_orders::net::admin::RequestEmission<'_>,
        _sync_a: &RefCell<Synchronizer>,
        _sync_s: &mut Synchronizer,
    ) -> Result<()> {
        self.process(players, local, slot, client, decree, fields)
    }
}
pub struct World<'a, 'owner, E> {
    pub players: PlayersBorrow<'owner>,
    pub local_client: LocalClientBorrow<'owner>,
    pub sync_a: &'owner RefCell<Synchronizer>,
    pub sync_s: &'owner mut Synchronizer,
    pub effects: &'a mut E,
}
impl<E: Effects> World<'_, '_, E> {
    fn current_player(&self, slot: u32) -> Result<Option<PlayerHandle>> {
        if slot == 0 {
            return Ok(None);
        }
        // Production LocalBootstrap currently creates non-film players only. Its
        // source pointer is represented by this same-owner stable player handle.
        // A positive film allocation needs a typed284 handle before supporting it.
        if self.players.find_order_player(slot, true).is_some() {
            return required("actual typed film-player284 allocation owner");
        }
        self.players
            .find_order_player(slot, false)
            .map(Some)
            .ok_or(Error::SchemaContext(
                "current Admin284 player allocation is not owned by this Session",
            ))
    }
    fn player_client(&self, slot: u32) -> Result<Option<PeerLink>> {
        self.current_player(slot)?
            .map(|p| source(self.players.player_link(&p)))
            .transpose()
    }
    pub fn set_player(
        &mut self,
        slot: &mut u32,
        client: &CurrentClient,
        id: u32,
    ) -> Result<PlayerOutcome> {
        let result = commands::try_process_set_player(
            id,
            &mut PlayerContext {
                world: self,
                slot,
                client,
                requested: id,
            },
        )?;
        if result == PlayerOutcome::SecurityRejected {
            self.effects.diagnostic(Diagnostic::SecurityBreach)?;
        }
        Ok(result)
    }
}
struct PlayerContext<'a, 'world, 'owner, E> {
    world: &'a mut World<'world, 'owner, E>,
    slot: &'a mut u32,
    client: &'a CurrentClient,
    requested: u32,
}
impl<E: Effects> TryPlayerContext for PlayerContext<'_, '_, '_, E> {
    type Player = PlayerHandle;
    type Client = PeerLink;
    type Error = Error;
    fn film_mode(&mut self) -> Result<bool> {
        Ok(self.world.players.film_mode_24())
    }
    fn find_player(&mut self, id: u32, film: bool) -> Result<Option<PlayerHandle>> {
        Ok(self.world.players.find_order_player(id, film))
    }
    fn player_is_film(&mut self, p: &PlayerHandle) -> Result<bool> {
        source(self.world.players.player_film(p))
    }
    fn is_authorizing_admin(&mut self) -> Result<bool> {
        Ok(self.world.local_client.is_host_self())
    }
    fn player_client(&mut self, p: &PlayerHandle) -> Result<PeerLink> {
        source(self.world.players.player_link(p))
    }
    fn ordering_client(&mut self) -> Result<PeerLink> {
        match self.client {
            CurrentClient::Null => Ok(PeerLink::Null),
            CurrentClient::Bound(ClientIdentity::OwnNode) => Ok(PeerLink::Node),
            CurrentClient::Bound(ClientIdentity::Remote(_)) => {
                required("same real remote-client token/transport owner")
            }
            CurrentClient::UnboundPortable => {
                required("portable fixture has no native current-client280 owner")
            }
        }
    }
    fn bind_client_player(&mut self, client: PeerLink, p: &PlayerHandle) -> Result<()> {
        match client {
            PeerLink::Node => {
                source(self.world.players.player_link(p))?;
                self.world.local_client.bind_player_74b350(p.id());
                Ok(())
            }
            PeerLink::Null => required("native74B350 requires nonnull actual player client"),
            PeerLink::Conn(_) => required("native74B350 actual remote Node2C owner"),
        }
    }
    fn set_current_player(&mut self, p: Option<PlayerHandle>) -> Result<()> {
        if let Some(p) = p {
            if source(self.world.players.player_film(&p))? {
                return required("actual typed film-player284 allocation owner");
            };
            *self.slot = p.id();
        } else {
            *self.slot = 0;
        }
        Ok(())
    }
    fn stop_film(&mut self) -> Result<()> {
        self.world
            .effects
            .diagnostic(Diagnostic::MissingFilmPlayer(self.requested))?;
        self.world.effects.stop_film(&mut self.world.players)
    }
    fn set_abort(&mut self) -> Result<()> {
        self.world.effects.set_admin_abort()
    }
}
impl<E: Effects> OrderingHost for World<'_, '_, E> {
    fn process_with_emission(
        &mut self,
        slot: &mut u32,
        client: &CurrentClient,
        decree: usize,
        fields: &mut Vec<Field>,
        emission: &mut aa_orders::net::admin::RequestEmission<'_>,
    ) -> Result<()> {
        if matches!(decree, 92 | 298) {
            self.process(slot, client, decree, fields)
        } else {
            self.effects.process_with_emission(
                &mut self.players,
                &mut self.local_client,
                slot,
                client,
                decree,
                fields,
                emission,
                self.sync_a,
                self.sync_s,
            )
        }
    }
    fn has_ordering_client(&mut self, slot: &mut u32, _: &CurrentClient) -> Result<bool> {
        if *slot == 0 {
            self.effects.diagnostic(Diagnostic::NullOrderingPlayer)?;
            return Ok(false);
        }
        Ok(self
            .player_client(*slot)?
            .is_some_and(|link| link != PeerLink::Null))
    }
    fn version_acknowledged(&mut self, slot: &mut u32, _: &CurrentClient) -> Result<bool> {
        match self.player_client(*slot)? {
            Some(PeerLink::Node) => Ok(self.local_client.order_version() != 0),
            Some(PeerLink::Conn(_)) => required("actual remote ordering client6C byte"),
            _ => required("version acknowledgment requires actual nonnull ordering client"),
        }
    }
    fn film_session(&mut self, _: &mut u32, _: &CurrentClient) -> Result<bool> {
        self.players
            .application_state()
            .map(|v| v == 4)
            .map_err(|_| Error::SchemaContext("actual SessionE4"))
    }
    fn valid(
        &mut self,
        slot: &mut u32,
        client: &CurrentClient,
        v: Validity,
        r: usize,
        f: &[Field],
    ) -> Result<bool> {
        self.effects.valid(&mut self.players, slot, client, v, r, f)
    }
    fn sync(&mut self, _: &mut u32, _: &CurrentClient, entry: SyncEntry) -> Result<()> {
        let log = match entry {
            SyncEntry::Int { log, .. } | SyncEntry::Text { log, .. } => log,
        };
        let mut a;
        let target = match log {
            Log::A => {
                a = self
                    .sync_a
                    .try_borrow_mut()
                    .map_err(|_| Error::SchemaContext("same native A synchronizer borrow"))?;
                &mut *a
            }
            Log::S => &mut *self.sync_s,
        };
        if !target.enabled || target.suspend >= 1 {
            return Ok(());
        }
        let gate = self.effects.sync_log_gate(log)?;
        match entry {
            SyncEntry::Int { label, value, .. } => target.log_int(gate, label, value),
            SyncEntry::Text { label, value, .. } => {
                let units = value.encode_utf16().collect::<Vec<_>>();
                target.log_text(gate, label, &units);
            }
        }
        Ok(())
    }
    fn process(
        &mut self,
        slot: &mut u32,
        client: &CurrentClient,
        decree: usize,
        fields: &mut Vec<Field>,
    ) -> Result<()> {
        if decree == 92 {
            //75DA40 only forwards the decoded seed to7710E0. Authority is
            //the existing90->92 request flow; no extra local-player/host gate.
            let [Field::Uint { width: 32, value }] = fields.as_slice() else {
                return Err(Error::SchemaField {
                    expected: "native SetMapSeed uint32",
                });
            };
            source_at(
                92,
                self.players
                    .set_seed_7710e0(*value, &mut SeedEffects(self.effects)),
            )
        } else if decree == 298 {
            let [Field::Uint { width: 32, value }] = fields.as_slice() else {
                return Err(Error::SchemaField {
                    expected: "native SetPlayer uint32 id",
                });
            };
            self.set_player(slot, client, *value).map(drop)
        } else {
            self.effects.process(
                &mut self.players,
                &mut self.local_client,
                slot,
                client,
                decree,
                fields,
            )
        }
    }
    fn mirror_enabled(&mut self, _: &mut u32, _: &CurrentClient) -> Result<bool> {
        Ok(self.local_client.mirror_enabled())
    }
}
