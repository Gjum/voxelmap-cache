"""
Sometimes Rust's zip reader can't open some region cache .zip files.
This script re-zips every region in a way that Rust can read them,
keeping their timestamps intact.
"""
import os
import sys
import time
from shutil import copystat, move
import zipfile as zf
from zipfile import ZipFile

compressions = dict(zip(
    ('bzip2'     , 'deflate'      , 'lzma'     , 'store'      ),
    (zf.ZIP_BZIP2, zf.ZIP_DEFLATED, zf.ZIP_LZMA, zf.ZIP_STORED)
))

def main():
    try:
        compression_name, target, source = sys.argv[1:4]
    except ValueError:
        print('Args: <compression method> <target dir> <source dir>')
        print('Available compression methods: ' + ' '.join(compressions.keys()))
    else:
        compression = compressions[compression_name]
        if source[-4:] == '.zip':
            rezip_one(target, source, compression)
        else: rezip_all(target, source, compression)


def rezip_all(target_dir, source_dir, compression=zf.ZIP_STORED):
    os.makedirs(target_dir, exist_ok=True)

    regions = [region for region in os.listdir(source_dir) if region[-4:] == '.zip']

    print('re-zipping', len(regions), 'regions ...')

    first_progress = last_progress = time.time()
    for rn, region in enumerate(regions):
        if last_progress + 3 < time.time():
            last_progress += 3
            time_left = (time.time() - first_progress) / rn * (len(regions) - rn)
            print('re-zip: %i/%i regions' % (rn, len(regions)),
                '%i:%02i left' % (int(time_left / 60), int(time_left % 60)))

        source_zip = source_dir + '/' + region
        target_zip = target_dir + '/' + region

        rezip_one(target_zip, source_zip, compression)

    print('Done, new cache is at', target_dir)


def rezip_one(target_zip, source_zip, compression):
        try:
            with ZipFile(source_zip).open('data') as f:
                data = f.read()
        except Exception as e:
            print('skipping zip file', source_zip, 'for error', e)
            return

        # write to intermediate file in case source_dir == target_dir
        zf = ZipFile(target_zip+'.new', 'w', compression=compression)
        zf.writestr('data', data)
        zf.close()

        copystat(source_zip, target_zip+'.new')
        move(target_zip+'.new', target_zip)


if __name__ == '__main__':
    main()
