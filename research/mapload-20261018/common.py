"""Task utilities; no build, game or release mutation at import."""
from pathlib import Path
import hashlib
import json

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
QA = ROOT / "qa-output/mapload-20261018"


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def record(path):
    path = Path(path).resolve()
    return dict(path=path.relative_to(ROOT).as_posix(), bytes=path.stat().st_size, sha256=sha(path))


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def write_new(path, data):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(data, stream, indent=2)
        stream.write("\n")


def sources():
    files = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
    files.extend(path for path in (ROOT / "crates").rglob("*") if path.is_file() and "target" not in path.relative_to(ROOT / "crates").parts)
    return [record(path) for path in sorted(files)]
