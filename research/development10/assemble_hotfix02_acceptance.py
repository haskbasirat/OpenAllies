"""Assemble exact diagnostic HF02 evidence without equating startup to battle."""
import argparse
import package_hotfix02 as p

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument("--independent-final",default=None)
args=parser.parse_args()
out=p.ROOT/"qa-output/development10/hotfix02-acceptance"
staged=p.read(p.RELEASE/"STAGING.json")
digest=p.sha(p.RELEASE/p.EXE)
assert digest==staged["executable_sha256"]
for name in ("cli-results.json","visual-review.json","gui-review.json"):
    record=p.read(out/name)
    assert record["passed"] is True and record["executable_sha256"]==digest,name
cli=p.read(out/"cli-results.json")
assert len(cli["checks"])==8 and all(r["passed"] for r in cli["checks"])
startup=p.read(out/"startup-review.json")
assert startup["passed"] and startup["source_files_match_gui_build"]
platform="qa-output/development10/hotfix02-platform/"
architecture=p.read(p.ROOT/platform/"architecture.json")
assert architecture["passed"] and architecture["executable_sha256"]==digest
assert p.read(p.ROOT/platform/"native-executables-preserved.json")["passed"]
assert p.read(p.ROOT/"qa-output/development10/hotfix02-package-tests/preparation-report.json")["passed"]
gui=p.read(out/"gui-review.json")

def local(name):return "qa-output/development10/hotfix02-acceptance/"+name
checks=[
    ("architecture-dependencies",[platform+"architecture.json",platform+"native-executables-preserved.json",platform+"system-dll-validation.json",platform+"results.json"]),
    ("asset-verification",[local("asset-verification.json")]),
    ("package-cli",[local("cli-results.json"),local("mechanics-fragments.json"),local("world-fragments.json"),local("record-loading.json")]),
    ("render-review",[local("visual-review.json"),local("hq-vulkan.png"),local("map-vulkan.png"),local("infantry-dx12.png")]),
    ("unfinished-battle-ui",[local("gui-review.json")]),
    ("targeted-startup-fixes",[local("startup-review.json"),
        "research/development10/berlin-clock-owner/author-findings.json",
        "research/development10/world-start-fresh-classification/implementation/findings.json"]),
    ("independent-review",[
        "research/development10/berlin-clock-owner/independent-leader-clock-implementation-review.json",
        "research/development10/world-start-fresh-classification/implementation/independent-leader-implementation-review.json",
        "research/development10/independent-leader-hotfix02/candidate-stage-precheck01.json",
        "qa-output/development10/hotfix02-package-tests/independent-leader-review.json"]),
]
if args.independent_final:
    review=p.read(p.ROOT/args.independent_final)
    assert review["passed"] is True
    checks[-1][1].append(args.independent_final)
files={name for _,names in checks for name in names}
files.update([
    "qa-output/development10/hotfix02-package-tests/preparation-report.json",
    "qa-output/development10/hotfix02-package-tests/synthetic-tests.json",
    "qa-output/development10/hotfix02-package-tests/synthetic-tests.log",
    "research/development10/validate_hotfix02.py",
    "research/development10/validate_hotfix02_gui.py",
    "research/development10/build_hotfix02_startup_test.py",
    "research/development10/run_hotfix02_startup_test.py",
    "research/development10/assemble_hotfix02_acceptance.py",
    "research/development10/berlin-clock-owner/native-review/findings.json",
    "research/development10/berlin-clock-owner/native-review/native-clock-read-probe-v2.json",
    "research/development10/berlin-clock-owner/caller-review.json",
    "research/development10/berlin-clock-owner/tests-refresh01.log",
    "research/development10/berlin-clock-owner/tests-fog-core01.log",
    "research/development10/berlin-clock-owner/tests-zone-services01.log",
    "research/development10/berlin-clock-owner/tests-actual-published01.log",
    "research/development10/berlin-clock-owner/tests-actual-fog01.log",
    "research/development10/world-start-fresh-classification/implementation/actual-source-test01.log",
    "research/development10/world-start-fresh-classification/implementation/regression-test01.log",
    "research/development10/world-start-fresh-classification/independent-implementation-source-review.json",
    "backups/source-checkpoint-20261017-hotfix02/pre-hotfix-source-index.json",
    "qa-output/development10/hotfix02-native-startup/build-record.json",
])
for map_name in ("berlin","firefight"):
    folder=p.ROOT/"qa-output/development10/hotfix02-gui"/map_name
    result=p.read(folder/"result.json")
    assert result["executable_sha256"]==digest and result["exit_code"]==0 and not result["timed_out"]
    for name in ("script.json","result.json","process.json","stdout.txt","stderr.txt"):
        files.add((folder/name).relative_to(p.ROOT).as_posix())
    native=p.ROOT/"qa-output/development10/hotfix02-native-startup"/map_name
    result=p.read(native/"result.json")
    assert result["strict_startup_passed"] and result["source_unchanged_since_build"]
    for name in ("result.json","process.json","attempt.main-launch.json","attempt.worker.json","stdout.txt","stderr.txt"):
        files.add((native/name).relative_to(p.ROOT).as_posix())
for row in gui["evidence_files"]:files.add(row["path"])
validation=dict(schema=1,passed=True,release_mode="diagnostic",battle_status="unavailable",
    playable_battle=False,full_game_parity=False,claims_native_battle=False,
    unfinished_battle_ui_labelled=True,executable_sha256=digest,
    scope="Exact packaged-client diagnostic startup and static previews on this PC. Fresh Berlin and Firefight strict mission startup passed on a distinct QA executable; actual GUI observations are separately bounded. No playable battle/full-game acceptance.",
    checks=[dict(name=name,status="passed",evidence_files=names) for name,names in checks],
    evidence_files=[p.record(p.ROOT/name) for name in sorted(files)],
    limitations=[
        "Both fresh strict Host/resource/StartedMission tests passed. This does not imply frontend activation, native View/task readiness, battle control or simulation.",
        "Actual GUI Start was clicked on both maps; final statuses and remaining limitations are in gui-review.json. No missing callback is converted into success.",
        "Clock-owner and fresh-classifier regressions prove only their specific source/owner/native predicate domains; no blanket new memory/factory/world parity.",
        "Native worker shutdown can join unfinished work; source-only unfinished jobs detach. No interruptible-loader or hard latency guarantee.",
        "Requires original Data.rwd/reconstruction assets, x64 MSVC runtime and suitable graphics drivers; not a standalone whole game.",
        "Build fingerprints cover Cargo/crates, not every external compiler include. Test-profile startup and default-profile GUI EXEs are distinct and separately hashed.",
        "Static previews are asset renders, with documented incomplete native Model/View, terrain, lighting and animation behavior.",
    ],
    not_tested=["Complete playable battle, native orders, full movement/combat/AI/victory", "Whole-game original saves and campaign", "Multiplayer or second PC/different internet connection", "Clean-PC standalone deployment", "150% laptop DPI and every original map", "Performance/SSD/VRAM/bandwidth benchmark", "Full client unit-test suite"])
destination=out/("validation.json" if args.independent_final else "validation-draft.json")
p.write_new(destination,validation)
print(destination)
