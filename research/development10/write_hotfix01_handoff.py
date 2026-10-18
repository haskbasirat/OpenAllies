"""Write continuation records only after the diagnostic seal has completed."""
import package_hotfix01 as p

manifest=p.read(p.RELEASE/"manifest.json")
assert manifest["release_mode"]=="diagnostic" and manifest["playable_battle"] is False
exe=p.RELEASE/p.EXE
archive=p.RELEASE/manifest["archive"]
assert p.sha(exe)==manifest["executable_sha256"]
result=dict(status="sealed diagnostic loading-responsiveness hotfix",release_mode="diagnostic",
    battle_status="unavailable",playable_battle=False,full_game_parity=False,standalone_game=False,
    executable=p.record(exe),archive=p.record(archive),manifest=p.record(p.RELEASE/"manifest.json"),
    validation=p.record(p.ROOT/"qa-output/development10/hotfix01-acceptance/validation.json"),
    original_game_changed=False,previous_releases_preserved=7251,
    backup=p.record(p.ROOT/"backups/source-checkpoint-20261017-loading-hotfix/pre-hotfix-source-index.json"),
    source_files=2075,changed_existing_source_files=9,new_source_files=5,
    observed_maps={"Berlin":"Responsive pending through155s; then scripted close; exit0 at187.875s with native cleanup wait.",
        "Firefight":"Responsive GUI/menu navigation; genuine unsupported WorldStart classifier error visible by125s; final error at220s, exit0."},
    cli_checks="8/8 passed",mechanics="55/55 bounded fragment checks",world="24/24 bounded fragment checks",
    next_battle_blocker="WorldStart4F1E10 object_active calls saved-only PostLoad classifier, rejecting actual Source::New Actor. Read-only minimal native-proven branch researched; not implemented in hotfix.")
