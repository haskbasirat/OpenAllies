"""Read-only Development-09 integrity checks and optional isolated CLI checks.

Integrity checks never launch the program. --checks requires an explicit JSON
specification. Its logs/profile stay outside release and original user data.
Exit0 from a map-load diagnostic is not proof of a playable/complete load.
"""
from pathlib import Path
import argparse
import hashlib
import json
import os
import re
import subprocess
import zipfile

from package_development_09 import (ROOT,RELEASE,QA,ARCHIVE_NAME,EXE_NAME,
    require,sha,read,write_json,relative_path,verified_rows,inspect_pe)


def check_staged(folder,staged):
    inspect_pe(folder/EXE_NAME,staged['exe_sha256'])
    names=verified_rows(folder,staged['staged_files'])
    actual={p.relative_to(folder).as_posix() for p in folder.rglob('*') if p.is_file()}
    require(actual==names,'Unexpected/missing staged files')
    return {'phase':'staged','payload_files':len(names),'all_staged_payload_hashes_verified':True}


def check_sealed(folder,staged):
    manifest=read(folder/'manifest.json')
    require(manifest.get('native_runtime_diagnostics_connected') is True
            and manifest.get('playable_runtime') is False
            and manifest.get('full_game_parity') is False
            and manifest.get('standalone_game') is False,'Package scope flags are incorrect')
    require(sha(folder/EXE_NAME)==staged['exe_sha256'],'Packaged executable differs from validated candidate')
    validation=read(folder/'evidence/validation.json')
    require(validation.get('passed') is True and validation.get('executable_sha256')==staged['exe_sha256'],
            'Packaged validation does not identify this executable')
    verified_rows(folder,staged['staged_files'])
    names=verified_rows(folder,manifest['files'])
    require('manifest.json' not in names and ARCHIVE_NAME not in names,'Manifest cannot include itself or its containing ZIP')
    expected=names|{'manifest.json'}
    actual={p.relative_to(folder).as_posix() for p in folder.rglob('*') if p.is_file()}
    require(actual==expected|{ARCHIVE_NAME},'Unexpected/missing sealed files')
    with zipfile.ZipFile(folder/ARCHIVE_NAME) as archive:
        entries=archive.infolist()
        zip_names=[str(relative_path(row.filename)) for row in entries]
        require(len(zip_names)==len(set(zip_names))==len({v.casefold() for v in zip_names}),
                'Duplicate/case-colliding ZIP path')
        require(set(zip_names)==expected and all(not row.is_dir() for row in entries),
                'ZIP payload differs from manifest/folder')
        require(archive.testzip() is None,'ZIP CRC verification failed')
        for item in entries:
            path=folder/item.filename
            require(item.file_size==path.stat().st_size,'ZIP uncompressed size mismatch')
            with archive.open(item) as stream:
                require(hashlib.file_digest(stream,'sha256').hexdigest()==sha(path),
                        f'ZIP payload SHA256 mismatch: {item.filename}')
    return {'phase':'sealed','payload_files':len(expected),'all_payload_hashes_and_crc_verified':True}


def at(value,path):
    for part in path.split('.'):
        value=value[int(part)] if isinstance(value,list) else value[part]
    return value


