"""Read-only comparison of Phase 6 independent-review source pins.

Writes only qa-output/phase6/review-pin-audit.json. A byte mismatch is not a
semantic failure; a newer review's scope must be checked before rebinding pins.
"""
from pathlib import Path
from datetime import datetime, timezone
from collections import Counter, defaultdict
import hashlib, json, re

ROOT = Path(__file__).resolve().parents[1]
HASH = re.compile(r'^[0-9a-fA-F]{64}$')
cache = {}
snapshots = {}
def sha(path):
    path = path.resolve()
    if path not in cache:
        cache[path] = hashlib.sha256(path.read_bytes()).hexdigest()
        stat=path.stat()
        snapshots[path]=(stat.st_mtime_ns,stat.st_size)
    return cache[path]
def label(path):
    try: return path.resolve().relative_to(ROOT).as_posix()
    except ValueError: return str(path.resolve()).replace('\\','/')
def resolve(path, report):
    path = Path(path.replace('\\', '/'))
    if path.is_absolute(): return path
    if path.as_posix().startswith(('crates/', 'research/', 'qa-output/', '../')):
        return (ROOT/path).resolve()
    return (report.parent/path).resolve()

SPECIAL = {
    ('phase6-actors', 'source_evidence_sha256'): 'evidence.json',
    ('phase6-weapon', 'author_ledger_sha256'): 'findings.json',
    ('phase6-experience', 'test_log_sha256'): 'independent-root-tests.log',
    ('phase6-spatial', 'producer_ledger_sha256'): 'terrain-findings.json',
    ('phase6-terrain', 'metadata_sha256'): 'terrain-render-metadata.json',
    ('phase6-terrain', 'fixtures_sha256'): 'compositor-fixtures.json',
}
def pins(document, report):
    output = []
    def add(path, expected, field, kind='file', transform=None):
        output.append(dict(path=label(path), expected_sha256=expected.lower(),
                           report_field=field, pin_kind=kind, transform=transform))
    def visit(value, field=''):
        # This explicitly named field is retained historical evidence of the
        # pre-integration files, not a claim about the current file contents.
        if field.endswith('.prior_related_pins'):
            return
        if isinstance(value, dict):
            for key, item in value.items():
                key_field = f'{field}.{key}' if field else key
                if isinstance(item, str) and HASH.fullmatch(item) and ('/' in key or '\\' in key):
                    add(resolve(key, report), item, key_field)
                elif key == 'sha256' and isinstance(item, str) and HASH.fullmatch(item):
                    named = value.get('path', value.get('log'))
                    if isinstance(named, str): add(resolve(named, report), item, key_field)
                else: visit(item, key_field)
        elif isinstance(value, list):
            for index, item in enumerate(value): visit(item, f'{field}[{index}]')
    visit(document)
    for field in ['original_sha256', 'v14_sha256', 'accepted_sha256']:
        if isinstance(document.get(field), str) and HASH.fullmatch(document[field]):
            path = ROOT.parent/('AA.exe' if field == 'original_sha256' else 'reconstruction/native-game/runtime-expanded-v14/AA.exe')
            add(path, document[field], field, 'native_image')
    for (folder, field), filename in SPECIAL.items():
        if report.parent.name == folder and isinstance(document.get(field), str):
            add(report.parent/filename, document[field], field, 'evidence_reference')
    if 'lib_health_scope_sha256' in document:
        # Exact producer algorithm in phase6-combat/independent_review.py: only
        # removes the dying export, normalizes lines, and omits the final newline.
        add(ROOT/'crates/aa-body/src/lib.rs', document['lib_health_scope_sha256'],
            'lib_health_scope_sha256', 'scoped_source',
            'LF join of splitlines excluding line.strip()==pub mod dying;')
    if report.name == 'independent-material-source-review.json':
        for index, entry in enumerate(document.get('masks', [])):
            relative = Path(entry['original'].replace('\\', '/'))
            add(ROOT.parent/'reconstruction/assets/original/Data'/relative,
                entry['source_sha256'], f'masks[{index}].source_sha256', 'authored_asset')
            add(ROOT.parent/'reconstruction/assets/converted/textures'/relative.with_suffix('.png'),
                entry['png_sha256'], f'masks[{index}].png_sha256', 'converted_asset')
    return output

def owner(path):
    if path.startswith('crates/aa-client/'): return 'root plus material_render/terrain_roads'
    if path.startswith('crates/aa-mapworld/'): return 'scenegraph'
    if path.startswith('crates/aa-body/'): return 'core_review'
    if path.startswith('crates/aa-navigation/'): return 'navigation'
    if path.startswith('crates/aa-spatial/'): return 'orders_spatial'
    return 'report reviewer and component owner'

