import sys

biomes = [0x88ff00ff for _ in range(256)]

next(sys.stdin)

for line in sys.stdin:
    s_id, s_name, s_red, s_green, s_blue, s_type = line.strip().split(',')
    i_id, i_red, i_green, i_blue = map(int, (s_id, s_red, s_green, s_blue))
    color = 0xff000000 | i_blue << 16 | i_green << 8 | i_red
    biomes[i_id] = color

for color in biomes:
    print(hex(color)+',')
