"""Read-only installed-AA comparison; original audit code, not upstream execution.

Only metadata and one already recovered small member are decoded. Archives are
opened rb. The only output is this directory's crosscheck.json. The existing
Rust reader is invoked in list mode, never extract or pack mode.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import struct
import subprocess
from datetime import datetime, timezone

HERE = Path(__file__).resolve().parent
WORKSPACE = HERE.parents[2]
GAME = WORKSPACE.parent
SOURCE = GAME / "reconstruction/references/kohan-leads-20261016/rwd"
RUST = WORKSPACE / "target/s-graphics-reference/release/rwd.exe"
EXPECTED = {
    "Data.rwd": 13428, "Music.rwd": 34, "Speech.rwd": 100,
    "Video.rwd": 42, "Video_En.rwd": 10,
    "skins/GBR.rwd": 226, "skins/GER.rwd": 220,
    "skins/JPN.rwd": 233, "skins/RUS.rwd": 229, "skins/USA.rwd": 226,
}


def sha256(path: Path) -> str:
    with path.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_exact(f, size: int) -> bytes:
    b = f.read(size)
    assert len(b) == size
    return b


def inspect(name: str, expected_count: int) -> dict:
    path = GAME / name
    before = sha256(path)
    size = path.stat().st_size
    with path.open("rb") as f:
        header = read_exact(f, 12)
        magic, version, chunk_count = struct.unpack("<4sII", header)
        assert (magic, version, chunk_count) == (b"TGCK", 2, 3)
        table_offset = size - chunk_count * 96
        f.seek(table_offset)
        raw_table = read_exact(f, chunk_count * 96)
        chunks = []
        for p in range(0, len(raw_table), 96):
            raw = raw_table[p:p + 96]
            chunk_name = raw[:64].decode("utf-16le").split("\0", 1)[0]
            offset, logical, opaque, stored = struct.unpack_from("<QQQQ", raw, 64)
            assert logical == stored and 12 <= offset <= offset + stored <= table_offset
            chunks.append(dict(name=chunk_name, offset=offset, size=logical,
                               opaque_u64=opaque, stored_size=stored))
        assert [c["name"] for c in chunks] == ["Header", "Files", "Footer"]
        by_name = {c["name"]: c for c in chunks}
        files, footer = by_name["Files"], by_name["Footer"]
        assert footer["offset"] + footer["size"] == table_offset
        f.seek(footer["offset"])
        raw_footer = read_exact(f, footer["size"])
        count, = struct.unpack_from("<I", raw_footer)
        assert count == expected_count
        rows, record_starts, raw_names = [], [], []
        at = 4
        for _ in range(count):
            record_starts.append(at)
            units, = struct.unpack_from("<H", raw_footer, at)
            at += 2
            raw_name = raw_footer[at:at + 2 * units]
            raw_names.append(raw_name)
            member_name = raw_name.decode("utf-16le")
            at += 2 * units
            relative, length, flags, index = struct.unpack_from("<QQII", raw_footer, at)
            at += 24
            assert flags == 0 and relative + length <= files["size"]
            rows.append([index, length, files["offset"] + relative,
                         member_name.replace("\\", "/")])
        assert at == len(raw_footer)

        # Explain the public source's four-byte shift using byte positions,
        # without running or embedding any upstream implementation.
        leading_words = [struct.unpack_from("<I", raw_footer, p - 4)[0]
                         for p in record_starts]
        assert leading_words == [count] + [row[0] for row in rows[:-1]]
        final_word, = struct.unpack_from("<I", raw_footer, len(raw_footer) - 4)
        assert final_word == rows[-1][0]
        f.seek(size - 292)
        assert read_exact(f, 4) == struct.pack("<I", final_word)
        f.seek(0)
        intro = read_exact(f, 18)
        description_units, = struct.unpack_from("<H", intro, 16)
        intro_end = 18 + description_units * 2 + 8
        assert intro_end == files["offset"]

        payload = None
        if name == "Data.rwd":
            row = next(r for r in rows if r[3] == "AVars_version.tgi")
            assert row[0] == 3 and row[2] == 22419
            f.seek(row[2])
            actual = read_exact(f, row[1])
            fixture = WORKSPACE / "crates/aa-formats/tests/fixtures/avars-version.original.tgi"
            assert actual == fixture.read_bytes()
            payload = dict(path=row[3], source_index=row[0], absolute_offset=row[2],
                           bytes=len(actual), sha256=digest(actual),
                           equals_recovered_fixture=True)

    run = subprocess.run([str(RUST), "list", str(path)], check=True,
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    lines = run.stdout.decode("utf-8").splitlines()
    assert lines[0] == "source_index\tsize\tabsolute_offset\tpath"
    rust_rows = []
    for line in lines[1:]:
        index, length, offset, member_path = line.split("\t", 3)
        rust_rows.append([int(index), int(length), int(offset), member_path])
    assert rust_rows == rows
    canonical = json.dumps(rows, ensure_ascii=False, separators=(",", ":")).encode()
    after = sha256(path)
    assert before == after
    return dict(path=name, bytes=size, sha256_before=before, sha256_after=after,
                unchanged=True, header_sha256=digest(header), version=version,
                chunk_count=chunk_count, chunk_table_offset=table_offset,
                chunk_table_sha256=digest(raw_table), chunks=chunks,
                footer_sha256=digest(raw_footer), member_count=count,
                footer_consumed_exactly=True, all_member_flags_zero=True,
                all_member_extents_inside_files=True,
                rust_cli_all_rows_equal=True, canonical_rows_sha256=digest(canonical),
                canonical_rows_columns=["source_index", "size", "absolute_offset", "path"],
                public_typeid_first_word_is_member_count=leading_words[0],
                public_typeid_remaining_words_are_prior_source_indices=True,
                public_metadata_padding_is_final_source_index=final_word,
                public_intro_end_equals_files_offset=intro_end,
                small_payload=payload)


def main() -> None:
    results = [inspect(name, count) for name, count in EXPECTED.items()]
    pinned = []
    for p in [RUST, WORKSPACE / "crates/aa-formats/src/rwd.rs",
              WORKSPACE / "crates/aa-formats/src/bin/rwd.rs",
              WORKSPACE / "crates/aa-formats/tests/rwd.rs",
              SOURCE / "rwd_edit.cpp", SOURCE / "README.md", SOURCE / "LICENSE"]:
        pinned.append(dict(path=str(p), bytes=p.stat().st_size, sha256=sha256(p)))
    out = dict(schema=1, generated_utc=datetime.now(timezone.utc).isoformat(),
               audit="read-only installed Axis & Allies TGCK metadata crosscheck",
               scope="10 archive metadata tables, all member rows, one existing small payload fixture; no upstream code execution",
               archive_count=len(results), member_count=sum(r["member_count"] for r in results),
               all_archives_unchanged=True, all_rust_cli_rows_equal=True,
               pins=pinned, archives=results)
    (HERE / "crosscheck.json").write_text(json.dumps(out, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({k: out[k] for k in ("archive_count", "member_count", "all_archives_unchanged", "all_rust_cli_rows_equal")}))


if __name__ == "__main__":
    main()
