"""Separate original/v14 TeamData leaves and real color-vector initialization.

Global point words are explicit arbitrary transport inputs, not a claimed
startup constructor. The PointShape owner has a separate startup writer proof.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
ORIGINAL=h.P
def main():
 rows=[]
 leaves={0x24:0x6a71f0,0x28:0x4942f0,0x2c:0x6a71f0,0x30:0x4c9fa0,0x38:0x4c9fa0,0x3c:0x4c9fa0}
 for image in ('original','v14'):
  for pc in h.PRECISION:
   h.P=ORIGINAL if image=='original' else h.V;m=h.Machine(pc)
   for slot,leaf in leaves.items():assert m.r32(0xa20770+slot)==leaf
   assert m.r32(0xa29780+0xb0)==m.r32(0xa29780+0xb4)==0x6a71f0
   assert m.r32(0xa29780+0xc)==0x4e3e90
   obj=m.alloc(0x80);m.u.reg_write(h.UC_X86_REG_ESI,obj);m.u.reg_write(h.UC_X86_REG_ESP,h.STACK)
   m.u.emu_start(0x4e67ff,0x4e6810,count=100);assert m.r32(obj)==0xa29780
   rows.append(dict(image=image,precision=pc,operation='team_constructor_vtable_store',output=hex(m.r32(obj))))
   for words in ([0]*6,[3,0x3f800000,0xbf800000,0xc0000000,0x3f800000,0x40000000],[0xffffffff,0x7fc01234,0x80000000,1,0xdeadbeef,0x7f800000]):
    m.w32(0xb3a8c4,*words);out=m.alloc(24);m.w32(out,*([0xcccccccc]*6))
    result=m.call(0x4942f0,0,[out]);assert result==out and m.r32(out,6)==words
    rows.append(dict(image=image,precision=pc,operation='global_point_copy',input=words,output=m.r32(out,6)))
   for leaf in (0x6a71f0,0x4c9fa0):
    result=m.call(leaf,0,[]);assert result&255==0
    if leaf==0x4c9fa0:assert result==0
    rows.append(dict(image=image,precision=pc,operation='false_leaf',function=hex(leaf),output=result&255,full_eax=result if leaf==0x4c9fa0 else None))
   # GWorld4F2972 reads KingdomColor manager+28 (GameData+3F4),
   # then4F2979 calls430240 on the already-constructed World+160 vector.
   for count in (0,1,2,8,17,33):
    header=m.alloc(12);old=m.alloc(4);m.w32(old,0xcccccccc);m.w32(header,old,0,1)
    m.seams[0x6f7810]=lambda mm:mm.ret(0,mm.alloc(mm.arg(0)))
    m.seams[0x6f7430]=lambda mm:mm.ret()
    m.call(0x430240,header,[count]);assert m.r32(header+4)==count and m.r32(m.r32(header),count)==[0]*count
    rows.append(dict(image=image,precision=pc,operation='color_count_initialize',count=count,output=m.r32(m.r32(header),count)))
 pins=[dict(va=hex(va),bytes=raw) for va,raw in sorted(h.VISITED.items())]
 static=[dict(va=hex(va),bytes=ORIGINAL.range(va,size).hex()) for va,size in ((0xa20770+0x24,0x1c),(0xa29780+0xb0,8),(0xa29780+0xc,4),(0x4e67ff,0x11),(0x4f2949,6),(0x4f296c,0x12))]
 for pin in static:assert bytes.fromhex(pin['bytes'])==h.V.range(int(pin['va'],16),len(bytes.fromhex(pin['bytes'])))
 doc=dict(schema=1,scope=__doc__,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,cases=rows,instructions=pins,static=static)
 path=HERE/'native-team-leaves.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(cases=len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
