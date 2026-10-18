Native VWorld current-UVar reset child, 2026-10-17

This ports43F156..43F1B2, one ordered child of43F0A0. It does not claim complete
VWorld initialization, AD publication, resource-task completion, rendering,
loading-screen17, Session status3, unfreeze or a playable battle.

Actual complete registration41F9D0 has213 indices. The reset looks up all213,
but changes only Bool/type1 variables with(flags&5)==5 and index44. There are
eight such cells:22 ViewShowBlocking,24 ViewShowBounds,25 ViewShowSettlementDebug,
27 ViewShowGuardDebug,29 ViewShowControlDebug,44 CameraInformation,
100 RenderDrawTerrainGrid,101 RenderDrawRegionLines. All eight constructors
produce currentbyte0=0 and resetdefaultbyte1=0. The other205 entries are immutable
metadata only; their current values are NOT copied or owned a second time.

Overlap census: CameraInformation already existed in aa-ui::UVars and is read
by the HUD. The other seven names had no production current-cell owner. The
eight now live in canonical raw-byte cells beside that SAME UVars manager;
their values/defaults string-map entries are absent. Existing get/set/default,
options and private-profile accessors forward to those cells. UI option drafts
remain deliberate value clones, as before; reset receives the actual UI owner.
The new assign_view_bool accepts a real reached editor gate or fails explicitly.
The existing profile format preserves noncanonical raw bytes for these cells.

4B2840 resets flags5 Bool currentbyte tozero. Index44 reads the real Session+F8
through76AC90; if the first read is7 it skips the second. Otherwise it reads
again, and copies resetdefaultbyte1 unless that second result is5. The core
retains exactly that ordering and earlier mutations on a required getter stop.
It does not cache either Session value. Call actual runtime SessionPhase's
current_phase at each closure invocation; do not supply a guessed game type.

Fresh source scope is explicit ControlledNone. Exact root UVars*.tgi wildcard
files are read in native enumeration order, hashes retained. These eight source
declarations require Bool type and proven true/false literal scope. The seven
flags5 source fields are absent orfalse in shipped sources, so they remain at
constructorzero whether the developer write gate permits or rejects the write.
Positive editor source overrides need the actual EDITORDEV gate and currently
return an error. CameraInformation flags16 is independently writable. Unknown
profile/console ownership, source parse errors and typed setter requirements
remain explicit failures for strict reset; no empty-source fallback mints it.
This is a reached-field source projection, not execution of the complete native
TGI/variable loading graph. Client assembly must match source-file identities to
its retained application/source backend. No VWorld-ready receipt is minted.

Native default-immutable evidence:
- native-uvar-registration.json:6 original/v14/all-PC runs,213 registrations
  each,5764 executed instruction pins. Actual typed constructors run through
  the213th registration;4B3470 is captured transport, then execution stops
  before the later snapshot manager. CString/heap/log services are inherited
  explicit seams. No profile/source assignment is claimed in this fixture.
- native-view-uvar-reset.json:84 paired rows/168 fresh machines,52 executed
  pins. Actual reset interval,indexed lookup,reset and Session getter execute.
  Session memory changes at getter entry are external storage intervention to
  prove lazy reads, not an original application callback. Required rows stop
  before the getter body and retain earlier reset stores.
- native-view-bool-assignment.json:288 paired rows/576 fresh machines,122 pins,
  no game-code seams. Actual4B3190/4B29B0/40FCA0/40EAA0 execute with explicit
  E4/F4, null local Node+8 and DevMode UVar inputs. Positive player-name service
  and a concrete actual application binding are outside this fixture.
All three producers freshly replayed unchanged after source tests.

Rust validation:55/55 full aa-ui tests passed before adding one additional
native setter comparison. Final current library tests22/22 pass, so56 distinct
tests pass overall (22 library +34 integration); no production source changed
between the broad integration run and the final library run. The added oracle
test replays288 native setter rows. Existing options/loading/hotkey/layout/
sprite/text tests pass. Actual data test checks all3 root source files and the
same UI/HUD/Camera raw byte, private profile round-trip and both reset paths.
No native game process, Windows GUI capture or GPU acceptance test ran here.
Source/test/fixture/binary/log hashes are in findings.json.

Original game binaries/assets/saves/profiles were not modified. The pre-change
uvars.rs is preserved here. No vendor SDK source was copied or integrated.
