Saved Team per-record Init, 2026-10-16

Scope and lifecycle
Team vtableA29780+E8 is4E68C0. Saved4FAE80 passes NULL immediately after
Team Load and before PostCreate/4F9510/next record. The global+DC PostLoad
pass is a different phase and cannot replace this operation. See the separate
reviewer's ../skirmish-20261016-saved-flows/lifecycle-proof/README.txt and
static-proof.json for the complete caller order and real vtable pins.
This addition ports Team4E68C0 and its base4EFA40 NULL-parameter coordinator.
It does not mark generic restoration complete or insert guessed successful
world callbacks. Nonnull creation blocks remain outside this API.

Exact native order
4EFA40 checks current Thing flag16. If set, read world+30+8 tile size,
execute actual Team virtual+C=4E3E90, then46EE00 on Thing+20 with that size.
The instructions matter:4E3E90 returns constant grid contextB3A8C4 and RET
consumes no arguments. It does NOT return a position pointer.46EE00 ignores
ECX and uses the still-stacked Thing+20 and tile-size arguments.
For NULL creation params, the repeated position/facing write-back is skipped.
4EF6A0(NULL) then determines placement success. Only its low AL matters.
False returns immediately, retaining any completed snap changes.
On success call4EE4C0. Then reread flags: bit21 chooses height flags123;
otherwise clear bit12 chooses122; otherwise no height query.4F0E50 receives
the current Thing+20 position and returns extendedST0, which is stored to
Thing+28 as binary32. Required service errors retain earlier effects.
Only after successful base Init does4E68C0 call4BEDF0 to copy Thing+18 name,
404F60 to log, and release the temporary CString reference. It returns AL1.
Team display/localization fields+3C/+40 are not this diagnostic name.
No Team-specific payload field, RNG, rule or source list is modified here.

Public API
  aa_mapworld::team_init::init_saved(&mut impl World, ActorId, Precision)
  aa_mapworld::team_init::base_init_null(&mut impl World, ActorId, Precision)
Both return Result<bool>. World has no default methods. It requires actual
flags/position/name, tile size if reached, snap46EE00, placement4EF6A0(NULL),
index membership4EE4C0, terrain4F0E50 with its extended result, Z storage,
and diagnostic publication. The name is an owned temporary value snapshot;
the actor/Team payload is never cloned or replaced. Snap/index services can
mutate their actual owner before failing, and those writes survive.
The base helper is scoped to the Team vtable path, not an arbitrary GObject
or GActor fresh-creation adapter. Existing private creation helpers assumed
nonnull params and different virtuals, so they were not reused incorrectly.

Required unfinished services
Snap arithmetic,4EF6A0's complete placement checks,4EE4C0's spatial services,
terrain queries, actual diagnostic delivery, and outer PostCreate/4F9510 are
not implemented by this module. These callbacks must bind real owners and
return errors when unavailable; returning true because a Team exists is wrong.
No runtime adapter or generic restore completion marker was changed.
The later lifecycle driver and actual world services have separate owners.
Finite extended terrain values use aa_numeric binary32 storage; nonfinite
values/overflow remain explicit numeric errors, not native exception claims.

Evidence
native_team_init.py independently maps originalAA.exe and v14, executing
4E68C0,4EFA40, Team4E3E90,4BEDF0 and native temporary-reference decrement.
Required snap/placement/index/height/log bodies are explicit recorded seams;
4012C0 provides CString reference transport. No gameplay success is inferred
from an absent callback. Injected failures are ordering probes inside those
services, not claims about native SEH/unwind behavior.
240 cases (40 x PC24/53/64 x2images) cover flag gates, low-AL false/nonzero,
snap changes before false/error, missing dependencies, post-index flag
changes, terrain binary32 rounding/signed zero/underflow, and logging order.
All113 observed native instruction sites match both executable images.
Default replay compares the existing fixture; --write is explicit export.

Rust tests:2 passed with --include-ignored
1. All240 cases match result, callback order, native height stores and final
   owned Thing flags/position/name, including retained failure prefixes.
2. Actual source test loads v14 definitions and original TEAMS chunks from
   DeveloperMaps/(3v3)Firefight.RMP and Campaign/tutorial02.RMP. Firefight
   contains2 real Team records, team_0/team_1, flagsA0042800; tutorial02 has
   no present Team records. Actual saved Thing fields and source-derived
   flags reach the required4EF6A0 placement boundary. With no real spatial
   service supplied, Init errors and retains those fields. This is source
   and boundary validation, NOT a successful actual-world Team Init test.

Commands
  CARGO_TARGET_DIR=target/s-borrowed-flows AA_GAME_ROOT=<game root> cargo test -p aa-mapworld --test native_team_init --release --locked --offline -j4 -- --include-ignored --nocapture
  PYTHONPATH=<game root>/reconstruction/tools/python-packages python research/skirmish-20261016-team-init/native_team_init.py
See tests.log, native-run.log, native-replay.log, native.asm.txt and findings.json.

Preservation
Backup: backups/source-before-team-init-20261016.zip.
Only new team_init.rs, an appended export, a focused test and this evidence
folder were added. No generic restore, shared runtime, source parser,
manifest/dependency, original asset/save/profile or Development08 was edited.
