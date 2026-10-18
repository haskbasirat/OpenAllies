"""Source-SHA/canonical-path-correlated additional immutable NIF graph fields.

This successor sidecar leaves the frozen schema2 graph batch unchanged. It
projects fields required by material/effect/controller processing, not a native
LoadBinary or processed-metadata success receipt. Unknown positive controllers
and effects are preserved for the runtime's explicit required boundaries.
"""
from pathlib import Path
import sys,io,json,hashlib,argparse,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2];GAME=ROOT.parent
sys.path.insert(0,str(GAME/'reconstruction/tools'))
from convert_models import NifFormat,ORIGINAL,string
from index_native_graphs import write_atomic
def sha(b):return hashlib.sha256(b).hexdigest()
def word(f):return struct.unpack('<I',struct.pack('<f',f))[0]
def rgb(c):return[word(c.r),word(c.g),word(c.b)]
def text(s):
 s=string(s)
 if not s.isascii()or'\0'in s:raise ValueError('actual native non-ASCII name conversion is required')
 return s
def project(source,record):
 raw=(ORIGINAL/source).read_bytes();assert sha(raw)==record['source_sha256']
 graph_bytes=(ROOT/'assets-cache/model-native-graphs'/record['artifact']).read_bytes();assert sha(graph_bytes)==record['sha256'];graph=json.loads(graph_bytes)
 data=NifFormat.Data();data.read(io.BytesIO(raw));blocks={id(b):i for i,b in enumerate(data.blocks)}
 def controllers(owner):
  out=[];seen=set();current=getattr(owner,'controller',None)
  while current is not None:
   if id(current)in seen:raise ValueError('cyclic controller allocation')
   seen.add(id(current));out.append(dict(block=blocks[id(current)],kind=type(current).__name__,flags=int(current.flags),stop=word(current.stop_time)));current=getattr(current,'next_controller',None)
  return out
 nodes=[];seen=set()
 def walk(node):
  if node is None:return
  if id(node)in seen:raise ValueError('aliased AV object remains required')
  seen.add(id(node));index=len(nodes);base=graph['nodes'][index]
  assert base['name']==text(node.name)and base['kind']==type(node).__name__
  props=[blocks[id(p)]for p in node.properties if p is not None]
  assert props==[p['block']for p in base['properties']]
  geometry=None
  if isinstance(node,NifFormat.NiGeometry):
   owner=node.data
   if owner is not None:
    geometry=dict(block=blocks[id(owner)],kind=type(owner).__name__,count=int(owner.num_vertices),vertices_present=bool(owner.has_vertices),normals_present=bool(owner.has_normals),flags_word=int(owner.num_uv_sets)|(int(owner.extra_vectors_flags)<<8),consistency_flags=int(owner.consistency_flags),triangle_count=int(owner.num_triangles)if isinstance(owner,NifFormat.NiTriBasedGeomData)else None,colors=None if not owner.has_vertex_colors else[[word(c.r),word(c.g),word(c.b),word(c.a)]for c in owner.vertex_colors],uv_sets=[[[word(v.u),word(v.v)]for v in row]for row in owner.uv_sets])
  nodes.append(dict(block=blocks[id(node)],kind=type(node).__name__,name=text(node.name),property_blocks=props,effect_blocks=[None if p is None else blocks[id(p)]for p in getattr(node,'effects',[])],controllers=controllers(node),geometry_data=geometry))
  for child in getattr(node,'children',[]):walk(child)
 for root in data.roots:walk(root)
 assert len(nodes)==len(graph['nodes'])
 properties=[];effects=[]
 for i,owner in enumerate(data.blocks):
  if isinstance(owner,NifFormat.NiProperty):
   row=dict(block=i,kind=type(owner).__name__,controllers=controllers(owner))
   if isinstance(owner,NifFormat.NiMaterialProperty):row['material_words']=sum([rgb(getattr(owner,k))for k in ['ambient_color','diffuse_color','specular_color','emissive_color']],[])+[word(owner.glossiness),word(owner.alpha)]
   if isinstance(owner,NifFormat.NiAlphaProperty):row['alpha']=[int(owner.flags),int(owner.threshold)]
   if isinstance(owner,NifFormat.NiVertexColorProperty):row['vertex_color']=[int(owner.vertex_mode),int(owner.lighting_mode)]
   properties.append(row)
  if isinstance(owner,NifFormat.NiDynamicEffect):effects.append(dict(block=i,kind=type(owner).__name__,controllers=controllers(owner)))
 return dict(schema=2,source=source,source_sha256=sha(raw),graph_sha256=record['sha256'],nodes=nodes,properties=properties,effects=effects)
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--source',action='append',required=True);args=parser.parse_args()
 graph_index=json.loads((ROOT/'assets-cache/model-native-graphs/index.json').read_bytes());folder=ROOT/'assets-cache/model-native-fields';folder.mkdir(exist_ok=True);records={}
 for source in args.source:
  row=graph_index['by_source'][source.lower()];assert source==row['source'];doc=project(source,row);name=row['source_sha256']+'-'+sha(source.lower().encode())[:16]+'.json';digest=write_atomic(folder/name,doc);records[source.lower()]=dict(source=source,source_sha256=row['source_sha256'],graph_sha256=row['sha256'],artifact=name,sha256=digest)
 path=folder/'index.json';digest=write_atomic(path,dict(schema=2,scope=__doc__,by_source=records,producer_sha256=sha(Path(__file__).read_bytes())))
 print(json.dumps(dict(index_sha256=digest,source_count=len(records),records=records)))
if __name__=='__main__':main()
