#!/usr/bin/env python3
"""What must never reach the public repository. Run it before every push.

    python research/check_public.py                 every tracked file and every file `git add -A` would add
    python research/check_public.py --report F.md   the same, with a table: file, lines, first sample
    python research/check_public.py --history       every blob of every commit, all refs: path, kind and count
    python research/check_public.py --history --rev origin/main
    python research/check_public.py --skip list.txt leaves out the paths of list.txt

Exit status 1 when anything is found, 0 when the tree is clean.

Kinds of finding:

    listing    instruction listings of the original: address plus mnemonic, ("mov", "eax, ...") tables,
               instruction fields in JSON, three or more instruction-only lines in a row
    bytes      raw bytes: byte fields in JSON, hex or base64 longer than a digest, spaced hex dumps
    pseudo     decompiled code: type casts through undefined4 and friends, the decompiler's local names
    label      automatic labels of the decompiler, which name an address: say 0x4F3C40 instead
    tool       names of decompiler tools (kept here only as SHA-256 of the lower-case word)
    personal   the machine's user name and profile path, the game's folder, a drive path, and every word of
               target/public-denylist.txt, one per line (player names from films, email): that file is not
               tracked and this script must never hold such words itself
    asset      lines identical to the original's text assets, when <game root> is known (AA_GAME_ROOT, or
               the folder above this repository); files named like assets or dumps, binary files
    ignored    a tracked file that .gitignore excludes

What it leaves alone, deliberately: `asm!` blocks and harness stubs of our own code, an idiom of one to three
instructions quoted in a comment, short pins of 32 bytes or less that a research script compares with the
executable it is given, addresses and field offsets.
"""
import argparse
import hashlib
import os
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

MN = (
    r"mov|movzx|movsx|movsd|movss|movd|movq|movaps|movups|movapd|lea|push|pop|call|jmp|ja|jae|jb|jbe|je|jg|jge|jl|jle|"
    r"jne|jnz|jz|js|jns|jp|jnp|cmp|test|xor|or|and|add|sub|adc|sbb|imul|idiv|mul|div|neg|not|shl|shr|sar|rol|ror|inc|"
    r"dec|ret|retn|leave|nop|cdq|cwde|sete|setne|setz|setnz|setg|setl|fld|fst|fstp|fild|fist|fistp|fadd|faddp|fsub|"
    r"fsubp|fsubr|fsubrp|fmul|fmulp|fdiv|fdivp|fdivr|fdivrp|fcom|fcomp|fcompp|fucom|fucomp|fucompp|fnstsw|fnstcw|fldcw|"
    r"fxch|fabs|fchs|fsqrt|sahf|xchg|fldz|fld1|pshufd|psrlq|psllq|pxor|andpd|andnpd|orpd|addsd|subsd|mulsd|divsd|"
    r"cvtsi2sd|cvttsd2si|cvtdq2pd|unpcklpd|ucomisd|comisd|rep|repne|stosd|movsb|int3"
)
REG = r"eax|ebx|ecx|edx|esi|edi|ebp|esp|ax|bx|cx|dx|si|di|al|ah|bl|bh|cl|ch|dl|dh|st\(\d\)|st\d?|xmm\d|cs|ds|es|fs|gs"
SIZE = r"(?:xmm|d|q|t)?word ptr|byte ptr|[dqt]word\b"
# An address, then optionally a column of code bytes, then an instruction with machine operands.
LISTING_ADDR = re.compile(
    rf"(?<![\w.+])(?:0x)?[0-9a-fA-F]{{5,8}}h?(?::|\s)\s*(?:[0-9a-fA-F]{{2,}}\s+)?(?:{MN})\s+(?:{SIZE}|(?:{REG})\b(?!\s*[=+*/(-])|\[)",
    re.I,
)
# Address, a byte column, then a mnemonic with any operand.
LISTING_BYTES = re.compile(
    rf"(?<![\w.+])(?:0x)?[0-9a-fA-F]{{6,8}}[: \t]+[0-9a-fA-F]{{2}}(?:\s?[0-9a-fA-F]{{2}})*\s+(?:{MN})(?:\s*$|\s+(?:{SIZE}|(?:{REG})\b|\[|-?(?:0x)?[0-9a-f]+\b))"
)
LISTING_TUPLE = re.compile(rf"""\(\s*["'](?:{MN})["']\s*,\s*["'](?:{SIZE}|(?:{REG})\b|\[|0x[0-9a-f])""")
LISTING_JSON = re.compile(r'"(?:instruction|disassembly|disasm|mnemonic|op_str|listing)"\s*:\s*["\[]')
LISTING_OBJECTS = re.compile(r'"instructions"\s*:\s*\[\s*\{')
LISTING_TEXT = re.compile(rf'"text"\s*:\s*"(?:{MN})\b')
# An instruction alone on its line. A `{placeholder}` marks our own asm! block.
LISTING_LINE = re.compile(
    rf"""^\s*(?://[/!]?\s*|\#\s*|\*\s*|["'])?(?:{MN})\s+(?:(?:{SIZE})\s|(?:{REG})\s*(?:,|$|["'])|\[[a-z])[^{{]*$""",
    re.I,
)

