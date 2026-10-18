"""Read-only native/asset observations; writes only this evidence directory."""
from pathlib import Path
import collections, hashlib, json, re, struct, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
RECON = ROOT / 'reconstruction'
sys.path.insert(0, str(RECON / 'tools'))
sys.path.insert(0, str(RECON / 'tools/python-packages'))
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
from map_extract import read_chunks

def sha(b): return hashlib.sha256(b).hexdigest()
def pin(p):
    b = p.read_bytes()
    return {'path': p.relative_to(ROOT).as_posix(), 'bytes': len(b), 'sha256': sha(b)}

class PE:
    def __init__(self, path):
        self.path, self.b = path, path.read_bytes()
        pe = struct.unpack_from('<I', self.b, 0x3c)[0]
        n = struct.unpack_from('<H', self.b, pe+6)[0]
        op = pe+24
        self.base = struct.unpack_from('<I', self.b, op+28)[0]
        self.image_size = struct.unpack_from('<I', self.b, op+56)[0]
        sec = op+struct.unpack_from('<H', self.b, pe+20)[0]
        self.sections = [struct.unpack_from('<IIII', self.b, sec+i*40+8) for i in range(n)]
    def offset(self, va):
        rva = va-self.base
        for vs, r, size, off in self.sections:
            if r <= rva < r+size: return off+rva-r
        raise ValueError(hex(va))
    def range(self, va, size):
        off = self.offset(va)
        return self.b[off:off+size]

def strip_comments(text):
    return re.sub(r';;[^\r\n]*', '', re.sub(r'/\*.*?\*/', '', text, flags=re.S))

