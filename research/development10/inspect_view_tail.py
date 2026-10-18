"""Read-only disassembly of native43F0A0 resource call arguments."""
from pathlib import Path
import sys
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
def wide(va):
 try:
  data=h.P.range(va,300)
  text=data.decode('utf-16-le',errors='replace').split('\0')[0]
  return text if text and all(32<=ord(c)<127 or c in'\t\r\n'for c in text)else''
 except Exception:return''
def main():
 for ins in Cs(CS_ARCH_X86,CS_MODE_32).disasm(h.P.range(0x43f0a0,3759),0x43f0a0):
  if ins.mnemonic=='push'and ins.op_str.startswith('0x'):
   value=int(ins.op_str,16);text=wide(value)
   if text:print(hex(ins.address),'push',repr(text))
  elif ins.mnemonic=='call'and ins.op_str.startswith('0x'):
   print(hex(ins.address),'call',ins.op_str)
if __name__=='__main__':main()
