"""Record final acceptance facts only after the executable matrices finish."""
from pathlib import Path
import hashlib, json, re

root=Path(__file__).resolve().parents[1]
qa=root/'qa-output/phase6'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
exe=root/'target/x86_64-pc-windows-msvc/release/AxisAlliesRust.exe'
tests=(qa/'workspace-tests-final.txt').read_text(encoding='utf-8-sig')
count=sum(map(int,re.findall(r'test result: ok\. (\d+) passed;',tests)))
assert count==280 and 'FAILED' not in tests and read(qa/'workspace-validation.json')['passed']
maps=read(qa/'all-maps-final/report.json')
clips=read(qa/'animations-final/report.json')
checks=read(qa/'executable-check.json')
visual=read(qa/'visuals-final/report.json')
assert checks['passed'] and visual['passed']
assert len(maps['maps'])==80 and all(row['exit_code']==0 for row in maps['maps'])
assert len(clips['results'])==85 and all(row['exit_code']==0 for row in clips['results'])
assert maps['executable_sha256']==clips['executable_sha256']==checks['executable_sha256']==visual['exe_sha256']==sha(exe)
unsupported=[]
for row in maps['maps']:
    for warning in row['result'].get('warnings',[]):
        assert 'Original water source binding failed' not in warning
        match=re.search(r'(\d+) unresolved cells skipped',warning)
        if match and int(match[1]):unsupported.append({'map':row['id'],'cells':int(match[1]),'warning':warning})
visual['visual_inspection']={
    'reviewer':'root','images':['crete-vulkan.png','crete-dx12.png','corps.png','oil.png'],
    'observations':'Road alignment and blended terrain edges are retained. GBR Corps and Oil Depot show continuous textured surfaces instead of the earlier missing/dark texture regions. Both Crete backend images show water and the same scene layout.',
    'limits':'No identical-camera native screenshot was captured; lighting, pitch/roll and other documented fidelity gaps remain.'}
(qa/'visuals-final/report.json').write_text(json.dumps(visual,indent=2))
(qa/'water-limitations.json').write_text(json.dumps({'scope':'Explicit unsupported WaterInfo cells in the final80-map render sweep','maps':unsupported,'total_cells':sum(r['cells'] for r in unsupported)},indent=2))
review=read(qa/'review-pin-audit.json')
assert review['summary']['mismatched_or_missing_pins']==0
text=f'''DEVELOPMENT 06 — VALIDATION

Final Windows x86-64 executable
SHA256: {sha(exe)}
Intel i7-11700KF / NVIDIA GeForce RTX 3080, local Windows host.

- {count} Rust workspace release tests passed, zero failed or ignored, including
  original source corpora and actual GPU tests. Strict all-target workspace
  Clippy passed. Cargo.lock and the Windows MSVC target were used.
- 85 final executable diagnostic invocations passed from E:/Games, outside the
  project: 24 embedded world samples,53 mechanics comparisons, catalog/actor/
  economy checks, and all80 original map inputs. The embedded and raw-map
  checks passed with a deliberately nonexistent recovered-asset argument.
- All80 inputs decode225851 placements,160 height layers,328 saved Kingdoms,
  21 Teams and11701 actor-owner references:7512 resolved non-null and4189 null.
  No generation mismatch, unresolved non-null owner or unsupported base record.
  This does not instantiate a running world or restore a complete original save.
- All80 maps render successfully with the final executable under Vulkan.
  All85 animation bindings across46 rigs render successfully.
- Five additional final renders cover Crete on Vulkan and Direct3D12, Stalingrad,
  GBR Corps and GBR Oil Depot. Root visually inspected both Crete backends and
  both buildings. No identical-camera native screenshot comparison was made.
- Crete61 repeated frames rebuild native ordering once. Source/GPU tests also
  check direction changes, pose invalidation, invalid-upload retention and live
  team-color changes inside an existing bundle. Timings are CPU encode/submit
  plus validation, not isolated GPU time, gameplay FPS or a controlled speedup.

Native component and review scope
{review['summary']['reports']} independent review reports are included. The final audit checks
{review['summary']['pins']} explicit file/evidence pins with no stale or missing pins. One terrain
report has no explicit file hashes itself and links its separately pinned source
report; historical prior hashes are preserved but not compared to current files.
Reports distinguish original instruction execution, intercepted callbacks,
Rust library comparisons, source corpora and actual GPU tests.

New evidence covers native definitions/construction/load shells, map/header/
Team/Kingdom/actor/recruitment decoding, tile/region paths and costs, dynamic
collision, height binding, weapon-to-Body damage, periodic effects, corpse/credit/
experience routing, production/economy sequences, and world queues. These are
bounded components, not full-match acceptance. Independent review caught and
fixed experience routing after a promotion callback mutates an ancestor.
All249 experience cases pass, including both new mutation regressions in all
three tested precision modes.

Rendering evidence includes native terrain masks/transforms/blend decisions,
geometry transparency keys/policy/order,888 water color cases,4608 selectors,
16 regular quads and80 saved water-cache readers. Water metadata preserves
original first-frame textures. No HD replacements are used.

Remaining limits and unavailable tests
Static water has {sum(r['cells'] for r in unsupported)} unsupported cells across{len(unsupported)} rendered maps because the
native fallback for missing WaterInfo is not implemented. Those cells are
explicitly counted/skipped; details are in evidence/water-limitations.json.
No map reported a water source-binding failure. Texture animation/scroll,
runtime shoreline rebuilding, lighting/gamma, generated bounds, pitch/roll,
road draping, shadows and full scenegraph behavior remain unfinished.

A complete playable match is NOT implemented. Full actor/component restoration,
live-world scheduling/commands, actual-map routing adapters, targeting/weapons,
AI, supply/morale/fog, original UI/audio, campaigns/strategic mode, full native
save/replay compatibility and multiplayer remain incomplete. Separate-PC and
separate-internet tests remain unavailable and were not claimed. Live laptop
DPI/multi-monitor GUI tests were not repeated. No game FPS improvement claimed.

Validation correction and preservation
The first executable gate failed only its path-string assertion: Rust returned
the Windows extended-length spelling of the same asset directory. The validator
now compares filesystem identity; all85 checks passed unchanged executable.
The original failed log is retained. An earlier water corpus test lacked its
AA_GAME_ROOT environment; it passed unchanged when set. Final full tests set
the original-data paths explicitly.

Source/cache backup: backups/development-05-source-before-phase6.zip, plus
backups/model-materials-before-phase6.json. Original/v14 executables, game data,
profiles/saves and Development01–05 packages were not edited.
Development06 requires the original recovered assets; it is not a standalone
whole-game distribution. Packaging gates final executable identity, reviews,
unchanged animation cache, AMD64/PerMonitorV2/asInvoker metadata and ZIP payload
hashes/CRC. Package and copied-executable results are in qa-output/phase6.
'''
(root/'docs/VALIDATION-06.txt').write_text(text,encoding='utf-8')
print(json.dumps({'tests':count,'maps':80,'animations':85,'water_unsupported':unsupported,'exe_sha256':sha(exe)}))