def run_checks(folder,staged,specification,out):
    require(not out.exists(),'CLI evidence directory already exists; choose a fresh output')
    require(out.resolve().is_relative_to(QA.resolve()),'CLI output must stay in qa-output/development09')
    spec=read(specification)
    checks=spec.get('checks')
    require(isinstance(checks,list) and bool(checks),'Empty CLI check specification')
    names=set()
    for check in checks:
        require(re.fullmatch('[a-z0-9][a-z0-9-]*',check['name']) is not None
                and check['name'] not in names,'Invalid/duplicate CLI check name')
        names.add(check['name'])
        require(isinstance(check['arguments'],list)
                and all(isinstance(v,str) and '\x00' not in v for v in check['arguments']),
                'CLI arguments must be strings')
        require(type(check['expected_exit_code']) is int,'Each check must state expected exit code')
        require(bool(check.get('json_equals')) or bool(check.get('stdout_contains')),
                'Each check needs a positive output assertion; exit0 alone proves no diagnostic semantics')
        require(0<check.get('timeout_seconds',180)<=600,'Unsupported diagnostic timeout')
    out.mkdir(parents=True,exist_ok=False)
    (out/'work').mkdir()
    for directory in ['appdata','localappdata']:(out/directory).mkdir()
    env=os.environ.copy();env['APPDATA']=str((out/'appdata').resolve());env['LOCALAPPDATA']=str((out/'localappdata').resolve())
    rows=[]
    for check in checks:
        command=[str((folder/EXE_NAME).resolve()),*check['arguments']]
        failure=None;parsed=None;stdout='';stderr='';code=None
        try:
            process=subprocess.run(command,cwd=out/'work',env=env,capture_output=True,
                                   text=True,encoding='utf-8',errors='replace',shell=False,
                                   timeout=check.get('timeout_seconds',180))
            stdout,stderr,code=process.stdout,process.stderr,process.returncode
            require(code==check['expected_exit_code'],f'Expected exit{check["expected_exit_code"]}, got{code}')
            require('panicked at' not in stderr,'Client panic reported')
            for text in check.get('stdout_contains',[]):require(text in stdout,f'Missing stdout marker: {text}')
            if check.get('json_equals'):
                parsed=json.loads(stdout)
                for path,wanted in check['json_equals'].items():
                    require(at(parsed,path)==wanted,f'JSON assertion failed: {path}')
        except subprocess.TimeoutExpired as error:
            stdout=error.stdout or '';stderr=error.stderr or '';code=None
            if isinstance(stdout,bytes):stdout=stdout.decode('utf-8','replace')
            if isinstance(stderr,bytes):stderr=stderr.decode('utf-8','replace')
            failure=f'Timeout after{check.get("timeout_seconds",180)}seconds'
        except Exception as error:
            failure=str(error)
        stdout_path=out/(check['name']+'.stdout.txt');stderr_path=out/(check['name']+'.stderr.txt')
        stdout_path.write_text(stdout,encoding='utf-8');stderr_path.write_text(stderr,encoding='utf-8')
        rows.append({'name':check['name'],'arguments':check['arguments'],'exit_code':code,
                     'passed':failure is None,'failure':failure,
                     'stdout':stdout_path.relative_to(ROOT).as_posix(),'stdout_sha256':sha(stdout_path),
                     'stderr':stderr_path.relative_to(ROOT).as_posix(),'stderr_sha256':sha(stderr_path)})
    require(sha(folder/EXE_NAME)==staged['exe_sha256'],'Executable changed during CLI checks')
    report={'passed':all(row['passed'] for row in rows),'executable_sha256':staged['exe_sha256'],
            'scope':'Specified CLI diagnostic behavior only; not battle, Init/PostCreate, publication or multiplayer acceptance',
            'profile_scope':'Isolated APPDATA/LOCALAPPDATA inside QA; original profiles untouched',
            'specification_sha256':sha(specification),'checks':rows}
    write_json(out/'report.json',report)
    return report


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase',choices=['staged','sealed'])
    parser.add_argument('--output',required=True,help='New QA integrity report path')
    parser.add_argument('--checks',help='Optional explicit JSON CLI specification')
    parser.add_argument('--checks-output',help='New QA directory for CLI logs and isolated profile')
    options=parser.parse_args()
    output=Path(options.output).resolve()
    require(output.is_relative_to(QA.resolve()) and not output.exists(),'Output must be a new Development09 QA file')
    staged=read(QA/'staged.json')
    result=check_staged(RELEASE,staged) if options.phase=='staged' else check_sealed(RELEASE,staged)
    verified_rows(ROOT,staged['preserved_files'])
    if options.phase=='sealed' and (QA/'package-report.json').exists():
        sealed=read(QA/'package-report.json')
        require(sealed['zip_sha256']==sha(RELEASE/ARCHIVE_NAME),'ZIP changed since package sealing')
        require(sealed['exe_sha256']==staged['exe_sha256'],'Sealed report references different executable')
    result.update(passed=True,executable_sha256=staged['exe_sha256'],previous_release_hashes_verified=True)
    if options.checks:
        require(options.checks_output,'--checks requires --checks-output')
        result['cli']=run_checks(RELEASE,staged,Path(options.checks).resolve(),Path(options.checks_output).resolve())
        result['passed']=result['cli']['passed']
    elif options.checks_output:
        raise RuntimeError('--checks-output requires --checks')
    output.parent.mkdir(parents=True,exist_ok=True)
    write_json(output,result)
    print(json.dumps(result))
    raise SystemExit(0 if result['passed'] else 1)


if __name__=='__main__':main()
