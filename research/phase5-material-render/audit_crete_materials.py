"""Read-only authored material-state audit of every Crete model reference."""
from pathlib import Path
import sys, json, hashlib, collections
ROOT=Path(__file__).resolve().parents[2]
GAME=ROOT.parent
sys.path.insert(0,str(GAME/'reconstruction/tools'))
from convert_models import NifFormat, ORIGINAL, string
from convert_models_animations import read_nif

def properties(node, inherited):
    out=dict(inherited)
    for p in node.properties:
        if p is None: continue
        row={'type':type(p).__name__}
        for field in ('flags','threshold','draw_mode','vertex_mode','lighting_mode'):
            if hasattr(p,field): row[field]=int(getattr(p,field))
        if isinstance(p,NifFormat.NiMaterialProperty): row['alpha']=float(p.alpha)
        out[row['type']]=row
    return out

def audit(relative):
    source=ORIGINAL/relative
    data=read_nif(source)
    result=[]; seen=set()
    def walk(n,path=(),inherited=None):
        if not isinstance(n,NifFormat.NiAVObject) or id(n) in seen: return
        seen.add(id(n)); path=path+(string(n.name),)
        props=properties(n,inherited or {})
        if isinstance(n,NifFormat.NiTriBasedGeom):
            result.append({'hierarchy':path,'flags':int(n.flags),'properties':list(props.values())})
        for child in getattr(n,'children',[]): walk(child,path,props)
    for node in data.roots: walk(node)
    return {'source':relative,'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'meshes':result}

placement=GAME/'reconstruction/game/data/map-actors/Campaign/axis01_crete.json'
source=json.loads(placement.read_text())
paths=sorted({r['path'] for t in source['types'].values() for r in t['models']})
rows=[audit(path) for path in paths]
summary=collections.Counter()
for model in rows:
    for mesh in model['meshes']:
        for p in mesh['properties']:
            if p['type'] in ('NiAlphaProperty','NiZBufferProperty','NiStencilProperty','NiMaterialProperty'):
                summary[json.dumps(p,sort_keys=True)]+=1
result={'map':source['id'],'placement_sha256':hashlib.sha256(placement.read_bytes()).hexdigest(),'models':rows,'state_counts':dict(summary)}
output=Path(__file__).with_name('crete-materials.json')
output.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'models':len(rows),'state_counts':dict(summary),'report_sha256':hashlib.sha256(output.read_bytes()).hexdigest()},indent=2))
