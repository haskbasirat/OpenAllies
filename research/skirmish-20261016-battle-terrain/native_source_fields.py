"""Execute bounded native TerrainData scalar stores and retained first pointers.

Typed getter returns are explicit source facts captured from aa-rules' verified
V14Sources catalog; this does not execute the full TGI parser or texture loader.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
ORIGINAL=h.P
def segment(m,entry,end,ecx=0,esi=0,ebp=0):
 m.w32(h.STACK,h.STOP);m.u.reg_write(h.UC_X86_REG_ESP,h.STACK);m.u.reg_write(h.UC_X86_REG_ECX,ecx);m.u.reg_write(h.UC_X86_REG_ESI,esi);m.u.reg_write(h.UC_X86_REG_EDI,0x77777777);m.u.reg_write(h.UC_X86_REG_EBP,ebp)
 m.u.emu_start(entry,end,count=10000);assert m.u.reg_read(h.UC_X86_REG_EIP)==end
def main():
 source=json.loads((HERE/'source-metadata.json').read_text());cases=[]
 for image in ('original','v14'):
  for data in source['precisions']:
   pc=data['precision'];h.P=ORIGINAL if image=='original' else h.V;m=h.Machine(pc)
   for terrain in data['terrains']:
    obj=m.alloc(0x100);m.w8(obj+0x4c,0xfc);values=iter([terrain['passability_bits']&1,(terrain['passability_bits']>>1)&1])
    m.seams[0x5b5380]=lambda mm:mm.ret(4,next(values))
    segment(m,0x4b88cd,0x4b8911,esi=obj)
    assert m.r8(obj+0x4c)==(0xfc|terrain['passability_bits'])
    if terrain['native_type']==1:
     assert terrain['noise_bits'] is not None;values=iter(terrain['noise_bits'])
     m.seams[0x5b5130]=lambda mm:mm.ret_float(next(values),'f32',4)
     segment(m,0x4b8bda,0x4b8bf8,esi=obj)
     assert m.r32(obj+0x68,2)==terrain['noise_bits'] and m.fpu_top()==0
    indices=[i for i,v in enumerate(data['variations']) if v['terrain']==data['terrains'].index(terrain)]
    assert indices;array=m.alloc(4*len(indices))
    for at,index in enumerate(indices):m.w32(array+at*4,0x10000000+index*16)
    m.w32(obj+0x24,array);segment(m,0x4b8e12,0x4b8e21,esi=obj)
    assert m.r32(obj+0x30)==0x10000000+indices[0]*16
    cases.append(dict(image=image,precision=pc,terrain=terrain['id'],passability_bits=m.r8(obj+0x4c)&3,noise_bits=m.r32(obj+0x68,2) if terrain['native_type']==1 else None,initial_variation=data['variations'][indices[0]]['id']))
   for biome in data['biomes']:
    terrain=data['variations'][biome['variation']]['terrain']
    obj=m.alloc(0x100);array=m.alloc(4);target=0x11000000+terrain*16;m.w32(array,target);m.w32(obj+0x6c,array)
    segment(m,0x4b97b9,0x4b97c1,ebp=obj);assert m.r32(obj+0x78)==target
    cases.append(dict(image=image,precision=pc,biome_path=biome['path'],first_pointer=target))
 doc=dict(schema=1,scope='Native4B87A0 land/water and mountain noise stores; first-entry retained Terrain/Biome pointers, typed source getter seams explicit',original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,source_sha256=hashlib.sha256((HERE/'source-metadata.json').read_bytes()).hexdigest(),cases=cases,instructions=[dict(va=hex(va),bytes=raw) for va,raw in sorted(h.VISITED.items())])
 path=HERE/'native-source-fields.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(cases=len(cases),instructions=len(h.VISITED),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
