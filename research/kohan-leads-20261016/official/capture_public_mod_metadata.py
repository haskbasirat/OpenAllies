"""Reference-only public documentation capture; never downloads game binaries."""
import datetime
import hashlib
import json
from pathlib import Path
import urllib.request

ROOT = Path(r"E:\Games\Axis and Alies\reconstruction\references\kohan-leads-20261016\official")
COMMIT = "7fd8ebe9a838e634c9886c4778fb22c1178570ab"

for name, relative in (
    ("public-mod-lead-README.md", "README.md"),
    ("public-mod-lead-LICENSE", "LICENSE"),
):
    url = f"https://raw.githubusercontent.com/ibm2187/axis-and-allies-2004-remastered/{COMMIT}/{relative}"
    request = urllib.request.Request(url, headers={"User-Agent": "AxisAllies-reference-audit"})
    with urllib.request.urlopen(request, timeout=30) as response:
        payload = response.read()
        metadata = {
            "url": url,
            "final_url": response.geturl(),
            "content_type": response.headers.get("Content-Type"),
            "bytes": len(payload),
            "sha256": hashlib.sha256(payload).hexdigest(),
            "retrieved_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        }
    with (ROOT / name).open("xb") as output:
        output.write(payload)
    with (ROOT / (name + ".source.json")).open("x", encoding="utf-8") as output:
        json.dump(metadata, output, indent=2)
        output.write("\n")
    print(json.dumps({"file": name, **metadata}))
