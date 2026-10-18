"""Select unmodified original-execution cases for bounded client diagnostics."""
from pathlib import Path
import hashlib
import json

ROOT = Path(__file__).resolve().parents[2]
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

for area, filename, field, indices in [
    ('orders', 'commands-native-oracle.json', 'payloads', [0, 130, 260, 270, 296, 308]),
    ('spatial', 'native-spatial-fixtures.json', 'fixtures', [179, 245, 250, 262, 267, 308]),
]:
    directory = ROOT / f'research/phase5-{area}'
    source = directory / filename
    native = json.loads(source.read_text())
    samples = []
    for index in indices:
        case = native[field][index]
        name = case.get('input', {}).get('name', f'command_payload_{index}')
        samples.append({'name': name, 'source_index': index, 'fixture': case})
    output = {
        'schema': 1,
        'component': area,
        'source': source.relative_to(ROOT).as_posix(),
        'source_sha256': sha(source),
        'source_field': field,
        'adapter_sha256': sha(directory / 'client_samples.rs'),
        'scope': 'Fixture-only callbacks and registry inputs; no live world or gameplay execution.',
        'samples': samples,
    }
    (directory / 'client-samples.json').write_text(json.dumps(output, indent=2) + '\n')
    print(area, len(samples), sha(directory / 'client-samples.json'))
