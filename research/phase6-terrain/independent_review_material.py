"""Independent rerun and source/mask integrity checks; no production writes."""
from pathlib import Path
import importlib.util,json,hashlib,re,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];GAME=ROOT.parent
DATA=GAME/'reconstruction/assets/original/Data'
spec=importlib.util.spec_from_file_location('native_compositor_review',HERE/'recover_compositor.py')
native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
sys.path.insert(0,str(GAME/'reconstruction/tools/python-packages'))
from PIL import Image
sha=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
saved=json.loads((HERE/'compositor-fixtures.json').read_text())
assert native.matrix_cases()==saved['matrix_cases']
states,stages=native.state_cases()
assert states==saved['render_state_cases'];assert stages==saved['texture_stages']
selector=native.Selector()
for row in saved['selector_cases']:
    assert selector.select(*row['cell'],row['width'],row['count'])==row['index']
metadata=json.loads((HERE/'terrain-render-metadata.json').read_text())
pins=metadata['sources']+[metadata['translation_source'],metadata['terrain_template']]
for pin in pins:assert sha(DATA/pin['path'])==pin['sha256'],pin['path']
canonical={key:value for key,value in metadata['variants'].items()if key==value['canonical']}
def cleaned(raw):return re.sub(r'/\*.*?\*/|;[^\r\n]*','',raw,flags=re.S)
def value(raw,name):
    match=re.search(r'^\s*'+re.escape(name)+r'\s*=\s*([^\r\n;]+)',raw,re.M|re.I)
    return match[1].strip()if match else None
for ident,entry in canonical.items():
    raw=cleaned((DATA/entry['path']).read_text(encoding='cp1252'))
    blocks=re.split(r'\[Variation\]',raw,flags=re.I)
    matches=[(index,block)for index,block in enumerate(blocks[1:])if value(block,'IDS')==ident]
    assert len(matches)==1,(ident,matches)
    index,block=matches[0]
    assert entry['priority']==int(value(blocks[0],'base_priority'))+index
    assert entry['width']==float(value(block,'texture_width') or 8)
    assert entry['height']==float(value(block,'texture_height') or 8)
    expected=(Path(entry['path']).parent/value(block,'texture')).with_suffix('.png').as_posix()
    assert entry['texture']=='/assets/converted/textures/'+expected
    assert (GAME/'reconstruction'/entry['texture'].lstrip('/')).is_file(),entry['texture']
for ident,entry in metadata['variants'].items():assert entry==metadata['variants'][entry['canonical']]
for names in metadata['biomes'].values():assert all(name in metadata['transition_sets']for name in names)
# Independently trace only active original translation declarations. Do not
# promote disabled historical names or accept a producer's alias table on trust.
translations={'variation':{},'biome':{}}
for section in re.split(r'\[Translate\s+template=Translate\]',cleaned((DATA/'translate.tgi').read_text(encoding='cp1252')),flags=re.I)[1:]:
    node=(value(section,'node') or '').lower()
    if node in translations:translations[node][value(section,'ids_old')]=value(section,'ids_new')
first_variations={};authored_biomes={};first_terrains={}
for path in (DATA/'Terrain').rglob('*.tgi'):
    raw=cleaned(path.read_text(encoding='cp1252'))
    if re.search(r'\[Terrain\s+template=',raw,re.I):
        parts=re.split(r'\[Variation\]',raw,flags=re.I)
        first_variations[value(parts[0],'IDS')]=value(parts[1],'IDS')
    if re.search(r'\[Biome\s+template=',raw,re.I):
        name=value(raw,'IDS')
        authored_biomes[name]=re.findall(r'^\s*set_IDS\s*=\s*(\S+)',raw,re.M|re.I)
        first_terrains[name]=re.findall(r'^\s*terrain_IDS\s*=\s*(\S+)',raw,re.M|re.I)[0]
resolved={};unresolved={}
for node,authored in [('variation',canonical),('biome',authored_biomes)]:
    resolved[node]={};unresolved[node]={}
    for alias in translations[node]:
        target=alias;chain=[]
        while target not in authored and target in translations[node]:
            assert target not in chain
            chain.append(target);target=translations[node][target]
        chain.append(target)
        (resolved if target in authored else unresolved)[node][alias]=chain
assert resolved==metadata['resolved_aliases']
assert unresolved==metadata['unresolved_aliases']
expected_defaults={name:{'terrain_id':terrain,'variation_id':first_variations[terrain]}for name,terrain in first_terrains.items()}
for alias,chain in resolved['biome'].items():expected_defaults[alias]=expected_defaults[chain[-1]]
assert expected_defaults==metadata['biome_defaults']
lookup=json.loads((ROOT/'research/phase5-terrain/mask-lookup.json').read_text())['table']
base_masks=sorted({entry['base_mask']for entry in lookup if entry})
checked=[]
for prefix in metadata['transition_sets'].values():
    for mask in base_masks:
        suffix=''.join(str(i+1)for i in range(9)if i==4 or mask&(1<<(i-int(i>4))))+'.tga'
        original=DATA/(prefix+suffix)
        converted=GAME/'reconstruction/assets/converted/textures'/Path(prefix+suffix).with_suffix('.png')
        with Image.open(original)as source,Image.open(converted)as png:
            assert source.size==png.size and source.convert('RGBA').tobytes()==png.convert('RGBA').tobytes(),original
        checked.append({'original':str(original.relative_to(DATA)),'source_sha256':sha(original),'png_sha256':sha(converted)})
report={'schema':1,'status':'PASS','native_cases':{'matrices':len(saved['matrix_cases']),'selectors':len(saved['selector_cases']),'blend_states':len(states),'stage_sequences':len(stages)},
        'source_pins_verified':len(pins),'canonical_variations':len(canonical),'variations_including_aliases':len(metadata['variants']),
        'original_mask_pixels_verified':len(checked),'masks':checked,'metadata_sha256':sha(HERE/'terrain-render-metadata.json'),
        'fixtures_sha256':sha(HERE/'compositor-fixtures.json'),
        'independent_active_aliases':{k:len(v)for k,v in resolved.items()},'independent_initial_biomes':len(expected_defaults),
        'scope':'Native state/matrix choices and source/converted-mask integrity; not a native screenshot/gamma/cache-texture equivalence claim.'}
(HERE/'independent-material-source-review.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({key:val for key,val in report.items()if key!='masks'}))
