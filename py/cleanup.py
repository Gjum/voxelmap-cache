"""
python3 cleanup.py <region files directory> [-f]

Removes region files (cache, tile, chunk times) that are
outside the 5000 blocks world border of DevotedMC 3.0.

Without -f, only lists the files being removed.
With -f, removes them.
"""
import os, sys

to_remove = []
for f in os.listdir(sys.argv[1]):
    region_pos, file_ending = f.rsplit('.', 1)
    if file_ending in ('zip', 'png', 'gz'):
        x, z = map(int, region_pos.split(',', 1))
        if not -20 <= x < 20 or not -20 <= z < 20:
            to_remove.append(f)

if len(sys.argv) > 2 and sys.argv[2] == '-f':
    print('removing:', len(to_remove))
    for f in to_remove:
        os.remove(f)
else:
    print(*to_remove)
    print('total:', len(to_remove))