def terrain_metadata():
    source = RECON/'assets/original/Data'
    template = source/'Templates/template_terrain.tgi'
    raw_template = strip_comments(template.read_text(encoding='cp1252'))
    mountain = raw_template.split('[template MountainTerrain inherit=Terrain]',1)[1].split('[template WaterTerrain',1)[0]
    defaults = {field: float(re.search(r'\b'+field+r'\s*=\s*([-\d.]+)', mountain)[1]) for field in ('noise_min','noise_max')}
    variations, inputs = {}, []
    for p in sorted((source/'Terrain').rglob('*.tgi')):
        raw = strip_comments(p.read_text(encoding='cp1252'))
        root = re.search(r'\[Terrain\s+template=(\w+)\]', raw, re.I)
        if not root: continue
        assert len(re.findall(r'\[Terrain\s+template=',raw,re.I)) == 1, p
        base = raw[root.end():].split('[Variation]',1)[0]
        parent = root[1].lower()
        typ = {'terrain':0,'mountainterrain':1,'waterterrain':2}[parent]
        explicit = re.search(r'^\s*type\s*=\s*(\w+)',base,re.M|re.I)
        if explicit: typ = {'land':0,'mountain':1,'water':2}[explicit[1].lower()]
        ids = re.search(r'^\s*IDS\s*=\s*(\S+)',base,re.M|re.I)[1]
        noise = {}
        if typ == 1:
            for f in defaults:
                m = re.search(r'^\s*'+f+r'\s*=\s*([-\d.]+)',base,re.M|re.I)
                noise[f] = float(m[1]) if m else defaults[f]
        record = pin(p); inputs.append(record)
        for section in re.split(r'\[Variation\]', raw, flags=re.I)[1:]:
            m = re.search(r'^\s*IDS\s*=\s*(\S+)',section,re.M|re.I)
            assert m, p
            key = m[1]
            assert key not in variations, key
            variations[key] = {'terrain_id':ids,'native_type':typ,'type':['LAND','MOUNTAIN','WATER'][typ],
                               **noise, 'source':record['path'], 'source_sha256':record['sha256']}
    translation_path=source/'translate.tgi'
    translations={}
    for block in re.split(r'\[Translate\s+template=Translate\]',strip_comments(translation_path.read_text(encoding='cp1252')),flags=re.I)[1:]:
        fields=dict(re.findall(r'^\s*(node|ids_old|ids_new)\s*=\s*(\S+)',block,re.M|re.I))
        if fields.get('node','').lower()!='variation': continue
        key,value=fields['ids_old'],fields['ids_new']
        assert key not in translations or translations[key]==value, key
        translations[key]=value
    canonical=dict(variations)
    resolved_aliases={}
    unresolved_aliases={}
    for key in translations:
        chain=[key]; current=key
        while current not in canonical and current in translations:
            current=translations[current]
            assert current not in chain, chain
            chain.append(current)
        if current in canonical:
            variations[key]={**canonical[current],'canonical_variation':current,'translation_chain':chain}
            resolved_aliases[key]=chain
        else: unresolved_aliases[key]=chain
    maps=[]; missing=set(); max_rounding=0.0
    for p in sorted((source/'Maps').rglob('*')):
        if p.suffix.lower() != '.rmp': continue
        chunks = read_chunks(p)
        relative=p.relative_to(source/'Maps').with_suffix('.json')
        jpath=RECON/'game/data/maps'/relative
        doc=json.loads(jpath.read_text())
        missing.update(v['id'] for v in doc['palette'].values() if v['id'] not in variations)
        for chunk, field in [('HEIGHT_MAP','heights'),('MOUNTAIN_HEIGHT_MAP','mountainHeights')]:
            raw=chunks[chunk][0]; count=struct.unpack_from('<I',raw,20)[0]
            vals=struct.unpack_from('<'+str(count)+'f',raw,24)
            assert len(doc[field])==count
            max_rounding=max(max_rounding,max(abs(a-b) for a,b in zip(vals,doc[field])))
        unknown={key:v['id'] for key,v in doc['palette'].items() if v['id'] not in variations}
        maps.append({'id':doc['id'],'width':doc['width'],'height':doc['height'],
                     'unresolved_variations':unknown,
                     'unresolved_tile_count':sum(str(v) in unknown for v in doc['tiles']),
                     'height_header_u32':list(struct.unpack_from('<6I',chunks['HEIGHT_MAP'][0])),
                     'source':pin(p),'recovered_json':pin(jpath)})
    return {'schema':'aa-native-terrain-variation-evidence-v1', 'scope':'Derived from original TGI, for recovered map palette IDs; not a full TGI parser.',
            'template':pin(template),'translation_source':pin(translation_path),'sources':inputs,
            'canonical_variation_count':len(canonical),'variation_count':len(variations),
            'resolved_aliases':resolved_aliases,'unused_unresolved_aliases':unresolved_aliases,
            'variations':variations,'map_count':len(maps),'missing_map_palette_variations':sorted(missing),
            'maps':maps,'non_square_maps':[m['id'] for m in maps if m['width']!=m['height']],
            'max_json_height_rounding_absolute_error':max_rounding}