p.write_new(p.ROOT/"qa-output/development10/hotfix01-release-result.json",result)
doc=f"""# Development-10 Hotfix-01 (2026-10-17)

DIAGNOSTIC / UNFINISHED. BATTLE STATUS: UNAVAILABLE.

The user reported Berlin stuck at Loading Resources and asked which map
had worked. No map has completed a playable Rust battle. Previous Firefight
tests were startup diagnostics; Crete was a static asset viewer. The sealed
Development-10 GUI Start button had not been exercised in its validation.

## Delivery

- Folder: `release/Development-10-Hotfix-01`.
- EXE SHA-256: `{p.sha(exe)}` ({exe.stat().st_size} bytes).
- ZIP: `{archive.name}` ({archive.stat().st_size} bytes).
- ZIP SHA-256: `{p.sha(archive)}`.
- Package seal checks every payload hash, exact ZIP membership, CRC and
  preservation of all7,251 prior Dev08/09/10 files. Earlier releases remain
  unchanged. This package needs existing original Data.rwd/reconstruction
  assets and is not a standalone whole-game friend copy.

## What was fixed and observed

The old e8488970 client reproduced GUI starvation: actual Berlin Start
stopped servicing its next screenshot/quit for a25-second test. Heavy source
preparation and scoped worker join were inside GUI update. No pre-activation
UI bridge deadlock was demonstrated.

The successor prepares source-only Send data in a background job and polls
the actual owned ResourceWorker without blocking unfinished work. Actual
ApplicationThread capture, Host/reset/launch and on_created registration
remain on GUI; child release occurs afterward. Same owners/native task body,
no fake readiness, activation, viewer-as-battle or original-engine fallback.
The GUI now displays a spinner and real elapsed time. Global Fog reads the
same published spatial owner after map adoption.

| Real GUI test | Observed result |
|---|---|
| Fall of Berlin / (2v2)Berlin | Actual Start, responsive captures1/3/10/30/60/90/125/155s. Still pending155s; scripted close, exit0 at187.875s. |
| DeveloperMaps/(3v3)Firefight | Actual Start, active menu button; Custom returns to same pending attempt. Genuine WorldStart classifier error visible by125s and still displayed220s. Exit0. |

Native worker cleanup can still wait during close (observed about30s on
Berlin). No interruptible-loader, faster loading or hard latency claim.
Source-only unfinished jobs detach; arbitrary panic unwinding is not covered
by a generic retained-state error prefix. No playable battle accepted.

CLI8/8, mechanics55/55 and world24/24 bounded checks passed.80 map structures,
225,851 placements and1,847 model catalog entries verified. Firefight record
restoration2386/0 failures is separate from startup and reports battle_ready
false. New HQ/Crete Vulkan and infantry DX12 static previews were directly
inspected; original visual/gameplay parity remains unfinished. GUI timings
are diagnostics, with CLI checks overlapping part of Firefight run, not a
performance benchmark. Static dependency resolution passed on this PC only.

## Evidence and continuation

Read `docs/NATIVE-PORT-RULES.md` first, then
`research/development10/ACTIVE-SESSION-20261017.md`.

- Exact EXE validation: `qa-output/development10/hotfix01-acceptance/validation.json`.
- Actual GUI scripts/logs/screenshots: `qa-output/development10/hotfix01-gui`.
- Build: `qa-output/development10/hotfix01-build/build-record.json`;2,075
  Cargo/crate source pins unchanged throughout release build. This is not a
  fingerprint of every external compiler input.
- Independent async and package reviews: `research/development10/gui-loading-stall-review`
  and `research/development10/independent-leader-hotfix01`.
- Exact predecessor copies for all9 changed existing files:
  `backups/source-checkpoint-20261017-loading-hotfix/pre-hotfix-source-index.json`.
  Five newly added source files are separately listed. No native EXE was changed.

Next reached battle blocker: native WorldStart4F1E10 object_active reuses
saved-only post_load_world::object_class, which rejects fresh Actor owners.
Read-only source/native proofs at
`research/development10/world-start-fresh-classification/findings.json`
and independent-minimal-classification-review.json recommend a local,
validated Source::New Actor constructor3 branch preserving native AL result
from vtable+B4 ->6A71D0. Preserve saved Group8 true behavior. Do not introduce
liveness/phase/flag gates or compare originally requestedNULL saved_id with
assigned currentGTID. The failed graph already has native partial effects;
never clear/retry it to pretend completion. This successor is RESEARCH ONLY
and is not implemented in this hotfix. Use a fresh47 strict startup after
the proven adapter fix, then real GUI integration again.

No natural Session/View activation, live rendered battle, native orders,
complete movement/combat/AI/victory, original full saves, multiplayer or
campaign acceptance. Separate internet/second-PC tests remain unavailable.
No defensible overall completion percentage/finish date is established.
The hotfix source freeze is complete; future source development may resume
without modifying sealed releases. All root test/build processes are reaped.
"""
path=p.ROOT/"docs/DEVELOPMENT-10-HOTFIX-01-20261017.md"
with path.open("x",encoding="utf-8") as f:f.write(doc)
journal=f"""

## Development-10 Hotfix-01 SEALED (2026-10-17)
Exact release EXE{p.sha(exe)} ({exe.stat().st_size}bytes), build476.66s/defaultx64MSVCrelease/2075sourcepinsunchanged. Separatefolderrelease/Development-10-Hotfix-01, prior08/09/10all7251filesprotected. ZIP{archive.name} {archive.stat().st_size}bytes SHA{p.sha(archive)}. Diagnosticseal/everypayloadSHA/ZIPCRC/oldreleasepreservationPASS. No battle/standalone/fullparityclaim.
ActualBerlin GUI Startresponsive1..155s, stillLoading155s, scriptedclose exit0total187.875s (nativecleanupwait~30s); no Berlinstartupcompletion/erroraccepted. ActualFirefightStart+Mainmenu+returnCustomkeepssameattempt, stillLoading91s thenby125s genuineWorldStartPostLoadfresh-object error, sameerror220s, exit0total227.844s. GUIscriptsandresultsisolated; no userprocesskilled. CLI8/8PASS, mechanics55/world24, all3newpreviewsdirectlyviewed; noFPS/visparityclaim. Exactvalidationhotfix01-acceptance/validation.json; finalindependentreview6a8b1f4c... . Nativeoriginal/v14hashespreserved. All9existingchangedfilesbackedup exactoldDev10hashes +5newlisted.
Continuationdocs/DEVELOPMENT-10-HOTFIX-01-20261017.md. NextactualWorldStart4F1E10 freshActorclass3B4 proofavailable READONLY, notimplemented: world-start-fresh-classification/findings.json (50b46108...) +Stateindependent2959e48c... . PreserveGroup8 semantics/actualGTID notrequestedsaved_id andnativepartialeffects. Sourcefreezecomplete; noactiveownedbuild/testprocesses. Still0completeplayablebattleaccepted.
"""
with (p.ROOT/"research/development10/ACTIVE-SESSION-20261017.md").open("a",encoding="utf-8") as f:f.write(journal)
print(path)
