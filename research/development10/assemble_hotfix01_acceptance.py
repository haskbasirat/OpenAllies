"""Assemble bounded diagnostic evidence; semantic GUI/render review is external."""
from pathlib import Path
import argparse
import package_hotfix01 as p

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument("--independent-final",default=None)
args=parser.parse_args()
out=p.ROOT/"qa-output/development10/hotfix01-acceptance"
staged=p.read(p.RELEASE/"STAGING.json")
digest=p.sha(p.RELEASE/p.EXE)
assert digest==staged["executable_sha256"]
cli=p.read(out/"cli-results.json")
assert cli["passed"] is True and cli["executable_sha256"]==digest
assert len(cli["checks"])==8 and all(r["passed"] for r in cli["checks"])
visual=p.read(out/"visual-review.json")
assert visual["passed"] is True and visual["executable_sha256"]==digest
gui=p.read(out/"gui-review.json")
assert gui["passed"] is True and gui["executable_sha256"]==digest
platform="qa-output/development10/hotfix01-platform/"
architecture=p.read(p.ROOT/platform/"architecture.json")
assert architecture["passed"] is True and architecture["executable_sha256"]==digest
preserved=p.read(p.ROOT/platform/"native-executables-preserved.json")
assert preserved["passed"] is True
tests=p.read(p.ROOT/"qa-output/development10/hotfix01-package-tests/results.json")
assert tests["passed"] is True and tests["tests_run"]==13

def local(name): return "qa-output/development10/hotfix01-acceptance/"+name
checks=[
    ("architecture-dependencies",[platform+"architecture.json",platform+"native-executables-preserved.json",platform+"system-dll-validation.json",platform+"results.json"]),
    ("asset-verification",[local("asset-verification.json")]),
    ("package-cli",[local("cli-results.json"),local("mechanics-fragments.json"),local("world-fragments.json"),local("record-loading.json")]),
    ("render-review",[local("visual-review.json"),local("hq-vulkan.png"),local("map-vulkan.png"),local("infantry-dx12.png")]),
    ("unfinished-battle-ui",[local("gui-review.json")]),
    ("independent-review",[
        "research/development10/gui-loading-stall-review/async-successor-review01.json",
        "research/development10/independent-leader-hotfix01/helper-plan-docs-review01.json",
        "research/development10/independent-leader-hotfix01/built-staged-review01.json"]),
]
if args.independent_final:
    final=p.read(p.ROOT/args.independent_final)
    assert final["passed"] is True
    checks[-1][1].append(args.independent_final)
files={name for _,names in checks for name in names}
files.update([
    "qa-output/development10/hotfix01-package-tests/results.json",
    "qa-output/development10/hotfix01-package-tests/unittest.log",
    "research/development10/validate_hotfix01.py",
    "research/development10/validate_hotfix01_gui.py",
    "research/development10/record_hotfix01_gui_review.py",
    "research/development10/assemble_hotfix01_acceptance.py",
    "research/development10/nonblocking-startup/transport-findings.json",
    "research/development10/global-fog-adopted/findings.json",
    "research/development10/global-fog-adopted/independent-state-binding-review.json",
    "research/development10/global-fog-adopted/actual46-acceptance.json",
    "research/development10/playable-startup/attempt46.result.json",
    "qa-output/development10/loading-hang-20261017/berlin-repro-result.json",
    "qa-output/development10/loading-hang-20261017/berlin-repro-script.json",
    "qa-output/development10/loading-hang-20261017/berlin-repro.stdout.txt",
])
for map_name in ("berlin","firefight"):
    folder=p.ROOT/"qa-output/development10/hotfix01-gui"/map_name
    result=p.read(folder/"result.json")
    assert result["executable_sha256"]==digest and result["exit_code"]==0 and result["timed_out"] is False
    assert result["screenshot_count"] >= 9
    for name in ("script.json","result.json","process.json","stdout.txt","stderr.txt","before-start.png","after-001s.png","after-030s.png","after-155s.png"):
        files.add((folder/name).relative_to(p.ROOT).as_posix())
    if map_name=="firefight":
        for name in ("menu-during-loading.png","resumed-same-loading.png"):
            files.add((folder/name).relative_to(p.ROOT).as_posix())
for row in gui.get("evidence_files",[]):
    files.add(row["path"])
validation=dict(schema=1,passed=True,release_mode="diagnostic",battle_status="unavailable",
    playable_battle=False,full_game_parity=False,claims_native_battle=False,
    unfinished_battle_ui_labelled=True,executable_sha256=digest,
    scope="Diagnostic loading-responsiveness hotfix and existing previews on this PC with original external assets. No completed battle or full-game parity.",
    checks=[dict(name=name,status="passed",evidence_files=names) for name,names in checks],
    evidence_files=[p.record(p.ROOT/name) for name in sorted(files)],
    limitations=[
        "GUI Start was actually clicked on Berlin and Firefight. Responsiveness/error reporting are separate from successful battle startup.",
        "Neither attempt establishes completed startup. Berlin remained loading through155s before the scripted close; Firefight's final observed status is in gui-review.json. No map has completed a playable Rust battle.",
        "Native resource/simulation worker shutdown can still join unfinished work; no interruptible-loader guarantee.",
        "Source-only unfinished jobs detach. Generic retained-state error prefixes do not prove retention across arbitrary panics.",
        "CLI/native fragment tests and static previews prove only their documented bounds; no whole Model/View/gameplay parity.",
        "Build pins cover Cargo/crates, not every external compiler input. Strict run46 uses a different QA EXE with shared-source-drift qualifications.",
        "Requires existing original assets, x64 MSVC runtime and compatible graphics drivers; not a standalone whole-game friend package.",
    ],
    not_tested=["Completed live battle, orders, full movement/combat/AI/victory", "Whole-game original saves and campaign", "Multiplayer login/lobby/hosting/joining/save transfer or second internet PC", "Clean-PC standalone deployment", "150% laptop DPI or every map", "Hardware/input-to-frame latency or performance benchmark", "Full client unit-test executable (Controller regression typechecked only)"])
destination=out/("validation.json" if args.independent_final else "validation-draft.json")
p.write_new(destination,validation)
print(destination)
