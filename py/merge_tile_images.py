"""
Combine two or more sets of terrain tiles, treating black pixels as transparent.
Example: python3 merge_tile_images.py /tiles/out/ /tiles/bottom/ /tiles/middle/ /tiles/top/
"""
import os
import sys
import time
from PIL import Image
import numpy as np


# uses img.alpha_composite()
def merge_one_pil(pos, out_dir, in_dirs):
    tile_filename = '/%s,%s.png' % pos
    out_img = Image.new("RGBA", (256,256), (0,0,0,0))

    for in_dir in in_dirs:
        try:
            in_img = Image.open(in_dir + tile_filename).convert("RGBA")
        except FileNotFoundError:
            continue

        img_arr = np.array(in_img)

        black_areas = img_arr[:,:,0] | img_arr[:,:,1] | img_arr[:,:,2] == 0
        img_arr[black_areas] = [0,0,0, 0]
        transparent_img = Image.fromarray(img_arr)
        out_img.alpha_composite(transparent_img)

    out_img.save(out_dir + tile_filename, 'PNG')


# uses numpy exclusively
def merge_one_np(pos, out_dir, in_dirs):
    tile_filename = '/%s,%s.png' % pos
    out_arr = np.zeros((256,256,4), dtype='uint8')

    for in_dir in in_dirs:
        try:
            in_img = Image.open(in_dir + tile_filename).convert("RGBA")
        except FileNotFoundError:
            continue

        img_arr = np.array(in_img)

        present_areas = img_arr[:,:,0] | img_arr[:,:,1] | img_arr[:,:,2] != 0
        out_arr[present_areas] = img_arr[present_areas]

    out_img = Image.fromarray(out_arr)
    out_img.save(out_dir + tile_filename, 'PNG')


def merge_all(out_dir, in_dirs):
    os.makedirs(out_dir, exist_ok=True)

    tiles = set(
        tuple(map(int, filename[:-4].split(',')))
        for in_dir in in_dirs
        for filename in os.listdir(in_dir)
        if filename[-4:] == '.png' and ',' in filename)

    print('total', len(tiles), 'tile locations')

    first_progress = last_progress = time.time()
    for tn, pos in enumerate(tiles):
        if last_progress + 3 < time.time():
            last_progress = time.time()
            time_left = (time.time() - first_progress) / tn * (len(tiles) - tn)
            print('merge tile images: %i/%i tiles' % (tn, len(tiles)),
                '%i:%02i left' % (int(time_left / 60), int(time_left % 60)))

        # merge_one_pil(pos, out_dir, in_dirs)
        merge_one_np(pos, out_dir, in_dirs)

    print('Done, merged tile images are at ', out_dir)


if __name__ == '__main__':
    try:
        out_dir, *in_dirs = sys.argv[1:]
        if len(in_dirs) < 2: raise ValueError()
    except ValueError:
        print('Args: <out dir> <bottom dir> [...] <top dir>')
    else:
        merge_all(out_dir, in_dirs)
