from pathlib import Path
import sys,json,hashlib
GAME=Path(__file__).resolve().parents[3];sys.path.insert(0,str(GAME/'reconstruction/tools'))
from convert_models import NifFormat,ORIGINAL,string
from convert_models_animations import read_nif
OUT=GAME/'rust-rebuild/research/phase2-textures'
MODELS=['Buildings/GBR/GBRCorps/GBRCorps.nif','Buildings/GBR/GBRCorps/GBRCorpsPack.nif','Buildings/GBR/GBRDepotOil/GBRDepotOil.nif']
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def color(c):return [float(c.r),float(c.g),float(c.b)]
def extras(n):
 es=[e for e in getattr(n,'extra_data_list',[])if e];x=getattr(n,'extra_data',None);seen=set()
 while x and id(x)not in seen:seen.add(id(x));es.append(x);x=getattr(x,'next_extra_data',None)
 return [{'type':type(e).__name__,'name':string(getattr(e,'name','')),'value':string(getattr(e,'string_data',''))}for e in es]
def prop(p):
 d={'type':type(p).__name__,'name':string(getattr(p,'name',''))}
 if hasattr(p,'flags'):d['flags']=int(p.flags)
 if isinstance(p,NifFormat.NiMaterialProperty):d.update(diffuse=color(p.diffuse_color),ambient=color(p.ambient_color),emissive=color(p.emissive_color),specular=color(p.specular_color),alpha=float(p.alpha),glossiness=float(p.glossiness))
 if isinstance(p,NifFormat.NiAlphaProperty):d.update(threshold=int(p.threshold))
 if isinstance(p,NifFormat.NiTexturingProperty):
  d.update(apply_mode=int(p.apply_mode),texture_count=int(p.texture_count));slots=[]
  for slot in ['base','dark','detail','gloss','glow','bump_map','decal_0']:
   if getattr(p,'has_'+slot+'_texture',False):
    t=getattr(p,slot+'_texture');slots.append({'slot':slot,'source':string(t.source.file_name)if t.source else None,'uv_set':int(t.uv_set),'clamp_mode':int(t.clamp_mode),'filter_mode':int(t.filter_mode)})
  d['slots']=slots
 if isinstance(p,NifFormat.NiStencilProperty):d.update(draw_mode=int(p.draw_mode))
 if isinstance(p,NifFormat.NiVertexColorProperty):d.update(vertex_mode=int(p.vertex_mode),lighting_mode=int(p.lighting_mode))
 return d
reports=[]
for rel in MODELS:
 p=ORIGINAL/rel;data=read_nif(p);converted=GAME/'reconstruction/assets/converted/models'/Path(rel).with_suffix('.json');d=json.loads(converted.read_text());nodes=[];seen=set()
 def walk(n,path=(),properties=None,hidden=False):
  if not isinstance(n,NifFormat.NiAVObject)or id(n)in seen:return
  seen.add(id(n));name=string(n.name);path=path+(name,);props=dict(properties or{});own=[prop(x)for x in n.properties if x]
  for x in own:props[x['type']]=x
  hidden=hidden or bool(n.flags&1)
  row={'path':list(path),'type':type(n).__name__,'flags':int(n.flags),'hidden_by_self_or_parent':hidden,'extras':extras(n),'own_properties':own}
  for a in ['active_index','switch_node_flags','initial_index','num_children']:
   if hasattr(n,a):row[a]=int(getattr(n,a))
  controller=n.controller;cs=[];ids=set()
  while controller and id(controller)not in ids:
   ids.add(id(controller));cs.append({'type':type(controller).__name__,'flags':int(controller.flags),'start':float(controller.start_time),'stop':float(controller.stop_time)});controller=controller.next_controller
  if cs:row['controllers']=cs
  if isinstance(n,NifFormat.NiTriBasedGeom):
   row['effective_properties']=list(props.values());row['vertices']=int(n.data.num_vertices)if n.data else 0
  nodes.append(row)
  for c in getattr(n,'children',[]):walk(c,path,props,hidden)
 for n in data.roots:walk(n)
 reports.append({'path':rel,'sha256':sha(p),'version':hex(data.version),'converted_sha256':sha(converted),'nodes':nodes,'converted_meshes':[{'name':m['name'],'hierarchy':m['hierarchy'],'material':m['material'],'vertices':len(m['positions'])//3}for m in d['meshes']],'skipped':d['skippedHelpers']})
p=OUT/'original-gbr-materials.json';p.write_text(json.dumps(reports,indent=2)+'\n')
for m in reports:
 print(m['path'],m['version'],'nodes',len(m['nodes']),'meshes',len(m['converted_meshes']))
 for x in m['converted_meshes']:
  material=x['material'];print('/'.join(x['hierarchy']),json.dumps(material,separators=(',',':')))
print('report',p,sha(p))

