##### settings

ts = 256  # tile size (width and height)
outfmt = '{x},{z}.png'  # tiles output paths template
x_off = -20  # x coord of left tile
z_off = -20+5  # z coord of top tile

#####

import math, sys
from PIL import Image

Image.MAX_IMAGE_PIXELS = None

img = Image.open(sys.argv[1])
img_w, img_h = img.size

for z in range(math.ceil(img_h / ts)):
    for x in range(math.ceil(img_w / ts)):
        tile = img.crop((x*ts, z*ts, (x+1)*ts, (z+1)*ts))
        tile.save(outfmt.format(x=x+x_off, z=z+z_off))
