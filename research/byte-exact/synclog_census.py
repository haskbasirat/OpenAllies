"""Static census of every AA.exe call into the TGC_Synchronizer loggers.

Loggers (thiscall, ecx = synchronizer; A = [0xB3655C], S = [0xB36560]):
  0x77DA70 int, 0x77D930 float bits, 0x77D9D0 bool, 0x77DB10 text, 0x77DC50 marker
and the world RNG getter 0x4F02B0 (logs "(int)m_rng.GetSeed()" to A then S).

For every direct `call rel32` site the script records the containing function,
the synchronizer expression in ecx at the call (linear pass, branches merged:
HEURISTIC), the pushed label string, and static direct-call reachability from
native roots (lower bound: virtual calls are not followed).
Output: JSON to argv[1] (or stdout).
"""
import bisect
import collections
import json
import re
import struct
import sys

sys.path.insert(0, r"E:\Games\Axis and Alies\reconstruction\tools\python-packages")
import capstone  # noqa: E402

NATIVE = r"E:\Games\Axis and Alies\reconstruction\research\native-aa"
LOGGERS = {
    0x77DA70: "int",
    0x77D930: "float",
    0x77D9D0: "bool",
    0x77DB10: "text",
    0x77DC50: "marker",
    0x4F02B0: "rng_getter",
    # cdecl wrappers with a fixed instance
    0x40C970: "int_S_wrapper",
    0x40F950: "text_S_wrapper",
    0x40F8D0: "text_A_wrapper",
}
DRAWS = {0x85D840, 0x40C8A0}
ROOTS = {
    "world_update_4F9970": 0x4F9970,
    "world_tick_752AE0": 0x752AE0,
    "admin_tick_755290": 0x755290,
    "frame_75CA70": 0x75CA70,
    "create_gworld_76AA90": 0x76AA90,
    "gworld_init_4F27E0": 0x4F27E0,
    "session_launch_775BF0": 0x775BF0,
    "rmg_generate_5925E0": 0x5925E0,
    "sai_think_7A0080": 0x7A0080,
}
INSTANCES = {"[0xb3655c]": "A", "[0xb36560]": "S"}


def sections(pe):
    e = struct.unpack_from("<I", pe, 0x3C)[0]
    n = struct.unpack_from("<H", pe, e + 6)[0]
    opt = struct.unpack_from("<H", pe, e + 20)[0]
    base = struct.unpack_from("<I", pe, e + 24 + 28)[0]
    out, at = [], e + 24 + opt
    for _ in range(n):
        name = pe[at:at + 8].rstrip(b"\0").decode()
        vsize, va, rsize, raw = struct.unpack_from("<IIII", pe, at + 8)
        out.append((name, base + va, vsize, raw, rsize))
        at += 40
    return out


def load_tsv(name):
    with open(f"{NATIVE}\\{name}", encoding="utf-8", errors="replace") as f:
        header = f.readline().rstrip("\n").split("\t")
        for line in f:
            yield dict(zip(header, line.rstrip("\n").split("\t")))


