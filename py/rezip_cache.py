"""
Sometimes, Rust's zip reader can't open some region cache .zip files.
This script re-zips every region in a way that Rust can read them.
"""
import os
import sys
import time
from zipfile import ZipFile

def main():
    try:
        target_dir, source_dir = sys.argv[1:3]
    except ValueError:
        print('Args: <target dir> <source dir>')
    else:
        rezip_all(target_dir, source_dir)


def rezip_all(target_dir, source_dir):
    os.makedirs(target_dir, exist_ok=True)

    regions = [tuple(map(int, region[:-4].split(',')))
             for region in os.listdir(source_dir)
             if region[-4:] == '.zip']

    print('re-zipping', len(regions), 'regions ...')

    first_progress = last_progress = time.time()
    for rn, pos in enumerate(regions):
        if last_progress + 3 < time.time():
            last_progress += 3
            time_left = (time.time() - first_progress) / rn * (len(regions) - rn)
            print('re-zip: %i/%i regions' % (rn, len(regions)),
                '%i:%02i left' % (int(time_left / 60), int(time_left % 60)))

        with ZipFile(source_dir + '/%s,%s.zip' % pos).open('data') as f:
            data = f.read()

        zf = ZipFile(target_dir + '/%s,%s.zip' % pos, 'w')
        zf.writestr('data', data)
        zf.close()

    print('Done, new cache is at', target_dir)


if __name__ == '__main__':
    main()