HEX5 = re.compile(r"[0-9a-fA-F]{5}")
BYTES_FIELD = re.compile(r'"(?:bytes|\w*_bytes|bytes_\w*|raw_hex|code_hex)"\s*:\s*"(?:[0-9a-fA-F]{2} ?)+"')
# Longer than a digest, even with a word's hex-looking last letters run into it.
BYTES_HEX = re.compile(r"(?<![0-9a-fA-F])[0-9a-fA-F]{72,}(?![0-9a-fA-F])")
BYTES_SPACED = re.compile(r"(?<![0-9a-fA-F])(?:[0-9a-fA-F]{2} ){16,}[0-9a-fA-F]{2}(?![0-9a-fA-F])")
BYTES_B64 = re.compile(r"(?<![A-Za-z0-9+/])[A-Za-z0-9+/]{200,}={0,2}")

PSEUDO = re.compile(
    r"\bundefined[1248]\b|\b(?:i|u|p|pu|pv|pi|pp|ppv|pc|pf|ps|b|c|s|f|d|l)Var[0-9]+\b|\bin_E[A-Z]{2}\b|\bunaff_[A-Z]+\b|"
    r"\bextraout_[A-Z]+\b|\b[a-z]{1,3}Stack_[0-9a-f]+\b|\bCONCAT[0-9]{2}\b|\bSUB[0-9]{2}\(|\b[0-9a-f]{8}\.c:[0-9]+:|\*\((?:undefined[0-9]?|code) ?\*+\)"
)
LABEL = re.compile(r"\b(?:thunk_)?(?:FUN|DAT|LAB|PTR_FUN|PTR_DAT|PTR_LAB|UNK|switchD|caseD)_(?:[0-9a-fA-F]{5,8}|[xX]{4,})\b|\bparam_[0-9]+\b")

# SHA-256 of the lower-case word. A word of a file is compared after splitting at anything not a letter or digit.
TOOL_WORDS = {
    "59a2ea36c20b30a30f3aa251b346188d5afbeddfca2d496e67b0c2d9eb74a345",
    "c67dae9a01b5e9e1c27ebc42873d4dd0d81096a708ff1ec4f9b8052206a94a2f",
    "4c2b87d25d1de24de188c214e441d98888e82afd2af9c55fb72dbc4db57ae204",
    "bb6b24bf13999272b27006f7711965f77861a326a30b43b9b1852a500ad68ac7",
    "0de0b9f8e822f9e8f1fc32a241757e4e3bed941f03c71eca5d8cd5df4e7a3869",
    "c54e87cc84b5a9c94843dee839d9f01967b80d2f5512dcb2b260be3685f5eb77",
    "18780d475772839659283d409bad25fcf712643dac68f79dd34ac4747975e599",
    "893a28f1f27ecaef8d64f468a6036f087c877daf12226612a23e29b1fc9bf1b1",
    "c4599aab5cbf87f8189071b86ce3c16a7dc83fd6a365466c4374939ef6b04812",
    "37cb07f657ec37693c53614061ae86e845efb9f72ea574b4fffff1d646d87758",
}
# The same for two words in a row: the first word alone is an ordinary word.
TOOL_FIRST = {"128df13c1e54ffaaafcc9d07ec7427d61f764214e6ae0321de23c94d261d0860", "9a3a45d01531a20e89ac6ae10b0b0beb0492acd7216a368aa062d1a5fecaf9cd"}
TOOL_PAIRS = {"000adf46232404584255ec3328a6581947fbe8cdcc1653559f114ea0d1d94383", "619aba39c3762e080f5826243f2f5eee0702011fa77621794a596d3acc470846"}