def main():
    pe = open(f"{NATIVE}\\imported-0.bin", "rb").read()
    secs = sections(pe)
    text = next(s for s in secs if s[0] == ".text")
    _, tva, tvsize, traw, trsize = text

    def off(va):
        for _, sva, vsize, raw, rsize in secs:
            if sva <= va < sva + min(vsize, rsize):
                return raw + va - sva
        return None

    funcs = sorted((int(r["address"], 16), int(r["bodyBytes"] or 0), r["name"]) for r in load_tsv("functions.tsv"))
    starts = [f[0] for f in funcs]

    def func_of(va):
        i = bisect.bisect_right(starts, va) - 1
        if i >= 0 and funcs[i][0] <= va < funcs[i][0] + max(funcs[i][1], 1):
            return funcs[i]
        return None

    strings = sorted((int(r["address"], 16), int(r["length"] or 0), r["value"]) for r in load_tsv("strings.tsv"))
    sstarts = [s[0] for s in strings]

    def string_at(va):
        i = bisect.bisect_right(sstarts, va) - 1
        if i < 0:
            return None
        a, n, v = strings[i]
        if not a <= va < a + n:
            return None
        k = (va - a) // 2 if n >= 2 * len(v) - 2 else va - a
        return v[k:] if k < len(v) else None

    cpp = collections.defaultdict(set)
    for r in load_tsv("string-references.tsv"):
        v = r.get("value", "")
        if ".cpp" in v and "\\" in v and r.get("functionAddress"):
            try:
                cpp[int(r["functionAddress"], 16)].add(v.split("\\")[-1])
            except ValueError:
                pass

    calls = collections.defaultdict(set)
    for r in load_tsv("calls.tsv"):
        try:
            calls[int(r["caller"], 16)].add(int(r["callee"], 16))
        except (KeyError, ValueError):
            pass

    def closure(root):
        seen, todo = {root}, [root]
        while todo:
            f = todo.pop()
            for c in calls.get(f, ()):
                if c not in seen:
                    seen.add(c)
                    todo.append(c)
        return seen

    reach = {name: closure(va) for name, va in ROOTS.items()}

    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
    md.detail = False

    # direct call sites
    body = pe[traw:traw + min(tvsize, trsize)]
    sites = []
    for m in re.finditer(b"\xe8", body):
        i = m.start()
        if i + 5 > len(body):
            break
        site = tva + i
        target = (site + 5 + struct.unpack_from("<i", body, i + 1)[0]) & 0xFFFFFFFF
        if target in LOGGERS:
            sites.append((site, target))

    def disasm(start, end):
        o = off(start)
        return list(md.disasm(pe[o:o + (end - start) + 16], start))

    def context(site):
        f = func_of(site)
        start = f[0] if f else site - 0x60
        if not f:
            # find an alignment that lands exactly on the site
            for s in range(site - 0x60, site):
                ins = disasm(s, site + 5)
                if any(x.address == site for x in ins):
                    start = s
                    break
        ins = [x for x in disasm(start, site + 5) if x.address <= site]
        if not ins or ins[-1].address != site:
            return f, None, [], {}
        regs, pushes, draws_before = {}, [], 0
        for x in ins[:-1]:
            mn, ops = x.mnemonic, x.op_str
            if mn == "call":
                for r in ("eax", "ecx", "edx"):
                    regs[r] = f"ret({ops})" if r == "eax" else "?"
                pushes = []
                try:
                    if int(ops, 16) in DRAWS:
                        draws_before += 1
                except ValueError:
                    pass
                continue
            if mn == "push":
                pushes.append(ops)
                continue
            m = re.match(r"(e[a-d]x|e[sd]i|ebp)\s*,\s*(.*)", ops)
            if mn in ("mov", "lea") and m:
                dst, src = m.group(1), m.group(2)
                src = src.replace("dword ptr ", "")
                regs[dst] = regs.get(src, src) if re.fullmatch(r"e[a-d]x|e[sd]i|ebp", src) else (("lea " if mn == "lea" else "") + src)
            elif m and mn not in ("cmp", "test", "push"):
                regs[m.group(1)] = "?"
        return f, regs.get("ecx", "?"), pushes, {"draws_before_in_function": draws_before}

    rows = []
    for site, target in sorted(sites):
        f, ecx, pushes, extra = context(site)
        label = None
        for p in reversed(pushes):
            try:
                s = string_at(int(p, 16))
            except ValueError:
                continue
            if s:
                label = s
                break
        fva = f[0] if f else None
        rows.append({
            "site": f"{site:08x}",
            "logger": LOGGERS[target],
            "function": f"{fva:08x}" if fva else None,
            "function_name": f[2] if f else None,
            "sources": sorted(cpp.get(fva, ())) if fva else [],
            "instance": INSTANCES.get(ecx, ecx),
            "label": label,
            "pushes": pushes[-4:],
            "reachable_from": [n for n, s in reach.items() if fva in s],
            **extra,
        })
    # draws per getter-calling function
    getter_funcs = collections.Counter(r["function"] for r in rows if r["logger"] == "rng_getter")
    draws = {}
    for fva in getter_funcs:
        if fva is None:
            continue
        f = func_of(int(fva, 16))
        n = 0
        for x in disasm(f[0], f[0] + max(f[1], 1)):
            if x.mnemonic == "call":
                try:
                    n += int(x.op_str, 16) in DRAWS
                except ValueError:
                    pass
        draws[fva] = n
    for r in rows:
        if r["logger"] == "rng_getter" and r["function"]:
            r["draw_calls_in_function"] = draws.get(r["function"])
            r["getter_calls_in_function"] = getter_funcs[r["function"]]
    doc = {"sites": rows, "roots": {k: f"{v:08x}" for k, v in ROOTS.items()}}
    if len(sys.argv) > 1:
        with open(sys.argv[1], "w", encoding="utf-8", newline="\n") as f:
            json.dump(doc, f, indent=1)
            f.write("\n")
    else:
        json.dump(doc, sys.stdout, indent=1)


main()
