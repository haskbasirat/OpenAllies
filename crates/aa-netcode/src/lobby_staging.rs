//! Native staging integration for caller-owned complete settings. The snapshot
//! reader `0x77AAC0` calls `0x77A750`; staging in turn uses the real `0x779F00`
//! settings assignment through [`Staging`]. Required UI/world/transport effects
//! remain explicit, with no placeholder successes.
//!
//! [`SnapshotOwner`] binds whole-snapshot decoding to its non-wire companions.
//! It reconstructs decoder-created kingdom fields from the proven native
//! constructor, including equal-count replacements, and invalidates settings
//! after a failed decode instead of exposing stale companion ownership.
//! Direct users of [`Snapshot`] still own that synchronization themselves.
use crate::launch_assignment::SettingsStore;
use crate::lobby::{Session, SnapshotHost};
use crate::staging::{self, LaunchStep, SettingsSlot, StagingHost, StagingNode, StateEvent};
use crate::{Error, Result};
use aa_orders::payload::Field;

/// Owns the only mutable Session/settings pair on this snapshot path. After a
/// failed read, native-compatible partial Session writes remain inspectable,
/// but the settings cannot be copied or reused as though decoding completed.
/// There is no implicit rollback, retry, zero fill or guessed reconstruction.
pub struct SnapshotOwner {
    session: Session,
    settings: Option<SettingsStore>,
}
impl SnapshotOwner {
    /// Inputs must be a complete native-constructed pair, including exact
    /// kingdom correspondence. The check verifies extents, not historical
    /// identity; callers must not build this from already-stale companions.
    pub fn new(session: Session, settings: SettingsStore) -> Result<Self> {
        settings.launch.validate(&session.launch)?;
        settings.launch_88.validate(&session.launch_88)?;
        settings.single.retained.validate(&settings.single.wire)?;
        settings.multi.retained.validate(&settings.multi.wire)?;
        Ok(Self {
            session,
            settings: Some(settings),
        })
    }
    pub fn session(&self) -> &Session {
        &self.session
    }
    pub fn settings(&self) -> Result<&SettingsStore> {
        self.settings.as_ref().ok_or(Error::Unsupported(
            "session settings invalidated by a failed snapshot/staging operation",
        ))
    }
    fn take_settings(&mut self) -> Result<SettingsStore> {
        self.settings.take().ok_or(Error::Unsupported(
            "session settings invalidated by a failed snapshot/staging operation",
        ))
    }
    /// Native state transition through this owner's complete settings. Failed
    /// external effects retain the committed Session prefix and invalidate the
    /// settings owner, preventing an automatic retry of that partial operation.
    pub fn change_to_state(
        &mut self,
        state: u32,
        flag: bool,
        node: &StagingNode,
        effects: &mut impl Effects,
    ) -> Result<()> {
        let mut settings = self.take_settings()?;
        staging::change_to_state(
            &mut self.session,
            state,
            flag,
            node,
            &mut Staging {
                settings: &mut settings,
                effects,
            },
        )?;
        self.settings = Some(settings);
        Ok(())
    }
    /// Full `0x77AAC0` snapshot read with native ChangeToState/assignment and
    /// `0x50F4C0` kingdom companions. Session/Admin/cursor partial effects on
    /// error are not rolled back; the settings owner is then unavailable.
    pub fn read_snapshot(
        &mut self,
        reader: &mut aa_orders::Cursor<'_>,
        admin: &mut crate::lobby::ClientAdmin,
        data: &mut dyn crate::lobby::LobbyData,
        node: &StagingNode,
        effects: &mut impl Effects,
        map_status: &mut impl MapStatus,
    ) -> Result<Vec<crate::lobby::LobbyAction>> {
        let mut settings = self.take_settings()?;
        let actions = crate::lobby::read_session_snapshot(
            reader,
            &mut self.session,
            admin,
            data,
            &mut Snapshot {
                node,
                staging: Staging {
                    settings: &mut settings,
                    effects,
                },
                map_status,
            },
            &node.view,
        )?;
        settings.launch.after_snapshot_read(&self.session.launch)?;
        settings
            .launch_88
            .after_snapshot_read(&self.session.launch_88)?;
        self.settings = Some(settings);
        Ok(actions)
    }
    /// Recover a complete pair only after a successful operation. Invalidated
    /// owners must be discarded/reloaded by the session layer, not rebound.
    pub fn into_complete(self) -> Result<(Session, SettingsStore)> {
        let settings = self.settings.ok_or(Error::Unsupported(
            "cannot recover invalidated session settings",
        ))?;
        Ok((self.session, settings))
    }
}

