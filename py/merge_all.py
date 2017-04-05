"""
python3 merge_all.py <out dir> <tagged contrib dir>

Merges all tagged region cache files from <tagged contrib dir>
into a single cache in <out dir> which then can be rendered using VoxelMap.

Also generates <x>,<z>_chunk-times.gz for each region, containing a
gzip-compressed, comma-separated list of chunk modification timestamps.
These are the mtimes of the chunk's region.

The files in <tagged contrib dir> are in the format used by extract_regions.py.
"""
import os
import sys
import time
from collections import defaultdict
from gzip import compress
from zipfile import BadZipFile, ZipFile


def merge_all(out_dir, in_dir):
    os.makedirs(out_dir, exist_ok=True)

    in_files = os.listdir(in_dir)
    named_region_paths = defaultdict(list)
    for filename in in_files:
        if filename[-4:] == '.zip':
            pos_split = filename.split(',', 2)[:2]
            rx, rz = map(int, pos_split)
            if not -20 <= rx < 20 or not -20 <= rz < 20:
                continue  # outside the map, ignore
            pos = ','.join(pos_split)
            region_path = in_dir + '/' + filename
            mtime = os.path.getmtime(region_path)
            named_region_paths[pos].append((mtime, region_path))

    last_progress = first_progress = time.time()
    rn = -1

    for region_name, region_contribs in named_region_paths.items():
        unset_chunks = set(range(256))
        chunk_mtimes = [-1 for _ in range(256)]

        out_data = bytearray(256*256*17)
        mv_out = memoryview(out_data)

        for mtime, region_path in reversed(sorted(region_contribs)):
            rn += 1
            if not unset_chunks:
                continue

            if last_progress + 3 < time.time():
                last_progress += 3
                time_left = (time.time() - first_progress) / rn * (len(in_files) - rn)
                print('  merge tagged: %i/%i regions' % (rn, len(in_files)),
                    '%i:%02i left' % (int(time_left / 60), int(time_left % 60)))

            try:
                file = ZipFile(region_path).open('data')
                in_data = file.read()
            except BadZipFile:
                print('# bad zip file', region_path)
                continue

            mv_in = memoryview(in_data)

            for chunk_i in unset_chunks.copy():
                chunk_x = chunk_i % 16
                chunk_z = chunk_i // 16
                chunk_off = 16 * (chunk_x + 256 * chunk_z)

                if mv_in[chunk_off * 17] == 0:
                    continue  # erroneous/empty chunk

                for z in range(16):
                    # copy a line of the chunk
                    start = 17 * (256 * z + chunk_off)
                    end = start + 16 * 17
                    mv_out[start:end] = mv_in[start:end]

                unset_chunks.remove(chunk_i)
                chunk_mtimes[chunk_i] = mtime

        zf = ZipFile(out_dir + '/%s.zip' % region_name, 'w')
        zf.writestr('data', out_data)
        zf.close()

        with open(out_dir + '/%s_chunk-times.gz' % region_name, 'wb') as f:
            f.write(compress(','.join(map(str, chunk_mtimes)).encode()))


if __name__ == '__main__':
    try:
        out_dir, in_dir = sys.argv[1:]
    except ValueError:
        print('Args: <out dir> <tagged contrib dir>')
    else:
        merge_all(out_dir, in_dir)
