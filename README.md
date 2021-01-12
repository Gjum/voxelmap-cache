# voxelmap-cache

Utilities for [VoxelMap](https://www.planetminecraft.com/mod/zans-minimap/)
and [CivMap](https://github.com/Gjum/CivMap/)/[CCMap](https://ccmap.github.io/)
to merge map caches and render image tiles

## Installation

You will need [Rust](https://www.rust-lang.org/) Nightly,
for example using [rustup](https://rustup.rs/),
and [Python 3](https://www.python.org/downloads/)
with the [Pillow](https://pypi.org/project/Pillow/) package.

The following command line examples are written for Mac/Linux.
If you are using Windows, you can install [Git Bash](https://gitforwindows.org/#bash)
or use [Windows Subsystem for Linux](https://docs.microsoft.com/en-us/windows/wsl/install-win10).

## Updating the map with new data

Here's an overview. Each step is detailed below.

- get region caches from extracted archives using `extract_regions.py`
- merge all caches using `merge_caches`
- render terrain tiles using VoxelMap
- clean up any player-loaded chunks from the cache and terrain tiles using `cleanup.py`
- render other map tiles (simple/light/biome/height) using the custom CivMap renderer
- create zoomed-out tiles using `zoom.py` in each tileset directory
- optional: create a single image from all the tiles of one tileset

### Example Directory Structure

```
~/civmap-cc/
|- contrib/
|  |- player_2018-08-04.zip
|  |- player_2020-01-13.7z
|  ...
|- extracted/
|  |- -27,11,player_2020-01-13_overworld.zip
|  |- -27,11,player_2018-08-04_overworld.zip
|  ...
|-merged/
|  |- current -> 2020-01-13/
|  |- 2020-01-13/
|  |  |- -27,11.zip
|  ...
'- tiles/
   |- terrain/
   |  |- z0/
   |  |  |- -27,11.png
   |  |  ...
   |  |- z-1/
   |  ...
   |- height/
   ...
```

### extract_regions.py

Usage:

    python3 py/extract_regions.py [-v] <main_cache> <new_cache> [new_cache ...]

Takes a source directory containing voxelmap region caches (`<x>,<z>.zip`)
and creates hardlinks of them in the target directory,
tagged with the source directory's name.

Example:

```bash
mkdir contrib/player_2018-08-04/
cd contrib/player_2018-08-04/
unzip ../player_2018-08-04.zip
# ... do the same for all other contributions

python3 py/extract_regions.py extracted/ contrib/player_2018-08-04/ contrib/player_2020-01-13/ # ...
```

### Merge Caches

Compile with:

    cargo build --release --bin merge_caches

Usage:

    target/release/merge_caches [-q] [-t threads] [--between=<bounds>] <output-path> <cache-path>...

`cache-path` contains voxelmap caches in the format `<x>,<z>,<contrib-name>.zip`.

`output-path` should be an *empty* directory and will contain the merged cache.

Options:

    -q, --quiet         Do not output info messages.
    -t, --threads       Number of threads to use for parallel processing
    --between=<bounds>  Only merge tiles at least partially within this bounding box,
                        format: w,n,e,s [default: -99999,-99999,99999,99999]

Example:

```bash
# prepare empty directory
mkdir -p merged/2020-01-13/
rm merged/current # delete old symlink
ln -rs merged/2020-01-13/ merged/current

# merge ALL extracted contributions
# using `cargo run` compiles the program and then runs it, `--release` makes it run fast, `--bin merge_caches` selects this program only out of all the ones available
cargo run --release --bin merge_caches merged/current/ extracted/
```

### Rendering tiles using VoxelMap

- see also: [instructions at old VoxelMap-related project](https://github.com/MamiyaOtaru/anvilmapper/blob/0b1d5ff6bc4062c048645202f5b266f5f1288c2f/README.md#voxelmap-output-image-processor)
- create a singleplayer world (`WORLDNAME`),
  move outside of the world border (13000 blocks for CivClassic 2.0) plus render distance,
  so you don't overwrite the cache when you render it later
- copy/symlink the merged cache to `.minecraft/mods/VoxelMods/voxelMap/cache/WORLDNAME/overworld (dimension 0)/`
- stop Mincraft, put `Output Images:true` in `.minecraft/mods/VoxelMods/voxelmap.properties`, start Minecraft
  (note that this line gets deleted when you start Minecraft, it only applies to the running game instance,
  so if you are repeating this step you will have to edit it again each time)
- ensure you have your chosen resource pack loaded, and it's daytime
  (`/gamerule doDaylightCycle false`, `/time set 6000`);
  you may want to enable GammaBright mod to show under water terrain
- load the singleplayer world, open VoxelMap, pan around until all of the map is rendered
- images will be created in `.minecraft/mods/VoxelMods/voxelMap/cache/WORLDNAME/overworld (dimension 0)/images/z1/<x>,<z>.png`
- copy them to `tiles/terrain/z0/` (CivMap uses different zoom numbers)
- run `cleanup.py` on them to remove the area you were standing in during rendering

### cleanup.py

Usage:

    python3 py/cleanup.py <region files directory> [-f]

Removes region files (cache, tile, chunk times) that are
outside the 13000 blocks world border of CivClassic 2.0.
(Edit the source code to change this radius.)

Without `-f`, only lists the files being removed.
With `-f`, removes them.

This can be used after in-game rendering (using VoxelMap)
to remove the tiles and cache files around the player, if it is
standing outside render distance of the world border.

Example:

```bash
# remove extraneous tiles before zooming out
# 1. list files to be deleted ("dry run")
python3 py/cleanup.py tiles/terrain/z0/
# 2. add -f flag after you've confirmed it won't eat your precious files
python3 py/cleanup.py tiles/terrain/z0/ -f
# also clean the cache for custom renderer
python3 py/cleanup.py merged/current/
```

### Custom Renderer

Turns cache tiles (`<x>,<z>.zip`) into tile images (`<x>,<z>.png`).

Available modes:

- simple: blue-gray water-land map
- light: grayscale block light values, used to generate the nightmap
- biome: color coded biomes, using [AMIDST color map][amidst-biomecolors]
- height: color coded block heights and water depths

Compile with:

    cargo build --release --bin render

Usage:

    target/release/render [-q] [-t threads] <cache> <output> (simple | light | biome | height)

`cache-path` contains voxelmap caches in the format `<x>,<z>.zip`,
for example the result of `merge_caches`.

`output-path` is a directory that will contain the rendered tiles.

Options:

    -q, --quiet         Do not output info messages.
    -t, --threads       Number of threads to use for parallel processing
    --between=<bounds>  Only render tiles at least partially within this bounding box,
                        format: w,n,e,s [default: -99999,-99999,99999,99999]

Example:

```bash
cargo run --release --bin render merged/current tiles/height/z0 height
```

### build_night.py

    python3 py/build_night.py tiles/night/z0 /tiles/terrain/z0 /tiles/light/z0

or, when using bash:

    python3 py/build_night.py tiles/{night,terrain,light}/z0

Combine `terrain` and `light` into `night` tiles.

### zoom.py

    python3 zoom.py <tileset root path> [minimum zoom level = -1]

Zoom out a tileset, combining 4 tiles into one, and shrinking it to the original tile size.
The tileset root must contain a directory named `z0`.
If given, the minimum zoom level must be negative (n < 0), default is -1.
This will create new directories `z-1`, `z-2`, ... next to `z0`, containing the zoomed-out tiles.

Example:

```bash
python3 py/zoom.py tiles/terrain/ -6
python3 py/zoom.py tiles/height/ -6
```

### image_from_tiles.py

    python3 image_from_tiles.py <image path> <tiles dir>

Combines all tiles in `<tiles dir>` into a single image.

Example:

```bash
# using z-3 results in 8x zoom (2^3 = 8)
python3 py/image_from_tiles.py terrain_8x_zoomed.png tiles/terrain/z-3
# you can also create full-scale images (1:1 pixel:block),
# but they may crash your system when opened in a typical image viewer.
python3 py/image_from_tiles.py simple_full_scale.png tiles/simple/z0
```

## Miscellaneous Utilities

### convert_biomes_amidst.py

    python py/convert_biomes_amidst.py --rs --download > src/biomes.rs
    python py/convert_biomes_amidst.py --rs < biomes_amidst.csv > src/biomes.rs

Converts the color info into source code, sourcing it
from the [crbednarz/AMIDST Github repo][amidst-biomecolors] (`--download` flag)
or from a csv file, piped into stdin (format: `id,name,red,green,blue,type`).

[amidst-biomecolors]: https://github.com/crbednarz/AMIDST/wiki/biomecolors

If `--rs` is supplied, prints header and footer of the color array,
so the output is a valid Rust file as required for `src/biomes.rs`.

### rezip_cache.py

Sometimes Rust's zip reader can't open some region cache .zip files.
This script re-zips every region in a way that Rust can read them,
keeping their timestamps intact.
