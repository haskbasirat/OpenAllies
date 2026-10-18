"""Fetch pinned public source/notice bytes only; no upstream program execution."""
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib
import json
import urllib.request

HERE = Path(__file__).resolve().parent
META = json.loads((HERE / "github-metadata.json").read_text())
SHA = META["commit"]["sha"]
DEST = HERE.parents[1].parent / "reconstruction/references/byroredux-20261016/source"
EXTRA = {"Cargo.toml", "Cargo.lock", "README.md", "THIRD_PARTY_NOTICES.md", "rust-toolchain.toml",
         "crates/nif/Cargo.toml", "crates/core/Cargo.toml", "docs/engine/nifal.md"}
SELECTED = [x for x in META["tree"]["tree"] if x["type"] == "blob" and
            (x["path"] in EXTRA or x["path"].startswith(("crates/nif/src/", "crates/core/src/")))]


def fetch(x):
    name = x["path"]
    url = f"https://raw.githubusercontent.com/matiaszanolli/ByroRedux/{SHA}/{name}"
    data = urllib.request.urlopen(url).read()
    assert len(data) == x["size"]
    git_id = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
    assert git_id == x["sha"]
    p = DEST / name
    p.parent.mkdir(parents=True, exist_ok=True)
    if p.exists():
        assert p.read_bytes() == data
    else:
        p.write_bytes(data)
    return dict(path=name, bytes=len(data), sha256=hashlib.sha256(data).hexdigest(),
                git_blob=x["sha"], url=url,
                github_url=f"https://github.com/matiaszanolli/ByroRedux/blob/{SHA}/{name}")


if __name__ == "__main__":
    with ThreadPoolExecutor(max_workers=12) as pool:
        files = list(pool.map(fetch, SELECTED))
    out = dict(schema=1, repository=META["repository"]["html_url"],
               branch_observed=META["repository"]["default_branch"], commit=SHA,
               commit_date=META["commit"]["commit"]["committer"]["date"],
               retrieved_utc=datetime.now(timezone.utc).isoformat(),
               license_api=META["repository"]["license"],
               license_declaration="Root Cargo.toml workspace.package license = MIT; README License section says MIT. No root LICENSE in the pinned complete tree.",
               scope="Selected nif/core Rust source and manifests, root provenance/notices, nifal doc only. No renderer, engine, vendored SDK, binary, asset, setup script, or upstream agent instructions downloaded.",
               count=len(files), files=files)
    (DEST.parent / "source-manifest.json").write_text(json.dumps(out, indent=2) + "\n")
    print(json.dumps(dict(commit=SHA, files=len(files), bytes=sum(x["bytes"] for x in files))))
