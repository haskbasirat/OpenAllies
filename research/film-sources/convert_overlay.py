"""Convert the art of a loose data folder (a profile's `data`, a mod) the way the
stock art is converted: the same NIF exporter (reconstruction/tools/convert_models.py)
and texture writer (reconstruction/tools/archive_textures.py), into its own folder,
which `aa_assets::AssetStore::set_art_overlay` consults before the stock art.

    python convert_overlay.py <data folder> <output folder>

The input is read-only. The output folder gets assets/converted/{models,textures}.
A texture a model names that the folder does not hold is the archive's: its URL is the
stock one, resolved by the store when the folder has no file of that name. Files the
overlay holds byte for byte like the stock data are converted too: telling them apart
would cost more than converting them. Run it once per data folder.

Under research/: no game data, paths or results are kept here.
"""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
import importlib.util
import json
from pathlib import Path
import sys
import time

GAME_ROOT = Path(__file__).resolve().parents[3]
TOOLS = GAME_ROOT / 'reconstruction' / 'tools'


def load_tool(name):
    spec = importlib.util.spec_from_file_location(name, TOOLS / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('data', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--only-used', action='store_true',
                        help='Reserved for a future filter; every model is converted now')
    args = parser.parse_args()
    data = args.data.resolve()
    output = args.output.resolve()
    sys.path.insert(0, str(TOOLS))
    sys.path.insert(0, str(TOOLS / 'convert_models_vendor'))
    models = load_tool('convert_models')
    stock = models.ORIGINAL
    models.ORIGINAL = data
    models.OUTPUT = output / 'assets' / 'converted' / 'models'
    converter = models.Converter()
    # The archive's textures stay reachable from a model of this folder.
    stock_files = {p.relative_to(stock).as_posix().lower(): p for p in stock.rglob('*') if p.is_file()}
    resolve_own = converter.resolve_texture

    def resolve_texture(model, filename):
        found = resolve_own(model, filename)
        if found:
            return found
        raw = models.string(filename).replace('\\', '/').lstrip('/')
        relative = model.parent.relative_to(data).as_posix().lower() + '/' + raw.lower()
        name = raw.rsplit('/', 1)[-1].lower()
        path = stock_files.get(relative) or stock_files.get(raw.lower())
        if path is None:
            for candidate, file in stock_files.items():
                if candidate.endswith('/' + name):
                    path = file
                    break
        if path is None:
            return None
        converter.missing_textures.discard(raw)
        return '/assets/converted/textures/' + path.relative_to(stock).with_suffix('.png').as_posix()

    converter.resolve_texture = resolve_texture
    paths = [p for p in converter.files.values() if p.suffix.lower() == '.nif']
    records = []
    failures = []
    started = time.perf_counter()
    for i, path in enumerate(paths):
        source = path.relative_to(data).as_posix()
        try:
            records.append(converter.convert(path))
        except Exception as exc:
            failures.append({'source': source, 'error': str(exc), 'type': type(exc).__name__})
        if i % 100 == 0:
            print(f'{i + 1}/{len(paths)} models, {len(failures)} unsupported, {time.perf_counter() - started:.0f} s', flush=True)
    by_path = {}
    by_name = {}
    for model in sorted(records, key=lambda m: m['source'].lower()):
        by_path[model['source'].lower()] = model['url']
        by_path[str(Path(model['source']).with_suffix('')).replace('\\', '/').lower()] = model['url']
        by_name.setdefault(model['name'].lower(), model['url'])
    manifest = {'format': 'aa-model-manifest-v1', 'modelCount': len(records), 'models': records,
                'byPath': by_path, 'byName': by_name, 'failures': failures,
                'missingTextures': sorted(converter.missing_textures)}
    models.OUTPUT.mkdir(parents=True, exist_ok=True)
    (models.OUTPUT / 'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    print(f'{len(records)} models, {len(failures)} unsupported, {len(converter.missing_textures)} textures not found', flush=True)

    textures = load_tool('archive_textures')
    textures.SOURCE = data
    textures.TARGET = output / 'assets' / 'converted' / 'textures'
    textures.ROOT = output
    images = [p for p in data.rglob('*') if p.suffix.lower() in ('.dds', '.tga')]
    with ThreadPoolExecutor(max_workers=6) as pool:
        done = list(pool.map(textures.convert, images))
    print(f"{sum('output' in r for r in done)} textures, {sum('error' in r for r in done)} failed", flush=True)


if __name__ == '__main__':
    main()