DRIVE = re.compile(r"(?<![A-Za-z0-9_])[A-Za-z]:[\\/]{1,2}[^\s\"'`|*?<>]*")
POSIX_DRIVE = re.compile(r"(?<![A-Za-z0-9_.~/-])/(?:mnt/)?[a-z]/(?:Users|Games|Program Files|tmp|Temp)\b", re.I)
PROFILE_WORDS = re.compile(r"\bAppData\b|\bDocuments[\\/]+My\b")
# Drive paths that name no one and nothing: test inputs and system folders.
HARMLESS_DRIVE = re.compile(
    r"^[A-Za-z]:[\\/]{1,2}(?:Windows\b|Program\b|model\.|bad\.|caller\b|out\b|secret\b|definitely\b|Users[\\/]{1,2}x\b|Games[\\/]{1,2}AA\b|AA\b|Docs\b|Films\b|Rewrite\b|model-|winter\b|[^\\/]*model\.)",
    re.I,
)
ASSET_NAME = re.compile(
    r"(?:^|/)(?:disasm|asm|pseudocode|chunks)/|\.(?:rmp|rsg|rfm|nif|kf|kfm|dds|tga|wav|bik|rwd|tgi|exe|dll|bin)(?:\.txt)?$|"
    r"(?:^|/)native-autoexec-source\.txt$|(?:^|/)[^/]*(?:\basm\b|disasm|instructions|pseudo)[^/]*\.txt$",
    re.I,
)
ALLOWED_BINARY = re.compile(r"^docs/screenshots/[^/]+\.jpg$")
TEXT_ASSET_SUFFIXES = {".tgi", ".txt", ".lst", ".cfg", ".set", ".red"}
# Our own text that is the same as a line of the original's files by nature: names of rule fields in fixtures.
ASSET_MIN_LINE = 25


def git(*args, input=None):
    return subprocess.run(("git",) + args, cwd=REPO, capture_output=True, input=input, check=True).stdout


def tracked_and_addable():
    names = set(git("ls-files", "-z").decode("utf-8").split("\0"))
    names |= set(git("ls-files", "-z", "-o", "--exclude-standard").decode("utf-8").split("\0"))
    names.discard("")
    return sorted(names)


def game_root():
    value = os.environ.get("AA_GAME_ROOT")
    return Path(value) if value else REPO.parent


def personal_words():
    """Words that identify this machine's owner, from the machine itself: nothing is stored in this file."""
    words = set()
    for key in ("USERNAME", "USER"):
        if os.environ.get(key):
            words.add(os.environ[key].lower())
    profile = os.environ.get("USERPROFILE") or os.environ.get("HOME")
    if profile:
        words.add(Path(profile).name.lower())
    try:
        email = git("config", "user.email").decode().strip().lower()
        if email and "noreply" not in email:
            words.add(email)
    except subprocess.CalledProcessError:
        pass
    denylist = REPO / "target" / "public-denylist.txt"
    if denylist.is_file():
        for line in denylist.read_text(encoding="utf-8").splitlines():
            line = line.strip().lower()
            if line and not line.startswith("#"):
                words.add(line)
    return {w for w in words if w}


def personal_phrases():
    """The folder the game lives in, by its name here: for example a misspelt name nobody else has."""
    phrases = set()
    for path in (game_root(), REPO.parent):
        if path.name:
            phrases.add(path.name.lower())
    return phrases


