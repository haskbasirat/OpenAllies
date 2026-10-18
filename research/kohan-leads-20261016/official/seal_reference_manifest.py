"""Verify reference payload provenance and write non-cyclic local indexes."""
import datetime
import hashlib
import json
from pathlib import Path

LOCAL = Path(__file__).resolve().parent
RAW = Path(r"E:\Games\Axis and Alies\reconstruction\references\kohan-leads-20261016\official")


def digest(path):
    payload = path.read_bytes()
    return {"bytes": len(payload), "sha256": hashlib.sha256(payload).hexdigest()}


def write_new(path, value):
    with path.open("x", encoding="utf-8", newline="\n") as output:
        json.dump(value, output, indent=2, ensure_ascii=True)
        output.write("\n")


downloads = []
for sidecar in sorted(RAW.glob("*.source.json")):
    meta = json.loads(sidecar.read_text(encoding="utf-8"))
    payload = RAW / sidecar.name.removesuffix(".source.json")
    actual = digest(payload)
    assert actual["bytes"] == meta["bytes"], payload
    assert actual["sha256"] == meta["sha256"], payload
    downloads.append({
        "file": payload.name,
        **meta,
        "source_metadata_file": sidecar.name,
        "source_metadata_integrity": digest(sidecar),
        "classification": "exploratory_redirect_not_exact_post" if payload.name == "kohan2-announcement.html" else "reference_only",
    })
events = json.loads((RAW / "steam-97130-events.json").read_text(encoding="utf-8"))
event = next(e for e in events["events"] if e["gid"] == "672876020537229477")
body = event["announcement_body"]
assert event["appid"] == 97130
assert body["gid"] == "672876020537229478"
assert body["posttime"] == 1785285026
assert "Rebuilt from the original source code" in body["body"]
assert "Steamworks" in body["body"] and "125%" in body["body"] and "250%" in body["body"]
findings = {
    "schema": "kohan_official_source_lead_v1",
    "research_date_local": "2026-10-16 America/New_York",
    "retrieval_note": "Raw captures occurred 2026-10-17 UTC, still 2026-10-16 EDT.",
    "game": "Kohan II: Kings of War",
    "appid": 97130,
    "event_gid": event["gid"],
    "announcement_gid": body["gid"],
    "title": event["event_name"],
    "post_unix": body["posttime"],
    "post_utc": "2026-07-29T00:30:26Z",
    "post_edt": "2026-07-28T20:30:26-04:00",
    "canonical_event_url": "https://store.steampowered.com/news/app/97130/view/672876020537229477",
    "canonical_announcement_url": "https://steamcommunity.com/games/97130/announcements/detail/672876020537229478",
    "source_claim_quote": "Rebuilt from the original source code",
    "claim_class": "Official developer publication; not an independent runtime test or a public-source license.",
    "steam_developers": ["Lootmax Games", "TimeGate Studios"],
    "steam_publisher": "Big Bad Wolf, Inc.",
    "official_product_website": "https://www.lootmaxgames.com/",
    "public_original_game_source_or_sdk_located": False,
    "search_absence_is_exhaustive_proof": False,
    "aa_source_reuse_permission_verified": False,
    "cross_game_compatibility_verified": False,
    "separate_publisher_website_verified": False,
    "public_fan_patch_lead": {
        "url": "https://github.com/ibm2187/axis-and-allies-2004-remastered",
        "commit": "7fd8ebe9a838e634c9886c4778fb22c1178570ab",
        "classification": "Unofficial patch/installer project requiring original game; not original TimeGate source.",
        "code_documentation_license": "MIT; original game/art/middleware rights excluded from this inference.",
        "downloaded_scope": ["commit metadata", "README", "LICENSE"],
        "runtime_or_technical_claims_tested": False,
    },
    "searches": [
        'Kohan II Lootmax July 2026 original source code',
        'site:lootmaxgames.com source SDK github',
        'Kohan II source SDK TimeGate',
        'Axis & Allies 2004 TimeGate original source SDK official',
        'GitHub repositories: kohan timegate',
        'GitHub repositories: "lootmax games"',
        'GitHub repositories: axis allies rts',
    ],
    "mutations": {"production": False, "releases": False, "accounts": False, "external_messages": False, "compiled_game_downloads": False},
    "unsent_request": "UNSENT-SOURCE-REQUEST.txt",
    "announcement_api_evidence": {"file": "steam-97130-events.json", **digest(RAW / "steam-97130-events.json")},
}
write_new(LOCAL / "findings.json", findings)
write_new(RAW / "download-manifest.json", {
    "schema": "reference_only_downloads_v1",
    "created_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "payload_count": len(downloads),
    "payload_bytes": sum(x["bytes"] for x in downloads),
    "downloads": downloads,
    "local_notice": {"file": "REFERENCE-NOTICE.txt", **digest(RAW / "REFERENCE-NOTICE.txt")},
    "self_hash_omitted_to_avoid_cycle": True,
})
local_files = [p for p in sorted(LOCAL.iterdir()) if p.is_file() and p.name != "reference-manifest.json"]
raw_manifest = RAW / "download-manifest.json"
write_new(LOCAL / "reference-manifest.json", {
    "schema": "kohan_official_research_index_v1",
    "raw_root": str(RAW),
    "download_manifest": {"file": str(raw_manifest), **digest(raw_manifest)},
    "research_files": [{"file": p.name, **digest(p)} for p in local_files],
    "source_payload_count": len(downloads),
    "payload_and_sidecar_integrity_checks_passed": True,
    "self_hash_omitted_to_avoid_cycle": True,
})
print(json.dumps({"payloads_verified": len(downloads), "raw_manifest": digest(raw_manifest), "research_manifest": digest(LOCAL / "reference-manifest.json"), "findings": digest(LOCAL / "findings.json")}))
