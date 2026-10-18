"""Recursive-descent disassembly of one AA.exe function (read-only).
Usage: disfunc.py ADDR [ADDR...]  -> prints instructions in address order,
following jumps (not calls) until ret/jmp-out; annotates wide strings."""
import sys, struct
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-combat'))
import harness as h
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
md = Cs(CS_ARCH_X86, CS_MODE_32)
def wide(va):
    try:
        raw = h.ORIGINAL.at(va, 200)
    except ValueError:
        return None
    t = raw.decode('utf-16le', 'replace').split('\x00')[0]
    return t if len(t) >= 3 and all(32 <= ord(c) < 127 for c in t) else None
def func(start):
    seen = {}
    work = [start]
    while work:
        a = work.pop()
        while a not in seen:
            try:
                i = next(md.disasm(h.ORIGINAL.at(a, 16), a))
            except (StopIteration, ValueError):
                break
            seen[a] = i
            m = i.mnemonic
            if m.startswith('ret') or m == 'int3':
                break
            if m.startswith('j'):
                if i.op_str.startswith('0x'):
                    t = int(i.op_str, 16)
                    if m == 'jmp':
                        if abs(t - start) < 0x4000:
                            a = t
                            continue
                        break
                    work.append(t)
                elif m == 'jmp':
                    break
            a = i.address + i.size
    return [seen[k] for k in sorted(seen)]
def main():
  for arg in sys.argv[1:]:
      va = int(arg, 16)
      print(f'===== {va:08x}')
      for i in func(va):
          note = ''
          for tok in i.op_str.replace(',', ' ').replace(']', ' ').replace('[', ' ').split():
              if tok.startswith('0x') and len(tok) >= 7:
                  s = wide(int(tok, 16))
                  if s:
                      note = f'   ; L"{s[:80]}"'
          print(f'{i.address:08x}  {i.mnemonic} {i.op_str}{note}')

if __name__ == '__main__':
    main()
