"""Read rendered PNG pixels for QA metrics; does not modify image files."""
from pathlib import Path
import hashlib
import json
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT.parent/'reconstruction/tools/convert_models_vendor'))
from PIL import Image, ImageChops, ImageStat

base = ROOT/'qa-output/phase5'
pairs = [(name,base/'visuals-before'/f'{name}.png',base/'visuals-after'/f'{name}.png')
         for name in ['crete','tutorial','road','corps','corps_pack','depot_oil','gbr_infantry']]
pairs += [(name,base/'material-focus'/f'{name}-before.png',base/'material-focus'/f'{name}-after.png')
          for name in ['city','farm','vineyard01','vineyard02']]
pairs += [('crete_vulkan_dx12',base/'visuals-after/crete.png',base/'material-focus/crete-dx12.png'),
          ('corps_vulkan_dx12',base/'visuals-after/corps.png',base/'material-focus/corps-dx12.png'),
          ('city_vulkan_dx12',base/'material-focus/city-after.png',base/'material-focus/city-dx12.png')]
rows = []
for name,a,b in pairs:
    left,right = Image.open(a).convert('RGB'),Image.open(b).convert('RGB')
    assert left.size == right.size
    difference = ImageChops.difference(left,right)
    rows.append({'case':name,'before':str(a.relative_to(ROOT)),'after':str(b.relative_to(ROOT)),
                 'before_sha256':hashlib.sha256(a.read_bytes()).hexdigest(),
                 'after_sha256':hashlib.sha256(b.read_bytes()).hexdigest(),
                 'pixel_identical':difference.getbbox() is None,
                 'changed_pixel_count':sum(pixel != (0,0,0) for pixel in difference.getdata()),
                 'mean_absolute_channel_difference':ImageStat.Stat(difference).mean})
report = {'schema':1,'comparisons':rows,
          'visual_observations':[
              'Crete: previously crosswise disconnected pale road strips now form continuous paths in the same camera view.',
              'Crete: map buildings and props have the corrected native planar orientation; authored invisible editor/audio markers are absent.',
              'MediterraneanFarm01: barrels and rear posts previously appeared through the roof/wall; the current render correctly occludes those surfaces.',
              'MediterraneanCityCenter01: fountain overlay responds to the authored native blend state; base building geometry remains unchanged.',
              'ChaparralVineyard01/02: foreground post/foliage layers now occlude rear surfaces according to native depth writes.',
              'The three GBR building inspection renders and road inspection render preserve their previous geometry and appearance.'
          ],
          'remaining_limits':[
              'No original-game screenshot from an identical camera was captured; this is bounded regression QA plus independently executed native state/matrix evidence.',
              'Terrain blending, native water, road draping, shadow rendering and full camera/light parity remain separate incomplete systems.',
              'Static visual inspection cannot establish animation, gameplay, save or multiplayer parity.'
          ]}
(ROOT/'research/phase5-visual-audit/visual-review.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print(json.dumps([{'case':r['case'],'identical':r['pixel_identical'],'changed_pixels':r['changed_pixel_count']} for r in rows]))