/// External effects after removing the settings-copy seam from StagingHost.
/// Implementations must propagate missing world loading or transport as errors.
pub trait Effects {
    fn ui_message(&mut self, kind: u32, key: &'static str, number: Option<i32>) -> Result<()>;
    fn request(&mut self, descriptor: usize, fields: Vec<Field>) -> Result<()>;
    fn countdown_time(&mut self) -> Result<i32>;
    fn start_timer(&mut self, session: &mut Session) -> Result<()>;
    fn not_ready(&mut self, session: &mut Session) -> Result<()>;
    fn launch_step(&mut self, session: &mut Session, step: LaunchStep) -> Result<()>;
    fn load(&mut self, session: &mut Session) -> Result<bool>;
    fn dump_launch_settings(&mut self, session: &mut Session, label: &'static str) -> Result<()>;
    fn timeline_freeze(&mut self, flag: bool) -> Result<()>;
    fn reset_order_version(&mut self) -> Result<()>;
    fn state_changed(&mut self, session: &mut Session, event: StateEvent) -> Result<()>;
}

/// Native staging operates directly on Session and the actual saved slots.
pub struct Staging<'a, E: Effects + ?Sized> {
    pub settings: &'a mut SettingsStore,
    pub effects: &'a mut E,
}
impl<E: Effects + ?Sized> StagingHost for Staging<'_, E> {
    fn ui_message(&mut self, kind: u32, key: &'static str, number: Option<i32>) -> Result<()> {
        self.effects.ui_message(kind, key, number)
    }
    fn request(&mut self, descriptor: usize, fields: Vec<Field>) -> Result<()> {
        self.effects.request(descriptor, fields)
    }
    fn countdown_time(&mut self) -> Result<i32> {
        self.effects.countdown_time()
    }
    fn start_timer(&mut self, session: &mut Session) -> Result<()> {
        self.effects.start_timer(session)
    }
    fn not_ready(&mut self, session: &mut Session) -> Result<()> {
        self.effects.not_ready(session)
    }
    fn launch_step(&mut self, session: &mut Session, step: LaunchStep) -> Result<()> {
        self.effects.launch_step(session, step)
    }
    fn load(&mut self, session: &mut Session) -> Result<bool> {
        self.effects.load(session)
    }
    fn dump_launch_settings(&mut self, session: &mut Session, label: &'static str) -> Result<()> {
        self.effects.dump_launch_settings(session, label)
    }
    fn timeline_freeze(&mut self, flag: bool) -> Result<()> {
        self.effects.timeline_freeze(flag)
    }
    fn reset_order_version(&mut self) -> Result<()> {
        self.effects.reset_order_version()
    }
    fn state_changed(&mut self, session: &mut Session, event: StateEvent) -> Result<()> {
        self.effects.state_changed(session, event)
    }
    fn copy_launch_settings(
        &mut self,
        session: &mut Session,
        dst: SettingsSlot,
        src: SettingsSlot,
    ) -> Result<()> {
        self.settings.copy(session, dst, src)
    }
}

/// Actual map-status/checksum/request I/O (0x770B70 / 0x7770B0). Deliberately
/// required: this bridge cannot report that a peer has a map without checking.
pub trait MapStatus {
    fn update_map_status(
        &mut self,
        session: &mut Session,
        rmc: bool,
        file: &[u16],
        crc: u32,
        source_check: bool,
    ) -> Result<bool>;
}

/// Low-level state-transition seam. Whole-snapshot callers should use
/// [`SnapshotOwner::read_snapshot`], which also rebuilds decoded companions.
/// Calling the raw reader with this seam alone does not synchronize companions;
/// equal record counts alone do not prove matching record identities.
pub struct Snapshot<'a, E: Effects + ?Sized, M: MapStatus + ?Sized> {
    pub node: &'a StagingNode,
    pub staging: Staging<'a, E>,
    pub map_status: &'a mut M,
}
impl<E: Effects + ?Sized, M: MapStatus + ?Sized> SnapshotHost for Snapshot<'_, E, M> {
    fn change_to_state(&mut self, session: &mut Session, state: u32, arg: u32) -> Result<()> {
        staging::change_to_state(session, state, arg as u8 != 0, self.node, &mut self.staging)
    }
    fn update_map_status(
        &mut self,
        session: &mut Session,
        rmc: bool,
        file: &[u16],
        crc: u32,
        source_check: bool,
    ) -> Result<bool> {
        self.map_status
            .update_map_status(session, rmc, file, crc, source_check)
    }
}
