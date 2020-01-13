"""
python py/convert_biomes_amidst.py --rs --download > src/biomes.rs
python py/convert_biomes_amidst.py --rs < biomes_amidst.csv > src/biomes.rs

Gets the color info from the crbednarz/AMIDST Github repo (--download flag)
or from a csv file, piped into stdin (format: id,name,red,green,blue,type).

If --rs is supplied, prints header and footer of the color array,
so the output is a valid Rust file as required for src/biomes.rs.
"""
import sys

biome_colors = [0x88ff00ff for _ in range(256)]

if '--download' in sys.argv[1:]:
    url = 'https://raw.githubusercontent.com/crbednarz/AMIDST/master/src/amidst/minecraft/Biome.java'
    print('// Colors taken from', url)

    import re
    from urllib import request

    # public static final Biome frozenRiverM         = new Biome("Frozen River M",           139, Util.makeColor(160, 160, 255));
    biome_re = re.compile(r'new Biome\("[^"]+",[ \t]*([0-9]+),[ \t]*Util.makeColor\(([0-9]+),[ \t]*([0-9]+),[ \t]*([0-9]+)\)')

    data = request.urlopen(url).read().decode()

    for match in biome_re.finditer(data):
        i_id, i_red, i_green, i_blue = map(int, match.groups())
        color = 0xff000000 | i_blue << 16 | i_green << 8 | i_red
        biome_colors[i_id] = color

else:  # assume stdin is piped .csv
    # skip header: id,name,red,green,blue,type
    next(sys.stdin)

    for line in sys.stdin:
        s_id, s_name, s_red, s_green, s_blue, s_type = line.strip().split(',')
        i_id, i_red, i_green, i_blue = map(int, (s_id, s_red, s_green, s_blue))
        color = 0xff000000 | i_blue << 16 | i_green << 8 | i_red
        biome_colors[i_id] = color

if '--rs' in sys.argv[1:]:
    print('// Created using py/convert_biomes_amidst.py', *sys.argv[1:])
    print('pub const BIOME_COLOR_TABLE: [u32; 256] = [')

for color in biome_colors:
    print(hex(color)+',')

if '--rs' in sys.argv[1:]:
    print('];')
