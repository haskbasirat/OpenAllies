"""Read-only offline PE signature comparison, not DLL installation/execution."""
from pathlib import Path
import re,sys,json,hashlib,csv
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'research/engine-baseline'))
from audit_baseline import PE
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
H=lambda b:hashlib.sha256(b).hexdigest()
PATTERNS={
 'resolution':'8B 45 ?? 89 55 ?? 89 46',
 'aspect_1':'D8 8E ?? ?? ?? ?? D9 5C 24 ?? D9 C0 D8 8E ?? ?? ?? ?? D8 8E',
 'aspect_2':'D8 8E ?? ?? ?? ?? D9 5C 24 ?? D9 C0 D8 8E ?? ?? ?? ?? D9 5C 24 ?? D8 8E ?? ?? ?? ?? D9 1C 24',
 'camera_fov':'D9 86 ?? ?? ?? ?? A1',
}
def main():
 path=ROOT.parent/'reconstruction/research/native-aa/functions.tsv'
 text=path.read_text(encoding='utf-8-sig')
 if '\t' not in text.splitlines()[0]:text=text.replace('\\t','\t')
 functions=[]
 for row in csv.DictReader(text.splitlines(),delimiter='\t'):
  functions.append((int(row['address'],16),int(row['bodyBytes']),row['name']))
 functions.sort()
 results=[];cs=Cs(CS_ARCH_X86,CS_MODE_32)
 for native_path in [ROOT.parent/'AA.exe',ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe']:
  pe=PE(native_path);image=bytearray(pe.image_size)
  first_raw=min(section[3] for section in pe.sections if section[2])
  image[:first_raw]=pe.b[:first_raw]
  for _,rva,size,off in pe.sections:image[rva:rva+size]=pe.b[off:off+size]
  found={}
  for label,pattern in PATTERNS.items():
   tokens=pattern.split();expr=b''.join(b'.' if t=='??' else re.escape(bytes([int(t,16)])) for t in tokens)
   hits=[]
   for match in re.finditer(b'(?=('+expr+b'))',image,re.DOTALL):
    va=pe.base+match.start();candidates=[f for f in functions if f[0]<=va<f[0]+f[1]]
    enclosing=max(candidates,default=None)
    hits.append({'address':hex(va),'bytes':bytes(image[match.start():match.start()+len(tokens)]).hex(),
                 'enclosing_function':{'address':hex(enclosing[0]),'name':enclosing[2],'body_bytes':enclosing[1]} if enclosing else None,
                 'instructions_from_match':[{'address':hex(i.address),'asm':i.mnemonic+' '+i.op_str,'bytes':i.bytes.hex()}
                       for i in cs.disasm(bytes(image[match.start():match.start()+48]),va)]})
   found[label]={'pattern':pattern,'match_count':len(hits),'matches':hits}
  results.append({'path':str(native_path),'sha256':H(pe.b),'patterns':found})
 out={'schema':1,'scope':__doc__,'method':'PE headers/sections projected to an offline zero-filled image; all wildcard byte matches, with no module loading or memory writes. Native VirtualQuery page protections and execution are not tested.',
      'producer_sha256':H(Path(__file__).read_bytes()),'functions_tsv_sha256':H(path.read_bytes()),'images':results}
 dest=Path(__file__).parent/'native-fov-signatures.json'
 if '--write' in sys.argv:
  with dest.open('x',encoding='utf-8') as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert json.loads(dest.read_text())==out
 print(json.dumps([{ 'path':r['path'],'patterns':{k:[m['address'] for m in v['matches']] for k,v in r['patterns'].items()}} for r in results],indent=2))
if __name__=='__main__':main()
