"""Execute original WaterInfo interpolation and retain authored water inputs."""
from pathlib import Path
import json, re, struct, sys, random
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified,sha
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_EAX,UC_X86_REG_EIP
DATA=ROOT.parent/'reconstruction/assets/original/Data'
def strip(s):return re.sub(r';[^\r\n]*','',re.sub(r'/\*.*?\*/','',s,flags=re.S))
def field(s,name):
 m=re.search(r'^\s*'+name+r'\s*=\s*([^\r\n;]+)',s,re.M|re.I)
 return m[1].strip() if m else None
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
def curves(raw):
 result=[]
 for part in re.split(r'\[DepthInfo\]',raw,flags=re.I)[1:]:
  result.append([float(field(part,'depth')),*map(float,field(part,'color').split(',')),float(field(part,'alpha'))])
 return sorted(result,key=lambda a:a[0])
def source(p):return dict(path=p.relative_to(DATA).as_posix(),sha256=sha(p.read_bytes()))
def metadata():
 t=DATA/'Templates/template_waterinfo_aa.tgi'
 templates={}
 for part in re.split(r'\[Template\s+',strip(t.read_text(encoding='cp1252')),flags=re.I)[1:]:
  templates[part.split()[0].lower()]=curves(part)
 sources=[source(t)]; biomes={};terrains={}
 for p in sorted((DATA/'Terrain').rglob('*.tgi')):
  raw=strip(p.read_text(encoding='cp1252'))
  if re.search(r'\[Terrain\s+template=WaterTerrain\]',raw,re.I):
   base=raw.split('[Variation]',1)[0]
   tex=field(base,'anim_textures_path_prefix')
   if not tex:continue
   frame=p.parent/(tex % 0)
   ident=field(base,'IDS')
   terrains[ident]={'first_frame':'/assets/converted/textures/'+frame.relative_to(DATA).with_suffix('.png').as_posix(),
    'frames_per_second':float(field(base,'anim_fps') or 10),'source':source(p),'first_frame_source':source(frame)}
   sources.extend([source(p),source(frame)])
  if re.search(r'\[Biome\s+template=',raw,re.I):
   name=field(raw,'IDS'); water={}
   for part in re.split(r'\[WaterInfo',raw,flags=re.I)[1:]:
    header,body=part.split(']',1)
    ids=field(body,'terrain_ids')
    templ=re.search(r'Template=(\w+)',header,re.I)
    curve=templates[templ[1].lower()] if templ and templ[1].lower() in templates else curves(body)
    assert curve,(p,ids)
    for ident in ids.split(','):water[ident.strip()]=curve
   if water:biomes[name]=water;sources.append(source(p))
 # Existing terrain metadata already source-binds canonical translation aliases.
 terrain=json.loads((ROOT/'research/phase6-terrain/terrain-render-metadata.json').read_text())
 variants={k:v['terrain_id'] for k,v in terrain['variants'].items() if v['terrain_template']=='waterterrain' and v['terrain_id'] in terrains}
 if 'arctic_forest' not in biomes and 'boreal_forest' in biomes:biomes['arctic_forest']=biomes['boreal_forest']
 avars=DATA/'AVars.tgi';sources.append(source(avars))
 block=re.search(r'\bint\s+TerrainBlockSize\s*=\s*(\d+)',strip(avars.read_text(encoding='cp1252')))
 return dict(schema=1,block_size=int(block[1]),terrains=terrains,variants=variants,biomes=biomes,sources=sources,
  terrain_metadata_sha256=sha((ROOT/'research/phase6-terrain/terrain-render-metadata.json').read_bytes()))
def native_curve(m,points,depth):
 m.reset(0);m.w(m.STATE+8,m.STATE+0x100);m.w(m.STATE+12,len(points))
 m.u.mem_write(m.STATE+0x100,b''.join(struct.pack('<5f',*p) for p in points))
 m.w(m.STACK+4,m.STATE+0x900);m.w(m.STACK+8,bits(depth))
 m.u.emu_start(0x4b6e10,m.STOP,count=10000)
 assert m.u.reg_read(UC_X86_REG_ESP)==m.STACK+12
 return list(struct.unpack('<4I',m.u.mem_read(m.STATE+0x900,16)))
