import math
import os
import sys
from PIL import Image

try:
    jm_dir = sys.argv[1]
    vm_dir = sys.argv[2]
except IndexError:
    print('Args: <journeymap directory> <voxelmap directory>')
    sys.exit(1)

os.makedirs(vm_dir, exist_ok=True)

for jm_path in os.listdir(jm_dir):
    if jm_path[-4:] != '.png':
        continue
    jm_img = Image.open(jm_dir+'/'+jm_path)
    jm_x, jm_z = map(int, jm_path[:-4].split(','))
    vm_x0, vm_z0 = jm_x * 2, jm_z * 2
    for dx, dz in ((0, 0), (1, 0), (0, 1), (1, 1)):
        vm_img = jm_img.crop((dx*256, dz*256, (dx+1)*256, (dz+1)*256))
        vm_img.save(vm_dir + '/%i,%i.png' % (vm_x0+dx, vm_z0+dz))
