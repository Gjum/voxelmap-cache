"""
python3 cleanup.py <region files directory> [-f]

Removes region files (cache, tile, chunk times) that are
outside the 13000 blocks world border of CivClassic 2.0.

Without -f, only lists the files being removed.
With -f, removes them.
"""
import os, sys

radius = 52 # 13312 blocks = 52 tiles * 256 blocks/tile

to_remove = []
cleaned_dir = sys.argv[1]
for f in os.listdir(cleaned_dir):
    if '.' not in f: continue
    region_pos, file_ending = f.rsplit('.', 1)
    if file_ending in ('zip', 'png'):#, 'gz'):
        x, z = map(int, region_pos.split(',', 1))
        if not -radius <= x < radius or not -radius <= z < radius:
            to_remove.append(f)

if len(sys.argv) > 2 and sys.argv[2] == '-f':
    print('removing:', len(to_remove))
    for f in to_remove:
        os.remove(cleaned_dir + '/' + f)
else:
    print(*to_remove, sep='\n')
    print('total:', len(to_remove))
