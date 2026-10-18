"""Current plaintext film MAPSTRINGS representation: full native parser,
TextData field bucket order/prepend, lookup and49AD70 key-only resolution.
No byte-identical generic TGI formatter or Unicode/font-escape claim.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'))
from native_mapstrings import MapStrings,L
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}

def literal(units):
 assert all(32<=u<=255 and u not in [34,38,92,127] for u in units)
 return (b'@' if units[:1]==[64] else b'')+bytes(units)
def encode(case):
 data=b'[Text language=Default]\r\n{\r\n\tids = map_strings_table\r\n\tname = "'+literal(case['name'])+b'"\r\n\r\n'
 for key,value in case['entries']:data+=b'\t'+key.encode()+b' = "'+literal(value)+b'"\r\n'
 return data+b'}\r\n'

def run(case,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED
 m=MapStrings();m.u.reg_write(UC_X86_REG_FPCW,PC[pc]);field_order=[];capacity=[];constructor_pins=[]
 def fields(u,at,size,_):
  node=m.arg(0);ptr=m.r(node+64);n=m.r(node+68);capacity.append(n)
  for bucket in range(n):
   entry=m.r(ptr+4*bucket)
   while entry:field_order.append(m.text(m.r(m.r(entry)+4)));entry=m.r(entry+4)
 m.u.hook_add(UC_HOOK_CODE,fields,begin=0x4ba710,end=0x4ba710)
 raw=encode(case);mgr,entries=m.load(raw);assert mgr is not None
 # Run real enum initialization: only manager registration is a storage seam.
 array=m.alloc(28);m.w(0xb3b198,array)
 def enum_register():
  entry=m.alloc(12);m.w(entry+8,m.arg(2));m.w(array+4*m.arg(0),entry);m.ret(12,entry)
 m.seam(0x4976b0,enum_register);m.call(0x4981a0,m.alloc(32))
 resource=m.alloc(0xb4);stream=resource+60;vt=m.alloc(0x50);m.w(stream,vt);header=m.alloc(32);rules=m.alloc(32)
 m.w(resource+144,header);m.w(header+4,rules);m.w(rules+4,3);m.w(vt+0x48,0x6a71d0)
 pending=bytearray()
 def read():
  n=m.arg(1);assert n<=len(pending);m.u.mem_write(m.arg(0),bytes(pending[:n]));del pending[:n];m.ret(8)
 readstub=L.STUBS+0xc00;m.seam(readstub,read);m.w(vt,readstub)
 lookups=[]
 for key,expected in case['entries']:
  keyraw=key.encode('utf-16le');pending.extend((len(keyraw)//2).to_bytes(2,'little')+keyraw)
  pair=m.alloc(8);m.w(pair,m.cstring(''));m.w(pair+4,m.cstring('stale'))
  m.call(0x49ad70,0,[stream,pair]);assert not pending
  actual=m.text(m.r(pair+4));units=[ord(u)for u in actual];assert units==expected,(key,units,expected)
  assert all(m.u.reg_read(r)==0x5a5a0000 for r in [UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])
  lookups.append([key,units])
 assert (m.u.reg_read(UC_X86_REG_FPSW)>>11)&7==0
 # Every basic block is pinned against both executables, even in v14 runs.
 blocks=m.windows()
 return dict(bytes_hex=raw.hex(),field_capacity=capacity[0],field_order=field_order,
  entry_order=[key for key,_ in entries],lookups=lookups),blocks

def cases():
 allowed=[u for u in range(32,256)if u not in [34,38,92,127]]
 yield dict(name=[ord(c)for c in 'Film'],entries=[['map_name',[ord(c)for c in 'Film']],['all_supported_bytes',allowed],['leading_at',[64,64,65]],['empty',[]]])
 for n in [1,8,9,20,50,100,200,400]:
  yield dict(name=[ord(c)for c in 'Order'],entries=[['map_name',[ord(c)for c in 'Order']]]+[[f'key{i}',[65+(i%26)]]for i in range(n)])

def main():
 rows=[];blocks={};actual=[]
 for case in cases():
  for pc in PC:
   a,pa=run(case,'original',pc);b,pb=run(case,'v14',pc);assert a==b
   blocks.update({r['address']:r for r in pa+pb});rows.append(dict(input=case,pc=pc,output=a))
 # The current target maps seed the same order owner from their exact raw
 #MAPSTRINGS, rather than from encoder-generated surrogate source texts.
 sys.path.insert(0,str(ROOT/'research/phase6-mapheader'));from inspect_header import read_chunks
 for path in ['Campaign/tutorial02.RMP','DeveloperMaps/(3v3)Firefight.RMP','(2v2)Berlin.RMP']:
  raw=read_chunks(ROOT.parent/'reconstruction/assets/original/Data/Maps'/path)['MAPSTRINGS'][0]
  for pc in PC:
   outputs=[]
   for image in ['original','v14']:
    L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=MapStrings();m.u.reg_write(UC_X86_REG_FPCW,PC[pc])
    mgr,entries=m.load(raw);assert mgr is not None
    outputs.append(dict(entry_order=[k for k,_ in entries],entries=[[k,[ord(c)for c in v]]for k,v in entries]))
    blocks.update({r['address']:r for r in m.windows()})
   assert outputs[0]==outputs[1]
   actual.append(dict(path=path,pc=pc,source_hex=raw.hex(),sha256=hashlib.sha256(raw).hexdigest(),output=outputs[0]))
 doc=dict(schema=1,scope=__doc__,original_sha256=L.pe.ORIGINAL_SHA,v14_sha256=L.pe.ACCEPTED_SHA,
  cases=rows,actual_sources=actual,blocks=[blocks[k]for k in sorted(blocks)])
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-film-text.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows)+len(actual),executions=2*(len(rows)+len(actual)),blocks=len(blocks),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
