"""Final Development-06 executable gate; no GUI or graphics capture.

Prepare/build the candidate first, then explicitly run:
  python research/check_development_06_exe.py --exe <candidate.exe> --output <new directory>
Every candidate invocation uses E:/Games, independently of this script's CWD.
The output directory must not exist. This checks bounded diagnostics and source
map input decoding, not a completed playable game or network interoperability.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
GAME = ROOT.parent
ASSETS = GAME / 'reconstruction'
MAPS = ASSETS / 'assets/original/Data/Maps'
UNRELATED_CWD = GAME.parent  #E:/Games, outside both the game and reconstruction.
EXPECTED = {
    'maps': 80,
    'placements': 225851,
    'owner_references': 11701,
    'resolved_non_null_owners': 7512,
    'null_owners': 4189,
    'present_kingdoms': 328,
    'present_teams': 21,
}


def sha(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def embedded(result: dict[str, Any], expected: int) -> dict[str, Any]:
    require(result['passed'] is True, 'embedded diagnostic reported failure')
    require(result['check_count'] == expected, f'expected {expected} embedded cases')
    require(result['passed_checks'] == expected, 'not all embedded cases passed')
    checks = result['checks']
    require(len(checks) == expected and all(c['passed'] is True for c in checks),
            'embedded per-case results disagree with totals')
    return {'check_count': expected, 'passed_checks': expected,
            'sample_bundle_sha256': result['sample_bundle_sha256']}


def catalog(result: dict[str, Any]) -> dict[str, Any]:
    # Rust canonicalize uses the Win32 extended-length prefix. Compare actual
    # filesystem identity instead of treating that spelling as another folder.
    require(Path(result['asset_root']).samefile(ASSETS), 'wrong actual asset root')
    require(result['maps'] == EXPECTED['maps'], 'asset catalog map count changed')
    require(result['placements'] == EXPECTED['placements'], 'asset placement count changed')
    require(result['model_catalog'] > 0, 'empty model catalog')
    return {k: result[k] for k in ('maps', 'placements', 'model_catalog', 'status')}


def actor(result: dict[str, Any]) -> dict[str, Any]:
    require(result['actor']['id'] == 'usa_infantry', 'wrong authored actor')
    require(result['definitions'] > 0 and result['source_files'] > 0, 'empty authored source catalog')
    require(len(result['actor']['fields']) > 0, 'authored actor has no decoded fields')
    return {'id': result['actor']['id'], 'definitions': result['definitions'],
            'source_files': result['source_files'], 'source_notices': result['source_notices'],
            'field_count': len(result['actor']['fields'])}


def economy(result: dict[str, Any]) -> dict[str, Any]:
    require(result['provider_limit'] == 100, 'modified native capacity limit changed')
    require(result['shortage_money_per_minute'] == 7.0, 'shortage arithmetic differs')
    require(result['money_after_60_seconds'] == 1093.0, 'stock flow arithmetic differs')
    rows = {r['id']: r for r in result['rows']}
    for name in ('unit_limit_provided', 'building_limit_provided'):
        require(rows[name]['effective_provided'] == 100.0, f'{name} clamp differs')
    return {k: result[k] for k in ('provider_limit', 'shortage_money_per_minute',
                                 'money_after_60_seconds', 'shortage_helper_calls')}


def map_world(result: dict[str, Any], path: Path) -> dict[str, Any]:
    require(result['passed'] is True, 'map input check reported failure')
    require(result['map_source']['sha256'] == sha(path), 'map source identity differs')
    for field in ('header_error', 'teams_error', 'kingdoms_error'):
        require(result[field] is None, f'{field}: {result[field]}')
    for field in ('generation_mismatches', 'team_generation_mismatches',
                  'kingdom_generation_mismatches', 'unsupported_records'):
        require(result[field] == [], f'{field}: {result[field]}')
    for field in ('header', 'serialized_teams', 'serialized_kingdoms', 'ownership'):
        require(isinstance(result[field], dict), f'{field} not decoded')
    layers = result['height_layers']
    require(len(layers) == 2 and {r['name'] for r in layers} == {'HEIGHT_MAP', 'MOUNTAIN_HEIGHT_MAP'},
            'map must decode both original height layers')
    owners = result['ownership']
    require(owners['unresolved_non_null_count'] == 0 and owners['unresolved_records'] == [],
            'unresolved saved owner references')
    require(owners['resolved_non_null_count'] == owners['non_null_count'], 'owner resolution totals disagree')
    require(owners['reference_count'] == owners['null_count'] + owners['non_null_count'],
            'owner reference totals disagree')
    require(sum(result['classes'].values()) == result['placements'], 'placement class totals disagree')
    return {
        'map': path.relative_to(MAPS).as_posix(), 'source_sha256': result['map_source']['sha256'],
        'placements': result['placements'], 'height_layers': len(layers),
        'owner_references': owners['reference_count'],
        'resolved_non_null_owners': owners['resolved_non_null_count'], 'null_owners': owners['null_count'],
        'present_kingdoms': result['serialized_kingdoms']['present_count'],
        'present_teams': result['serialized_teams']['present_count'],
        'pending_section_count': len(result['pending_sections']),
        'absent_sections': result['absent_sections'],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--exe', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True, help='New directory for report and failure logs')
    args = parser.parse_args(argv)
    exe, output = args.exe.resolve(), args.output.resolve()
    require(exe.is_file(), f'candidate executable missing: {exe}')
    require(ASSETS.is_dir() and MAPS.is_dir(), 'original reconstruction assets missing')
    require(UNRELATED_CWD.is_dir() and UNRELATED_CWD != GAME, 'unrelated working directory missing')
    maps = sorted((p.resolve() for p in MAPS.rglob('*') if p.is_file() and p.suffix.lower() == '.rmp'),
                  key=lambda p: p.relative_to(MAPS).as_posix().casefold())
    require(len(maps) == EXPECTED['maps'], f'expected80 original maps, found{len(maps)}')
    output.mkdir(parents=True, exist_ok=False)
    absent = output / '__nonexistent_assets__'
    require(not absent.exists(), 'asset-free diagnostic path unexpectedly exists')
    started = time.monotonic()
    report: dict[str, Any] = {
        'schema': 1, 'passed': False, 'complete': False, 'executable': str(exe),
        'executable_sha256': sha(exe), 'script_sha256': sha(Path(__file__)),
        'cwd': str(UNRELATED_CWD), 'actual_assets': str(ASSETS),
        'nonexistent_assets': str(absent), 'expected': EXPECTED,
        'scope': 'Final executable bounded diagnostics and80 original map input checks; no GPU, live gameplay, save roundtrip or internet multiplayer acceptance.',
        'checks': [], 'maps': [], 'errors': [],
    }

    def save() -> None:
        report['seconds'] = round(time.monotonic() - started, 3)
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')

    def run(name: str, flags: list[str], validate: Any) -> dict[str, Any] | None:
        command = [str(exe), *flags]
        row: dict[str, Any] = {'name': name, 'command': command,
                               'command_line': subprocess.list2cmdline(command), 'passed': False}
        began = time.monotonic()
        stdout = stderr = ''
        try:
            process = subprocess.run(command, cwd=UNRELATED_CWD, capture_output=True,
                                     text=True, encoding='utf-8', errors='replace', timeout=120)
            row['exit_code'] = process.returncode
            stdout, stderr = process.stdout, process.stderr
            row['stdout_sha256'] = hashlib.sha256(stdout.encode()).hexdigest()
            row['stderr_sha256'] = hashlib.sha256(stderr.encode()).hexdigest()
            require(process.returncode == 0, f'exit code {process.returncode}')
            result = json.loads(stdout)
            row['summary'] = validate(result)
            row['passed'] = True
        except (OSError, subprocess.TimeoutExpired, ValueError, TypeError, KeyError) as error:
            row['error'] = f'{type(error).__name__}: {error}'
            report['errors'].append({'check': name, 'error': row['error']})
            stem = f'failure-{len(report["checks"]):03d}'
            (output / f'{stem}.stdout.txt').write_text(stdout, encoding='utf-8')
            (output / f'{stem}.stderr.txt').write_text(stderr, encoding='utf-8')
            row['failure_logs'] = [f'{stem}.stdout.txt', f'{stem}.stderr.txt']
        row['seconds'] = round(time.monotonic() - began, 3)
        report['checks'].append(row)
        save()
        return row.get('summary')

    save()
    try:
        for name, flags, validator in (
            ('world', ['--world-check', '--assets', str(absent)], lambda value: embedded(value, 24)),
            ('mechanics', ['--mechanics-check', '--assets', str(absent)], lambda value: embedded(value, 53)),
            ('verify-assets', ['--verify-assets', '--assets', str(ASSETS)], catalog),
            ('actor-usa_infantry', ['--actor-check', 'usa_infantry', '--assets', str(ASSETS)], actor),
            ('economy', ['--economy-check', '--assets', str(ASSETS)], economy),
        ):
            run(name, flags, validator)
            print(f'{name}: {report["checks"][-1]["passed"]}', flush=True)
        for index, path in enumerate(maps):
            summary = run(f'map-{path.relative_to(MAPS).as_posix()}',
                          ['--map-world-check', str(path), '--assets', str(absent)],
                          lambda value, path=path: map_world(value, path))
            if summary is not None:
                report['maps'].append(summary)
            if (index + 1) % 10 == 0:
                print(f'map inputs: {index + 1}/{len(maps)} checked', flush=True)
        totals = {'maps': len(report['maps'])}
        totals.update({key: sum(row[key] for row in report['maps']) for key in EXPECTED if key != 'maps'})
        report['totals'] = totals
        for key, expected in EXPECTED.items():
            if totals[key] != expected:
                report['errors'].append({'check': 'aggregate', 'error': f'{key}: expected{expected}, actual{totals[key]}'})
        report['executable_sha256_after'] = sha(exe)
        if report['executable_sha256_after'] != report['executable_sha256']:
            report['errors'].append({'check': 'identity', 'error': 'candidate executable changed during gate'})
        if absent.exists():
            report['errors'].append({'check': 'asset-independence', 'error': 'nonexistent asset path was created'})
        report['complete'] = True
        report['passed'] = not report['errors'] and len(report['checks']) == 85
    except KeyboardInterrupt:
        report['errors'].append({'check': 'interruption', 'error': 'gate interrupted before completion'})
    finally:
        save()
    print(json.dumps({'passed': report['passed'], 'complete': report['complete'],
                      'checks': len(report['checks']), 'totals': report.get('totals'),
                      'errors': report['errors'], 'report': str(output / 'report.json')}))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