def native_quad(m,cell,size):
 m.reset(0);m.w(0xb36534,m.STATE+0x200);m.w(m.STATE+0x230,m.STATE+0x400);m.w(m.STATE+0x408,bits(size))
 m.w(m.STACK+4,m.STATE+0x100);m.w(m.STATE+0x100,cell[0]);m.w(m.STATE+0x104,cell[1]);m.w(m.STATE+4,0);m.w(m.STATE+8,100)
 vertices=[];slots=[]
 def hook(u,a,size,_):
  if a not in [0x737600,0x766de0]:return
  sp=u.reg_read(UC_X86_REG_ESP)
  if a==0x737600:
   value=len(vertices);vertices.append(list(struct.unpack('<2I',u.mem_read(m.r(sp+4),8))));pop=8
  else:
   value=m.STATE+0x1000+len(slots)*4;slots.append(value);m.w(m.STATE+4,len(slots));pop=4
  u.reg_write(UC_X86_REG_EAX,value);u.reg_write(UC_X86_REG_EIP,m.r(sp));u.reg_write(UC_X86_REG_ESP,sp+pop)
 h=m.u.hook_add(UC_HOOK_CODE,hook);m.u.emu_start(0x7376b0,m.STOP,count=10000);m.u.hook_del(h)
 assert m.u.reg_read(UC_X86_REG_ESP)==m.STACK+8
 return dict(cell=cell,cell_size_bits=bits(size),vertices_bits=vertices,indices=[m.r(p) for p in slots])
def native_selection(m,mask,cell):
 #738320 consumes current block-localX+originX and worldY. Synthetic stack
 #values bind those documented inputs; no complete738280 allocator is claimed.
 m.reset(0);m.w(0xb36534,m.STATE+0x200);m.w(m.STATE+0x230,m.STATE+0x400)
 m.w(m.STATE+0x41c,3);m.w(m.STATE+0x420,3);m.w(m.STATE+0x428,m.STATE+0x1000)
 for i in range(9):
  m.w(m.STATE+0x1000+i*48,m.STATE+0x3000+i*64)
  m.w(m.STATE+0x301c+i*64,m.STATE+0x7000+i*64)
  m.w(m.STATE+0x701c+i*64,2 if mask&(1<<i) else 0)
 m.w(m.STACK+0x10,cell[0]);m.w(m.STACK+0x2c,0);m.w(m.STACK+0x20,cell[1]);m.w(m.STACK+0x1c,0)
 selected=[]
 def hook(u,a,size,_):
  if a in [0x7383e2,0x738476]:
   selected.append((m.r(m.STACK+0x1c)-m.STATE-0x3000)//64 if a==0x7383e2 else None);u.emu_stop()
 h=m.u.hook_add(UC_HOOK_CODE,hook);m.u.emu_start(0x738320,m.STOP,count=10000);m.u.hook_del(h)
 assert len(selected)==1
 return dict(mask=mask,cell=cell,selected=selected[0])
def main():
 meta=metadata(); cases=[];rng=random.Random(736)
 unique={json.dumps(c):c for b in meta['biomes'].values() for c in b.values()}
 unique['singleton']=[[0.25,0.3,0.5,0.75,0.9]]
 for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  m=Machine(cw)
  for curve in unique.values():
   depths=[-1,0,30]+[p[0] for p in curve]+[(a[0]+b[0])*0.5 for a,b in zip(curve,curve[1:])]+[rng.uniform(-0.5,5) for _ in range(12)]
   for d in depths:
    cases.append(dict(precision=pc,points_bits=[[bits(v) for v in p] for p in curve],depth_bits=bits(d),rgba_bits=native_curve(m,curve,d)))
 quads=[native_quad(Machine(0x7f),cell,size) for cell in [[0,0],[1,1],[17,93],[511,255]] for size in [1,2,4,8]]
 m=Machine();selection=[native_selection(m,mask,cell) for cell in [[1,1],[0,0],[2,0],[0,2],[2,2],[0,1],[2,1],[1,0],[1,2]] for mask in range(512)]
 ranges={}
 for name,a,b in [('curve',0x4b6e10,0x4b6f8c),('surface_colors',0x736ab0,0x736c60),('regular_quad',0x7376b0,0x7377e2),('water_mesh_material',0x7377f0,0x7381eb),('water_cell_selection',0x738280,0x7385b6),('texture_matrix',0x736d50,0x736dc1)]:
  blob=original.range(a,b-a);assert blob==modified.range(a,b-a)
  ranges[name]={'start':hex(a),'end':hex(b),'sha256':sha(blob),'hex':blob.hex()}
 (HERE/'water-metadata.json').write_text(json.dumps(meta,indent=2)+'\n')
 (HERE/'native-water-vectors.json').write_text(json.dumps({'schema':1,'original_sha256':sha(original.b),'ranges':ranges,'curve':cases,'regular_quads':quads,'selection':selection,'quad_hooks':'Only737600 vertex allocation/dedup and766DE0 index allocation are replaced; original7376B0 arithmetic and index emissions execute.'},indent=2)+'\n')
 print(json.dumps({'curves':len(unique),'cases':len(cases),'regular_quads':len(quads),'terrains':len(meta['terrains']),'variants':len(meta['variants']),'biomes':len(meta['biomes'])}))
if __name__=='__main__':main()
