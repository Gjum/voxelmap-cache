"""
NOTE: This file only serves as an example now.
Do not use it, it is outdated and does not work correctly.
Instead, use the `merge_all` Rust program.

python3 merge_all.py <out dir> <tagged contrib dir>
Merges all tagged tile cache files from <tagged contrib dir>
into a single cache in <out dir> which then can be rendered using VoxelMap.

The files in <tagged contrib dir> are in the format used by extract_tiles.py.
"""
import os
import sys
import time
from collections import defaultdict
from gzip import compress
from zipfile import BadZipFile, ZipFile, ZIP_DEFLATED

def serialize_key(key):
    s = b''
    # the trailing \r\n might be (un)necessary, anyway it's in voxelmap's key files
    for v, k in key.items():
        s += b'%i %s\r\n' % (k, v)
    return s

def merge_all(out_dir, in_dir):
    os.makedirs(out_dir, exist_ok=True)

    named_tile_paths = defaultdict(list)
    num_total_tiles = 0
    for filename in os.listdir(in_dir):
        if filename[-4:] == '.zip':
            pos_split = filename[:-4].split(',', 2)[:2]
            rx, rz = map(int, pos_split)
            pos = ','.join(pos_split)
            tile_path = in_dir + '/' + filename
            mtime = os.path.getmtime(tile_path)
            named_tile_paths[pos].append((mtime, tile_path))
            num_total_tiles += 1

    last_progress = first_progress = time.time()
    tiles_merged = -1
    skipped_tags = set()  # XXX

    for tile_pos, tile_contribs in named_tile_paths.items():
        unset_chunks = set(range(256))
        chunk_mtimes = [-1 for _ in range(256)]  # TODO read from file generated during previous merge

        out_data = bytearray(256*256*17)
        mv_out = memoryview(out_data)
        out_key = {}  # string -> index (inverse of key file)
        next_key = 1  # voxelmap keys start at 1

        for mtime, tile_path in reversed(sorted(tile_contribs)):
            tiles_merged += 1
            if not unset_chunks:
                continue

            if last_progress + 3 < time.time():
                last_progress += 3
                time_left = (time.time() - first_progress) / tiles_merged * (num_total_tiles - tiles_merged)
                print('  merge tagged: %i/%i tiles' % (tiles_merged, num_total_tiles),
                    '%i:%02i left' % (int(time_left / 60), int(time_left % 60)))

            try:
                zip_file = ZipFile(tile_path)
            except BadZipFile:
                print('# bad zip file', tile_path)
                continue

            in_data = zip_file.open('data').read()
            mv_in = memoryview(in_data)

            if len(zip_file.filelist) > 1:
                key_file = zip_file.open('key')
                in_key = { int(k): v for k, v in (l.split() for l in key_file.read().split(b'\r\n') if l) }
                if not in_key:
                    print('# skipping empty key', tile_path)
                    continue
            else:
                # TODO convert keyless to keyed
                # in_key = ...

                tag = tile_path.split(',')[2][:-4]
                if tag not in skipped_tags:
                    skipped_tags.add(tag)
                    print('# skipping old unkeyed format', tag)

                continue

            # merge in_keys into out_keys, produce in->out key mapping
            current_key_map = [0] * (1 + max(k for k in in_key.keys()))
            for in_id, str_id in in_key:
                out_id = out_key.get(str_id)
                if out_id is None:
                    out_id = out_key[str_id] = next_key
                    next_key += 1
                current_key_map[in_id] = out_id

            for chunk_i in unset_chunks.copy():
                chunk_x = chunk_i % 16
                chunk_z = chunk_i // 16
                chunk_off = 16 * (chunk_x + 256 * chunk_z)

                height_first_block = mv_in[chunk_off * 17]
                chunk_first_block = mv_in[chunk_off * 17 + 1] << 8 | mv_in[chunk_off * 17 + 2]
                if height_first_block == 0 and 'minecraft:air' == in_key[chunk_first_block]:
                    continue  # empty chunk

                # copy chunk, block by block
                for z in range(16):
                    for x in range(16):
                        start = 17 * (x + 256 * z + chunk_off)
                        # copy everything, then change just the 4 blocks
                        mv_out[start:start + 17] = mv_in[start:start + 17]
                        for i in range(1, 1+16, 4):
                            block = mv_in[i] << 8 | mv_in[i + 1]
                            new_block = current_key_map[block]
                            mv_out[i] = block >> 8
                            mv_out[i+1] = block & 0xff

                unset_chunks.remove(chunk_i)
                chunk_mtimes[chunk_i] = mtime

        if out_key == {}:
            continue

        zf = ZipFile(out_dir + '/%s.zip' % tile_pos, 'w', compression=ZIP_DEFLATED)
        zf.writestr('data', out_data)
        zf.writestr('key', serialize_key(out_key))
        zf.close()
        zip_file.close()

        #with open(out_dir + '/%s_chunk-times.gz' % tile_pos, 'wb') as f:
        #    f.write(compress(','.join(map(str, chunk_mtimes)).encode()))


if __name__ == '__main__':
    try:
        out_dir, in_dir = sys.argv[1:]
    except ValueError:
        print('Args: <out dir> <tagged contrib dir>')
    else:
        merge_all(out_dir, in_dir)
