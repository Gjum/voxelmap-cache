# voxelmap-cache

Utilities for VoxelMap and CivMap to merge and render region caches and map tiles

## Installation

- rust nightly, for example using `rustup`
- python 3, and the `pillow` package

## Updating the map with new data

- get region caches from extracted archives using `extract_regions.py`
- merge all caches using `merge_all.py`
- render terrain tiles using VoxelMap, [see below](#rendering-tiles-using-voxelmap)
- render other map tiles using the Rust tool, [see below](#custom-renderer)
- create zoomed-out tiles using `zoom.py` in each tileset directory
- add+commit+push tiles

### Rendering tiles using VoxelMap
- [see also](https://github.com/MamiyaOtaru/anvilmapper/blob/0b1d5ff6bc4062c048645202f5b266f5f1288c2f/README.md#voxelmap-output-image-processor)
- create a singleplayer world (`WORLDNAME`),
  move outside of the world border (5000 blocks for DevotedMC 3.0) plus render distance,
  so you don't overwrite the cache when you render it later
- copy/symlink the merged cache to `.minecraft/mods/VoxelMods/voxelMap/cache/WORLDNAME/Overworld (dimension 0)/`
- stop Mincraft, put `Output Images:true` in `.minecraft/mods/VoxelMods/voxelmap.properties`, start Minecraft
  (note that this line gets deleted when you start Minecraft, it only applies to the running game instance)
- ensure you have your chosen resource pack loaded and it's daytime
  (`/gamerule doDaylightCycle false`, `/time set 6000`)
- load the singleplayer world, pan around until all of the map is rendered
- images will be created in `.minecraft/mods/VoxelMods/voxelMap/cache/WORLDNAME/Overworld (dimension 0)/images/z1/`
- copy them to `/tiles/terrain/z0/` (CivMap uses different zoom numbers)
- run `cleanup.py` on them to remove the area you were standing in during rendering

## Scripts

### Custom Renderer

Compile with:

    cargo build --release

Usage:

    target/release/render [-q] [-t threads] <cache> <output> (simple | light | biome | height)

- simple: blue-gray water-land map
- light: grayscale block light values, used to generate the nightmap
- biome: color coded biomes
- height: color coded block heights and water depths

To output tiles instead of one full image, the `<output>` path has to contain `{tile}` or `{x}` and `{z}`.
They will be replaced with the tile coordinates for each tile image.
The tiles will be 256x256 pixels in size, the same as VoxelMap's tiles.

### py/build_night.py

    python3 build_night.py /tiles/night/z0 /tiles/terrain/z0 /tiles/light/z0

or, when using bash:

    python3 build_night.py /tiles/{night,terrain,light}/z0

Combine `terrain` and `light` into `night` tiles.

### py/cleanup.py

    python3 cleanup.py <region files directory> [-f]

Removes region files (cache, tile, chunk times) that are
outside the 5000 blocks world border of DevotedMC 3.0.

Without `-f`, only lists the files being removed.
With `-f`, removes them.

### py/convert_biomes_amidst.py

    python py/convert_biomes_amidst.py --rs --download > src/biomes.rs
    python py/convert_biomes_amidst.py --rs < biomes_amidst.csv > src/biomes.rs

Gets the color info from the skiphs/AMIDST Github repo (`--download` flag)
or from a csv file, piped into stdin (format: `id,name,red,green,blue,type`).

If `--rs` is supplied, prints header and footer of the color array,
so the output is a valid Rust file as required for src/biomes.rs.

### py/extract_regions.py

Takes a source directory containing voxelmap region caches (`<x>,<z>.zip`)
and creates hardlinks of them in the target directory,
tagged with the source directory's name.

### py/image_from_tiles.py

    python3 image_from_tiles.py <image path> <tiles dir>

Combines all tiles in `<tiles dir>` into a single image.

### py/merge_all.py

    python3 merge_all.py <out dir> <tagged contrib dir>

Merges all tagged region cache files from `<tagged contrib dir>`
into a single cache in `<out dir>` which then can be rendered using VoxelMap.

Also generates `<x>,<z>_chunk-times.gz` for each region, containing a
gzip-compressed, comma-separated list of chunk modification timestamps.
These are the mtimes of the chunk's region.

### py/rezip_cache.py

Sometimes Rust's zip reader can't open some region cache .zip files.
This script re-zips every region in a way that Rust can read them,
keeping their timestamps intact.

### py/zoom.py

    python3 zoom.py <tileset root path> [minimum zoom level = -1]

Zoom out a tileset, combining 4 tiles into one, and shrinking it to the original tile size.
The tileset root must contain a directory named `z0`.
If given, the minimum zoom level must be negative (n < 0), default is -1.
This will create new directories `z-1`, `z-2`, ... next to `z0`, containing the zoomed-out tiles.
