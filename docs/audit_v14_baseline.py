"""Read the accepted v14 manifest and record stock-vs-v14 data precedence.

Only the new Rust evidence ledger is written. No runtime/profile/old source is
changed. This records the preservation target, not implemented Rust gameplay.
"""
from pathlib import Path
import difflib, hashlib, json

ROOT=Path(__file__).resolve().parents[1]
OLD=ROOT.parent/'reconstruction'
NATIVE=OLD/'native-game'
RUNTIME=NATIVE/'runtime-expanded-v14'
MANIFEST=NATIVE/'expanded-v14-runtime-manifest.json'
MANIFEST_SHA='90a18261597d7d4db8ba56ebaa2797c18433813e35bf07b825eaddf039cdfccd'
CONTRACT=NATIVE/'mods/performance-v14/work/performance-v14-contract.json'
CONTRACT_SHA='8355c3454557c565347bbed94c93b518bdfa7601fac4cc2d121cc90727734234'


def sha(path):
    with path.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()


def reference(path):return {'path':path.relative_to(ROOT.parent).as_posix(),'sha256':sha(path),'bytes':path.stat().st_size}


def main():
    assert sha(MANIFEST)==MANIFEST_SHA and sha(CONTRACT)==CONTRACT_SHA
    manifest=json.loads(MANIFEST.read_text());assert manifest['file_count']==54 and manifest['total_bytes']==1107523614
    overrides=[];archives=[];files=[]
    for row in manifest['files']:
        path=row['path'];source=RUNTIME/path
        assert source.stat().st_size==row['size'] and sha(source)==row['sha256'],path
        files.append({k:row[k] for k in ('path','size','sha256')})
        if path.lower().endswith('.rwd'):
            installed=ROOT.parent/path
            assert sha(installed)==row['sha256'],path
            archives.append({'path':path,'sha256':row['sha256'],'size':row['size'],'unchanged_from_installed_archive':True})
        if path.startswith('Data/'):
            original=OLD/'assets/original'/path
            old=original.read_bytes() if original.exists() else None
            new=source.read_bytes()
            lines=list(difflib.unified_diff([] if old is None else old.decode('cp1252').splitlines(),
                       new.decode('cp1252').splitlines(),fromfile='stock/'+path,tofile='v14/'+path,n=1))
            overrides.append({'path':path,'kind':'added' if old is None else 'replaces_archive_member',
                'v14_sha256':row['sha256'],'v14_bytes':row['size'],
                'stock_sha256':None if old is None else hashlib.sha256(old).hexdigest(),
                'stock_bytes':None if old is None else len(old),
                'line_diff_sha256':hashlib.sha256(('\n'.join(lines)+'\n').encode()).hexdigest(),
                'line_diff_added':sum(s.startswith('+') and not s.startswith('+++') for s in lines),
                'line_diff_removed':sum(s.startswith('-') and not s.startswith('---') for s in lines)})
    assert len(archives)==10 and not any(r['path'].casefold().startswith('hdart/') for r in files)
    hotkey_path=NATIVE/'mods/camera-controls/wasd/wasd-hotkeys-mod.json'
    hotkeys=json.loads(hotkey_path.read_text())
    assert hotkeys['passed'] and sum(len(f['changed_bindings']) for f in hotkeys['files'])==81
    assert all(next(r for r in files if r['path']==f['path'])['sha256']==f['sha256'] for f in hotkeys['files'])
    native_proofs=[
      NATIVE/'mods/camera-controls/zoom/zoom-native-proposal.json',
      NATIVE/'mods/camera-controls/wasd-v12/wasd-native-proposal.json',
      NATIVE/'mods/camera-controls/crash-v12/text-reset-native-proposal.json',
      NATIVE/'mods/camera-controls/original-only-v13/original-only-native-proposal.json',
      NATIVE/'mods/openspy/openspy-native-proposal.json',
      NATIVE/'mods/lan-discovery/lan-discovery-native-proposal.json',
      NATIVE/'mods/dpi-scaling/dpi-scaling-native-proposal.json',
      NATIVE/'mods/fullscreen/fullscreen-native-proposal.json']
    report={'schema':1,'passed':True,'meaning':'Verified preservation baseline; not Rust gameplay parity.',
      'producer_sha256':sha(Path(__file__)),'manifest':reference(MANIFEST),'contract':reference(CONTRACT),
      'native_sha256':manifest['native_sha256'],'immutable_files':54,'immutable_bytes':manifest['total_bytes'],
      'target':'Accepted expanded v14 Original-only game; stock data is lower-priority source evidence.',
      'mount_precedence':'Verified v14 loose Data files replace matching original Data.rwd members; added files participate in their native loader stages. Do not flatten whole-file replacement into an unrelated definition merge.',
      'rules_profile':{'GameTicksPerGameSecond':16,'EconomyLimitedResourceMax':100,'building_limit_provided_start':100,
        'KingdomsMax':16,'MultiplayerMaxNumPlayers':16,'broadband_maxplayers':16,'modem_maxplayers':4,
        'stock_map_sizes':[320,384,448,512,576,640],'added_map_sizes':[768,1024],
        'headquarters_capacity_rules':'unchanged;100 is the ceiling, not free earned slots',
        'gameplay_camera_zoom_max':2.1,'stock_gameplay_camera_zoom_max':1.4,'gameplay_camera_zoom_min':0.7,
        'rts_camera_pan':'WASD plus arrows; held-input repair retained','displaced_action_bindings':81,
        'strategic_custom_hotkeys':'unchanged','hd_art':'absent; all original archives retained'},
      'v14_files':files,'original_archives':archives,'data_overrides':overrides,
      'hotkey_proof':reference(hotkey_path),
      'hotkey_counts':[{ 'path':f['path'],'changed_bindings':len(f['changed_bindings'])} for f in hotkeys['files']],
      'native_behavior_evidence':[reference(p) for p in native_proofs],
      'limitations':['This audit checks current file identities and exact source replacement differences; native behavior proofs have their own scoped historical validation.',
        'No existing user preferences, profiles, credentials or saves are read/copied.',
        'No full Rust native simulation, RSG/network parity or gameplay parity is claimed.',
        'OpenSpy endpoint integration is present; successful end-to-end Internet play was not established by the preserved release evidence.']}
    target=ROOT/'research/evidence/v14-baseline.json';target.parent.mkdir(parents=True,exist_ok=True)
    target.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'path':str(target),'sha256':sha(target),'data_override_count':len(overrides),
      'added_data_files':sum(r['kind']=='added' for r in overrides),'hotkey_counts':report['hotkey_counts']},indent=2))


if __name__=='__main__':main()