def main():
    reference=ROOT/'research/phase6-weapon/independent-review.json'
    assert resolve('crates/aa-combat/src/lib.rs',reference)==ROOT/'crates/aa-combat/src/lib.rs'
    assert resolve('crates\\aa-combat\\src\\lib.rs',reference)==ROOT/'crates/aa-combat/src/lib.rs'
    assert resolve('findings.json',reference)==reference.parent/'findings.json'
    reports=[]
    # Reports whose basename states review, excluding replay result inventories.
    paths=sorted(p for p in (ROOT/'research').glob('phase6*/*.json')
                 if 'review' in p.stem and 'replay' not in p.stem)
    for report in paths:
        document=json.loads(report.read_text(encoding='utf-8-sig'))
        declared = document.get('passed', document.get('status', document.get('result')))
        declared_passed = declared is True or (isinstance(declared,str) and declared.lower().startswith('pass'))
        entries=pins(document,report)
        for entry in entries:
            path=Path(entry['path'])
            if not path.is_absolute(): path=ROOT/path
            if not path.is_file():
                entry.update(current_sha256=None,status='missing')
            else:
                actual=sha(path)
                if entry['transform']:
                    content='\n'.join(line for line in path.read_text().splitlines() if line.strip()!='pub mod dying;')
                    actual=hashlib.sha256(content.encode()).hexdigest()
                    entry['whole_file_current_sha256']=sha(path)
                entry.update(current_sha256=actual,status='match' if actual==entry['expected_sha256'] else 'mismatch')
        problems=[entry for entry in entries if entry['status']!='match']
        scope=document.get('scope',document.get('review_scope',document.get('component', document.get('reviewed_behavior',document.get('source_review')))))
        report_row={'report':label(report),'report_sha256':sha(report),'reviewer':document.get('reviewer'),
                    'declared_result':declared,'declared_passed':declared_passed,'scope':scope,
                    'pin_status':'mismatches' if problems else ('all_explicit_pins_match' if entries else 'no_explicit_file_pins'),
                    'counts':dict(Counter(entry['status'] for entry in entries)), 'pins':entries,
                    'review_limits':document.get('limits',document.get('limitations',document.get('remaining_limits',[])))}
        reports.append(report_row)
    matching=defaultdict(list)
    for report in reports:
        if report['declared_passed']:
            for pin in report['pins']:
                if pin['status']=='match' and pin['pin_kind']!='scoped_source':
                    matching[(pin['path'],pin['current_sha256'])].append({'report':report['report'],'scope':report['scope']})
    refresh=[]
    for report in reports:
        for pin in report['pins']:
            if pin['status']!='match':
                refresh.append({'report':report['report'],'reviewer':report['reviewer'],
                    'path':pin['path'],'pin_kind':pin['pin_kind'],'expected_sha256':pin['expected_sha256'],
                    'current_sha256':pin['current_sha256'],'status':pin['status'],
                    'owner_hint':owner(pin['path']),
                    'current_whole_file_pinned_by':matching[(pin['path'],pin.get('whole_file_current_sha256',pin['current_sha256']))],
                    'action':'Owner/reviewer must inspect change and scope before refreshing; audit does not approve or rewrite this pin.'})
    changed_during_audit=[]
    for path,signature in snapshots.items():
        stat=path.stat()
        if (stat.st_mtime_ns,stat.st_size)!=signature:
            current=hashlib.sha256(path.read_bytes()).hexdigest()
            if current!=cache[path]:
                changed_during_audit.append({'path':label(path),'first_sha256':cache[path],'latest_sha256':current})
    output={'schema':1,'generated_utc':datetime.now(timezone.utc).isoformat(),
        'scope':'Explicit file/image/known evidence/scoped health pins in research/phase6*/ independent review reports. Replay inventories and standalone native vector files are not review reports.',
        'read_only_sources':True,'producer_sha256':sha(Path(__file__)),
        'snapshot_consistent':not changed_during_audit,'changed_during_audit':changed_during_audit,
        'summary':{'reports':len(reports),'declared_passed':sum(x['declared_passed'] for x in reports),
            'reports_all_explicit_pins_match':sum(x['pin_status']=='all_explicit_pins_match' for x in reports),
            'reports_with_mismatches':sum(x['pin_status']=='mismatches' for x in reports),
            'reports_without_explicit_file_pins':sum(x['pin_status']=='no_explicit_file_pins' for x in reports),
            'pins':sum(len(x['pins']) for x in reports),'mismatched_or_missing_pins':len(refresh)},
        'passed_current_pin_reports':[x['report'] for x in reports if x['declared_passed'] and x['pin_status']=='all_explicit_pins_match'],
        'owner_refresh_required':refresh,'reports':reports,
        'limitations':['Byte mismatches may be legitimate additive or formatting changes; this audit does not infer semantic failure or automatically expand review scope.',
            'Explicit prior_related_pins maps record historical pre-integration hashes and are excluded from current-file equality checks.',
            'A matching pin in another review is reported for context only; its bounded scope does not automatically refresh an older report.',
            'Native instruction-range hashes are not file hashes and are outside this source-pin audit.',
            'Reports with no explicit source-file pins are listed without inferring that their reviewed implementation is current.',
            'Report-only claims of source integrity are not rerun as native/GPU/corpus tests.']}
    destination=ROOT/'qa-output/phase6/review-pin-audit.json'
    destination.parent.mkdir(parents=True,exist_ok=True)
    destination.write_text(json.dumps(output,indent=2)+'\n')
    print(json.dumps({'summary':output['summary'],'output':label(destination),'sha256':sha(destination),
                      'refresh':[{'report':x['report'],'path':x['path'],'has_other_current_pin':bool(x['current_whole_file_pinned_by'])} for x in refresh]},indent=2))
if __name__=='__main__':main()
