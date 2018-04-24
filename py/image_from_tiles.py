"""
python3 image_from_tiles.py <image path> <tiles dir>

Combines all tiles in <tiles dir> into a single image.
"""
import os
import sys
import time
from PIL import Image

def stitch_all(img_path, tiles_dir):
    tiles = [tuple(map(int, tile[:-4].split(',')))
             for tile in os.listdir(tiles_dir)
             if tile[-4:] == '.png']

    tile_size = Image.open(tiles_dir + '/%i,%i.png' % tiles[0]).size[0]

    min_x = min(x for x,z in tiles)
    min_z = min(z for x,z in tiles)
    max_x = max(x for x,z in tiles)
    max_z = max(z for x,z in tiles)
    width  = max_x - min_x + 1
    height = max_z - min_z + 1

    out = Image.new('RGBA', (width*tile_size, height*tile_size))

    last_progress = time.time()
    for tn, tile in enumerate(tiles):
        if last_progress + 3 < time.time():
            last_progress += 3
            print('%i/%i tiles' % (tn, len(tiles)))

        x, z = tile

        try:
            tile_img = Image.open(tiles_dir + '/%i,%i.png' % tile)
            out.paste(im=tile_img,
                      box=((x - min_x) * tile_size,
                           (z - min_z) * tile_size))#,
                      # mask=tile_img if tile_img.hasAlphaChannel() else None)
        except:
            print('failed at', tile, tiles_dir + '/%i,%i.png' % tile)
            # continue
            raise

    print('saving image as', img_path)

    # out.thumbnail((256, 256))#, Image.NEAREST)
    out.save(img_path, 'PNG')

if __name__ == '__main__':
    try:
        img_path, tiles_dir = sys.argv[1:3]
    except ValueError:
        print('Args: <image path> <tiles dir>')
    else:
        stitch_all(img_path, tiles_dir)
