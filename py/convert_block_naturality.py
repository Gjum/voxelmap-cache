# Converts data/blockcounts-categories.tsv into
# CCNATURAL_COLORS_BLOCK_BIOME and CCNATURAL_COLORS_BLOCK_DEFAULT

import sys
from collections import defaultdict

# read tsv

blocks = defaultdict(lambda: defaultdict(list))  # block -> category -> biomes

sys.stdin.readline()  # skip header

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    biomeid, biome, block, category, *_ = line.split('\t')
    if category == 'x':
        category = 'unknown'
    biomeid = int(biomeid)

    blocks[block][category].append(biomeid)

# calculate primary category

# block -> category (with most biomes)
primcats = {
    block: max(cats.items(), key=lambda cb: len(cb[1]))[0]
    for block, cats in blocks.items()
}

# print primary category

print(*sorted(
    f'("{bl}", Naturality::{cat.capitalize()}),'
    for (bl, cat) in primcats.items()
), sep='\n')

print("""\
    ].iter().map(|(n,c)| (*n, *c)));

    pub static ref CCNATURAL_COLORS_BLOCK_BIOME: HashMap<(&'static str, u8), Naturality> = HashMap::from_iter([\
""")

# print non-primary block+biome to category mappings

print(*sorted(
    f'(("{bl}", {bi}), Naturality::{cat.capitalize()}),'
    for (bl, cats) in blocks.items()
    for (cat, bis) in cats.items()
    for bi in bis
    if primcats[bl] != cat
), sep='\n')

print("""\
    ].iter().map(|((n,b),c)| ((*n, *b), *c)));
}\
""")