def asset_lines():
    data = game_root() / "reconstruction" / "assets" / "original" / "Data"
    lines = set()
    if not data.is_dir():
        return lines
    for folder, _, files in os.walk(data):
        for name in files:
            if os.path.splitext(name)[1].lower() not in TEXT_ASSET_SUFFIXES:
                continue
            raw = Path(folder, name).read_bytes()
            if raw[:2] in (b"\xff\xfe", b"\xfe\xff"):
                text = raw.decode("utf-16", "replace")
            elif raw.count(b"\0") > len(raw) // 4:
                text = raw.decode("utf-16-le", "replace")
            else:
                text = raw.decode("latin-1")
            for line in text.splitlines():
                line = line.strip()
                if len(line) >= ASSET_MIN_LINE:
                    lines.add(line)
    return lines


class Scanner:
    def __init__(self):
        self.personal = personal_words()
        self.phrases = personal_phrases()
        self.assets = asset_lines()
        self.digests = {}

    def sha(self, word):
        digest = self.digests.get(word)
        if digest is None:
            digest = hashlib.sha256(word.encode()).hexdigest()
            self.digests[word] = digest
        return digest

    def tool_word(self, word):
        return self.sha(word) in TOOL_WORDS

    def scan(self, path, raw):
        """{kind: [line count, first sample]} of one file's bytes."""
        found = {}

        def hit(kind, number, line, count=1):
            row = found.setdefault(kind, [0, f"{number}: {line.strip()[:170]}"])
            row[0] += count

        if path.replace("\\", "/").endswith(("check_public.py",)):
            return found
        if ASSET_NAME.search(path):
            hit("asset", 0, "file name")
        if b"\0" in raw[:8000]:
            if not ALLOWED_BINARY.match(path):
                hit("asset", 0, "binary file")
            return found
        if path.endswith(("Cargo.lock", ".jpg")):
            return found
        text = raw.decode("utf-8", "replace")
        is_data = path.endswith((".json", ".txt", ".md"))
        if is_data:
            for match in LISTING_OBJECTS.finditer(text):
                hit("listing", text.count("\n", 0, match.start()) + 1, match.group(0).split("[")[0])
        run = []
        addressed = []
        identical = 0
        identical_sample = None
        checked = 0
        for number, line in enumerate(text.split("\n"), 1):
            stripped = line.strip()
            if not stripped:
                continue
            checked += 1
            if LISTING_LINE.search(line):
                run.append((number, line))
            else:
                if len(run) >= 3:
                    for n, l in run:
                        hit("listing", n, l)
                run = []
            has_address = HEX5.search(line) is not None
            if (has_address and LISTING_ADDR.search(line)) or LISTING_TUPLE.search(line):
                addressed.append((number, line))
            if (has_address and LISTING_BYTES.search(line)) or (is_data and (LISTING_JSON.search(line) or LISTING_TEXT.search(line))):
                hit("listing", number, line)
            if (is_data and BYTES_FIELD.search(line)) or BYTES_HEX.search(line) or BYTES_SPACED.search(line) or BYTES_B64.search(line):
                hit("bytes", number, line)
            if PSEUDO.search(line):
                hit("pseudo", number, line)
            if LABEL.search(line):
                hit("label", number, line)
            lowered = line.lower()
            words = re.findall(r"[a-z0-9]+", lowered)
            for at, word in enumerate(words):
                if self.tool_word(word) or (
                    at + 1 < len(words) and self.sha(word) in TOOL_FIRST and self.sha(f"{word} {words[at + 1]}") in TOOL_PAIRS
                ):
                    hit("tool", number, line)
                    break
            for phrase in self.phrases:
                if phrase in lowered:
                    hit("personal", number, line)
                    break
            else:
                personal = False
                for match in DRIVE.finditer(line):
                    if not HARMLESS_DRIVE.match(match.group(0)):
                        personal = True
                if not personal and (POSIX_DRIVE.search(line) or PROFILE_WORDS.search(line)):
                    personal = True
                if not personal:
                    if (set(re.findall(r"[a-z0-9@._-]+", lowered)) | set(words)) & self.personal:
                        personal = True
                if personal:
                    hit("personal", number, line)
            if stripped in self.assets:
                identical += 1
                identical_sample = identical_sample or (number, line)
        if len(run) >= 3:
            for n, l in run:
                hit("listing", n, l)
        # One or two addressed instructions are a comment's idiom, more is a listing.
        if len(addressed) >= 3:
            for n, l in addressed:
                hit("listing", n, l)
        if identical_sample and (identical >= 30 or (identical >= 5 and identical * 3 >= checked)):
            hit("asset", identical_sample[0], identical_sample[1], identical)
        return found


