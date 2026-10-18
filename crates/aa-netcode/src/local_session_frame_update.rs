//! Actual recurring Session update776540 over the retained Session/Node loans.
//!
//! Native order is775DB0(block=false),76E3D0,76C4B0. A NULL task never skips
//! the latter two calls. The admitted active-game path has no countdown and
//! uses the same Node+2E0 constructor/host-migration word, not a lobby snapshot.
//! Positive countdown76C960, completed-task diagnostics/callbacks, and the
//! migration request76B570 remain exact required boundaries.
use super::*;

const COUNTDOWN: &str =
    "76E3D0 positive countdown requires actual76C960 readiness and subsequent timer/request owners";
const MIGRATION: &str =
    "76C4B0 requires actual76B570 migration request after the retained Session16C store";

///776540. These must be sibling loans of the SAME LocalBootstrap. Native
///failure prefixes survive: in particular16C becomes true before76B570, even
///when that unbound request stops the update. No activation result is minted.
pub fn run(players: &mut PlayersBorrow<'_>, local: &mut LocalClientBorrow<'_>) -> Result<()> {
    if !Arc::ptr_eq(players.identity, local.owner_identity) {
        return Err(missing(
            "776540 Session and local Node belong to different owners",
        ));
    }
    //775DCA first reads the real task slot; task absence is not frame absence.
    leave_step::check_task_slot_775db0(&players.session.task_124, false)?;
    after_task(players.session, local)
}

fn after_task(session: &mut SessionFields, local: &LocalClientBorrow<'_>) -> Result<()> {
    //76E3DC host identity is read BEFORE the signed11C countdown test. The
    //first positive native child is76C960, never a fabricated ready=true.
    if local.is_host_self() && session.host_countdown > 0 {
        return Err(Error::Unsupported(COUNTDOWN));
    }
    //76C4B4. Preserve every short-circuit, including2E0 before SessionE4.
    let migration_sent = session.flags_168_16e[4]
        .ok_or_else(|| missing("76C4B0 actual Session16C byte is unbound"))?;
    if migration_sent || !local.is_host_self() {
        return Ok(());
    }
    if *local.takeover_2e0 == 0 || session.state != 1 {
        return Ok(());
    }
    session.flags_168_16e[4] = Some(true); //76C4F1, before the call.
    Err(Error::Unsupported(MIGRATION))
}

#[cfg(test)]
#[path = "local_session_frame_update_tests.rs"]
mod tests;
