# Local Session/player startup, checkpoint 1

This is a working native-backed startup prefix, not a completed playable Host.
The application keeps one real local Session/player owner, the same synchronizer
A, a stable owned VWorld selection list, and actual fallible Windows tick/GUID
and QPC inputs. No original files or profiles were changed or launched here.

## Implemented and checked

- `aa-netcode/src/local_bootstrap.rs`: source-proven reached constructor fields,
  fresh Session Reset76EC70 including no-world EndSession, real mutex/queue/
  Condvar, native RNG reseed, exact11-argument GUID formatting, properties copy,
  and first local player Host75B126..75B1B1 while Host is still null. Session+E8
  and +169, Sim time words, and VWorld+0C are not filled with invented zeroes.
- The player+50 VWorld is allocated even for a null Kingdom. Its private
  current-group slot and wildcard group follow4ED6D0. Its selection list is the
  same non-Clone stable-list owner consumed by the native selection adapter.
- Native765E10 low-level Kingdom replacement writes+28, destroys the old VWorld,
  then creates the replacement. Errors preserve that committed order. Nonnull
  production candidates must be live ActorStore Kingdom objects. This is not a
  claim that the caller773E30 PrepareSessionPlayersForBeginPlaying has run.
- `SelectionBorrow` implements runtime `player_selection::View` directly.
  Ordering selection resolves actual Admin+284 against this Session and does
  not substitute Node+8/+2C. Split Session reads borrow actual E4/F8/F4/name
  fields alongside the mutable actual VWorld; no copied SessionFacts.
- `aa-client/src/windows_reset_os.rs`: documented Windows GetTickCount,
  CoCreateGuid with checked HRESULT, QueryPerformanceFrequency and retained QPC
  anchor. Native59ABA0 integer wrap/truncation is retained, not replaced with
  Reset's unrelated tick source. Only successful documented outputs are read.

`native-bootstrap.json` has24 rows: original/v14, PC24/53/64, four inputs.
Each also executes three native765E10 replacements (72 total). Executed byte
windows agree across both PEs. Constructor owners are explicit reached-field
projections; OS locks/events/tick/GUID, diagnostics, allocation, CString and text
lookup are declared successful seams. The oracle caption is injected `All
Units`; the real accepted text source resolves `Group`. Disabled synchronizer
fixture storage has capacity2 for execution cost; production uses the real
0x10000 constructor argument and the disabled path reads no capacity.

`native-timer.json` has336 original/v14/all-PC rows running actual59ABA0 and
native integer CRT helpers with injected QPC samples. It covers signed deltas,
64-bit wrap and large inputs. The Windows smoke test separately calls real QPC.

Four unit tests pass in `rust-tests-final-view.log`. Four external tests pass
in `windows-source-tests-final-view.log`: real Windows Reset inputs, real QPC,
336-row timer replay, and real accepted original/v14 localization plus public
Session Reset/first-player/null765E10 API. The latter's explicitly supplied test
nickname/profile key are replacement UI inputs, not original account metadata.
Its scope stops before Host/launch; `windows-source-observation-final-view.json`
records the actual OS-derived result. The null765E10 call is an isolated writer
acceptance test and does not fabricate an observer match or finish BeginPlaying.

Both Rust commands used `--release --locked --offline -j4` and temporary command
overrides `profile.release.lto=false`, `profile.release.codegen-units=16` in
exclusive `target/s-local-bootstrap`. This is not a default thin-LTO build.
All checkpoint commands have been reaped. Parent owns final combined validation.

## Remaining native producers

Session Reset is not the entire751770 Node Reset. The next checkpoint must retain
one actual OrderAdmin request+0C/decree+214 owner through fresh Node Reset,
append-self, SetServer759790 and SetHost757010. It must not instantiate a second
PeerBase request history, invent eager heartbeat Config inputs, or infer
Admin+284 ordering authority from the local player. ServerClock arithmetic is
being implemented separately over the same retained QPC owner.

Selected settings/mode and77A950 staging/launch, actual773E30 Kingdom-name/AI
assignment, Admin order-player producer, DevMode40EAA0 input, positive static
groups, transport and a full battle are still required. Constructor-null+28 is
not accepted by late consumers until the real low-level replacement completed.
These files do not claim internet multiplayer or full application startup.

## Preserved attempts and documentation

Earlier logs remain historical: diagnostic timer seams, fixture instruction
budgets, two exactly owned expensive Python attempts, and an initially wrong
queue count offset were diagnosed before the final native fixture. Native ring
count is queue+38, not+34 (head); final rows compare actual queued values.
Initial Rust attempts also captured compile/API races and that measurement
failure. No failed attempt is presented as the final proof. `backups/` retains
the earlier sources/fixtures.

Windows ABI references checked2026-10-16:
[GetTickCount](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount),
[CoCreateGuid](https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-cocreateguid),
[QueryPerformanceCounter](https://learn.microsoft.com/en-us/windows/win32/api/profileapi/nf-profileapi-queryperformancecounter),
[QueryPerformanceFrequency](https://learn.microsoft.com/en-us/windows/win32/api/profileapi/nf-profileapi-queryperformancefrequency).

`checkpoint01.json` pins this checkpoint's exact owned sources and receipts.
Future Host work must use a new checkpoint rather than silently altering these
results or treating later source changes as having been tested by these logs.