def scan_tree(scanner, skip=()):
    rows = {}
    for name in tracked_and_addable():
        path = REPO / name
        if name in skip or not path.is_file():
            continue
        found = scanner.scan(name, path.read_bytes())
        if found:
            rows[name] = found
    # A rule of .gitignore does not stop a file that is already in the index: `git rm --cached` does.
    for name in git("ls-files", "-z", "-i", "-c", "--exclude-standard").decode("utf-8").split("\0"):
        if name and name not in skip:
            rows.setdefault(name, {})["ignored"] = [1, "0: tracked although .gitignore excludes it"]
    return rows


def scan_history(scanner, revs):
    """Every path and blob ever reachable. A path counts if any version of it has a finding."""
    objects = git("rev-list", "--objects", *revs).decode("utf-8", "replace").splitlines()
    pairs = {}
    for line in objects:
        parts = line.split(" ", 1)
        if len(parts) == 2 and parts[1]:
            pairs.setdefault(parts[0], set()).add(parts[1])
    process = subprocess.Popen(("git", "cat-file", "--batch"), cwd=REPO, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    rows = {}

    def feed():
        for sha in pairs:
            process.stdin.write(sha.encode() + b"\n")
        process.stdin.close()

    import threading

    threading.Thread(target=feed, daemon=True).start()
    for sha, names in pairs.items():
        header = process.stdout.readline().split()
        size = int(header[2])
        raw = process.stdout.read(size)
        process.stdout.read(1)
        if header[1] != b"blob":
            continue
        for name in names:
            found = scanner.scan(name, raw)
            for kind, (count, sample) in found.items():
                row = rows.setdefault(name, {}).setdefault(kind, [0, sample])
                row[0] = max(row[0], count)
    process.wait()
    return rows


def write_table(rows, out):
    kinds = {}
    for found in rows.values():
        for kind, (count, _) in found.items():
            files, lines = kinds.get(kind, (0, 0))
            kinds[kind] = (files + 1, lines + count)
    out.write(f"{len(rows)} files with findings\n\n")
    for kind, (files, lines) in sorted(kinds.items()):
        out.write(f"- {kind}: {files} files, {lines} lines\n")
    for kind in sorted(kinds):
        out.write(f"\n## {kind}\n\n")
        for name, found in sorted(rows.items(), key=lambda kv: -kv[1].get(kind, [0])[0]):
            if kind in found:
                count, sample = found[kind]
                out.write(f"- `{name}`: {count} | {sample}\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--report", help="write the whole table to this file")
    parser.add_argument("--history", action="store_true", help="scan old commits instead of the tree")
    parser.add_argument("--rev", action="append", help="revisions for --history (default: all refs)")
    parser.add_argument("--paths", help="write path<TAB>kinds to this file")
    parser.add_argument("--skip", help="file of paths to leave out, one per line: those about to leave the index")
    args = parser.parse_args()
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except AttributeError:
        pass
    scanner = Scanner()
    if not (REPO / "target" / "public-denylist.txt").is_file():
        print("note: target/public-denylist.txt is missing, so no player names were checked")
    if not scanner.assets:
        print("note: the original's text assets were not found under <game root>, so copies of them were not checked")
    skip = set(Path(args.skip).read_text(encoding="utf-8").split("\n")) if args.skip else set()
    rows = scan_history(scanner, args.rev or ["--all"]) if args.history else scan_tree(scanner, skip)
    if args.report:
        with open(args.report, "w", encoding="utf-8", newline="\n") as out:
            write_table(rows, out)
    if args.paths:
        with open(args.paths, "w", encoding="utf-8", newline="\n") as out:
            for name in sorted(rows):
                out.write(f"{name}\t{','.join(sorted(rows[name]))}\n")
    write_table(rows, sys.stdout)
    return 1 if rows else 0


if __name__ == "__main__":
    sys.exit(main())
