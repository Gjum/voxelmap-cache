"""
Zoom out a tileset, combining 4 tiles into one, and shrinking it to the original tile size.
The tileset root must contain a directory named `z0`.
If given, the minimum zoom level must be negative (n < 0), default is -1.
This will create new directories z-1, z-2, ... next to z0, containing the zoomed-out tiles.
"""
import os
import sys
from PIL import Image


def main():
    try:
        tiles_root = sys.argv[1]
    except IndexError:
        print('Args: <tileset root path> [minimum zoom level = -1]')
        sys.exit(1)

    try:
        min_zoom = int(sys.argv[2])
    except ValueError:
        print('Args: <tileset root path> [minimum zoom level = -1]')
        sys.exit(1)
    except IndexError:
        min_zoom = -1

    tiles_root += '/z%i'

    for current_zoom in range(-min_zoom):
        print('zooming', -current_zoom - 1,)
        stitch_all(tiles_root % (-current_zoom - 1), tiles_root % -current_zoom)


def stitch_four(size, x, z, out_path, in_path):
    """
    x,z are tile coords of the nw small tile
    size is the width of a small tile
    """
    nw_path = in_path + '/%i,%i.png' % (x, z)
    sw_path = in_path + '/%i,%i.png' % (x, z+1)
    ne_path = in_path + '/%i,%i.png' % (x+1, z)
    se_path = in_path + '/%i,%i.png' % (x+1, z+1)

    out = Image.new('RGBA', (2*size, 2*size))

    try:
        if os.path.isfile(nw_path):
            out.paste(im=Image.open(nw_path), box=(0, 0))
    except Exception as e:
        print('Exception at', nw_path, e)
    try:
        if os.path.isfile(sw_path):
            out.paste(im=Image.open(sw_path), box=(0, size))
    except Exception as e:
        print('Exception at', sw_path, e)
    try:
        if os.path.isfile(ne_path):
            out.paste(im=Image.open(ne_path), box=(size, 0))
    except Exception as e:
        print('Exception at', ne_path, e)
    try:
        if os.path.isfile(se_path):
            out.paste(im=Image.open(se_path), box=(size, size))
    except Exception as e:
        print('Exception at', se_path, e)

    out.thumbnail((256, 256))
    #out.thumbnail((256, 256), Image.NEAREST)
    out.save(out_path, 'PNG')

def stitch_all(out_path, in_path):
    os.makedirs(out_path, exist_ok=True)

    tiles = [tuple(map(int, region[:-4].split(',')))
             for region in os.listdir(in_path)
             if region[-4:] == '.png']

    size = Image.open(in_path + '/%i,%i.png' % tiles[0]).size[0]

    min_x = min(x for x,y in tiles) // 2
    min_z = min(z for x,z in tiles) // 2
    max_x = max(x for x,y in tiles) // 2
    max_z = max(z for x,z in tiles) // 2

    for x in range(min_x, max_x+1):
        for z in range(min_z, max_z+1):
            out_tile = out_path + '/%i,%i.png' % (x, z)
            try:
                stitch_four(size, 2*x, 2*z, out_tile, in_path)
            except Exception as e:
                print('Exception at', x, z, e.__class__.__name__)

if __name__ == '__main__':
    main()
