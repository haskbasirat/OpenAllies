from pathlib import Path
import sys
p=Path('research/development10/view-object-init');sys.path.insert(0,str(p.parent/'creator-labels'))
from native_creator_labels import PE0
for a in [0xa88720,0xab7670,0x9fe0c0,0xa4635c,0xb43cd8,0xb43ce0,0xb43cf0,0xb43f40,0xb43f34]:
 b=PE0.range(a,64);print(hex(a),b.split(b'\0')[0],b[:8].hex())