def main():
    imported=PE(RECON/'research/native-aa/imported-0.bin')
    accepted=PE(RECON/'native-game/runtime-expanded-v13-original/AA.exe')
    assert sha(imported.b)=='cbf863775762d0113bed0b6eb0eb074df6356eaad647f7cd2ad9a5cc96b1b3b2'
    assert sha(accepted.b)=='3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'
    ranges=[(0x4f0e50,0x22a),(0x550190,0x80),(0x550210,0x220),(0x5519a0,0x1a0),
            (0x4f27e0,0x100),(0x4f9970,0x250),(0x4b86b0,0xf0),(0x4b87a0,0x500),
            (0x431bc0,0x160),(0x71e2c0,0x460),(0x5065d0,0x440)]
    cs=Cs(CS_ARCH_X86,CS_MODE_32); evidence=[]
    for va,size in ranges:
        b=imported.range(va,size)
        assert b==accepted.range(va,size), hex(va)
        name=f'{va:08x}-assembly.txt'
        (HERE/name).write_text('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}' for i in cs.disasm(b,va))+'\n')
        evidence.append({'va':f'0x{va:08x}','bytes':size,'sha256':sha(b),'accepted_identical':True,'assembly':name})
    banners=[]
    for va in (0xAB94D8,0xB31FF4,0xB32240,0xB323F8,0xB32D34):
        off=imported.offset(va); raw=imported.b[off:off+160]
        # All five module/file banners are narrow strings in this image.
        txt=raw.split(b'\0',1)[0].decode('ascii')
        assert '10.1.0.0' in txt, (hex(va),txt)
        banners.append({'va':hex(va),'file_offset':off,'text':txt})
    versions=collections.Counter()
    for p in (RECON/'assets/original').rglob('*'):
        if p.suffix.lower()!='.nif': continue
        with p.open('rb') as f: header=f.read(100).split(b'\n',1)[0]
        versions[header.decode('ascii')]+=1
    terrain=terrain_metadata()
    (HERE/'terrain-variation-metadata.json').write_text(json.dumps(terrain,indent=2)+'\n')
    report={'scope':'Read-only original binary/asset baseline; no gameplay parity assertion or accepted runtime changes.',
            'generator':pin(Path(__file__)),'imported_binary':pin(imported.path),'accepted_native':pin(accepted.path),
            'unchanged_native_ranges':evidence,'engine_banners':banners,'original_nif_headers':dict(versions),
            'terrain_metadata':pin(HERE/'terrain-variation-metadata.json'),
            'native_height_fixture':pin(HERE/'native-height-tests.json') if (HERE/'native-height-tests.json').exists() else None,
            'native_rules':{
                'coordinate_basis':'Native X/Y ground, Z height; native render height is not divided by 5. Converted mesh JSON already maps to (X,Z,-Y) and flips UV V.',
                'height_lattice':'HeightMap uses spacing 1<<resolutionBits; stock bits1 gives2. Index=x+(width+1)*y. Sample origin is (0,0).',
                'height_query':'GWorld0x4F0E50 uses base+0x34. Query bit0x10 selects nearest vs interpolated sample. With bit2, current or any8-neighbor terrain type1/2 permits interpolated mountain+0x38. Otherwise mountain contribution is0.',
                'mountain_noise':'Only current type1/MOUNTAIN adds noise_min + (noise_max-noise_min)*(((tileIndex*4723+6323)%6997)%1000)*float32(0.001). Type2/WATER and neighbors do not add mountain noise.',
                'render_flags':'0x431BC0 render height grid queries0x122; 0x71E2C0 terrain edge queries0x123. Bit1 lower-clamps combined height to base-0.5; bits1+4 returnsbase-0.5.',
                'tick':'GWorld Init0x4F27E0 sets dt=1/SVarGameTicksPerGameSecond. Stock16 =>0.0625.0x4F9970 increments integer tick and derives worldtime=tick*dt.',
                'terrain_types':'0x4B86B0 registers LAND0/MOUNTAIN1/WATER2.0x4B87A0 binds noise_min/+68,noise_max/+6C for MOUNTAIN. These are type labels, not separate land/water passability flags.'},
            'limits':[
                'Gamebryo10.1.0.0 identifies embedded module banners/file format, not a recovered licensed SDK/source tree.',
                'RMP-derived JSON heights are rounded5decimal; original floats must be used for bit-exact numerical parity.',
                'All80 original maps here are square; dimension root inference must not be generalized to arbitrary rectangles.',
                'Twenty-one synthetic cases execute actual native height query/samplers/CRT; this is not exhaustive x87 or map-render parity. The Rust preview must still disclose rounded source heights and floating-point differences.',
                'Converted meshes use recovered authored/bind pose, highest LOD and omit helpers; actor yaw, animations, terrain tiling/blending and native camera projection remain incomplete.',
                '16Hz and ordered asset preprocessing are foundations; no combat, economy, AI, save, networking or full deterministic simulation parity follows.']}
    (HERE/'native-engine-baseline.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'passed':True,'banners':banners,'nifs':dict(versions),'maps':terrain['map_count'],
                      'variations':terrain['variation_count'],'metadata_sha256':report['terrain_metadata']['sha256']}))

if __name__=='__main__': main()
