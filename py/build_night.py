"""
Example: python3 build_night.py /tiles/night/z0 /tiles/terrain/z0 /tiles/light/z0
or, when using bash: python3 build_night.py /tiles/{night,terrain,light}/z0
"""
import os
import sys
import time
from PIL import Image

darkness = .3  # how dark the night areas are, 0 (black) to 1 (day)

def make_night(day_path, light_path):
    day = Image.open(day_path)
    light = Image.open(light_path)

    black = Image.new(day.mode, day.size, 'black')
    night = Image.blend(black, day, darkness)

    night.paste(day, (0, 0), light.convert('L'))

    return night

def make_night_all(night_dir, day_dir, light_dir):
    os.makedirs(night_dir, exist_ok=True)

    tiles = [tuple(map(int, tile[:-4].split(',')))
             for tile in os.listdir(day_dir)
             if tile[-4:] == '.png']

    print('converting', len(tiles), 'tiles to night ...')

    first_progress = last_progress = time.time()
    for tn, pos in enumerate(tiles):
        if last_progress + 3 < time.time():
            last_progress += 3
            time_left = (time.time() - first_progress) / tn * (len(tiles) - tn)
            print('night: %i/%i tiles' % (tn, len(tiles)),
                '%i:%02i left' % (int(time_left / 60), int(time_left % 60)))

        night = make_night(
            day_dir   + '/%s,%s.png' % pos,
            light_dir + '/%s,%s.png' % pos)

        night.save(night_dir + '/%s,%s.png' % pos, 'PNG')

    print('Done, night is at', night_dir)


if __name__ == '__main__':
    try:
        night_dir, day_dir, light_dir = sys.argv[1:4]
    except ValueError:
        print('Args: <night dir> <day dir> <light dir>')
    else:
        make_night_all(night_dir, day_dir, light_dir)
