use biomes::BIOME_COLOR_TABLE;
use ccnatural::{
    get_naturality_color, Naturality, CCNATURAL_COLORS_BLOCK_BIOME, CCNATURAL_COLORS_BLOCK_DEFAULT,
};
use std::u16;
use tile::{is_empty as is_empty_option, KeysMap, NamesVec};

fn is_empty(column: &[u8], keys: &KeysMap, _names: &NamesVec) -> bool {
    is_empty_option(column, Some(keys))
}

fn is_water(column: &[u8], keys: &KeysMap, _names: &NamesVec) -> bool {
    let block_nr = (column[1] as u16) << 8 | (column[2] as u16);
    return block_nr == *keys.get("minecraft:water[level=0]").unwrap_or(&u16::MAX);
}

pub fn biome(column: &[u8], keys: &KeysMap, names: &NamesVec) -> u32 {
    if is_empty(column, keys, names) {
        return 0;
    }
    let b = column[16];
    BIOME_COLOR_TABLE[b as usize]
}

pub fn naturality(column: &[u8], keys: &KeysMap, names: &NamesVec) -> u32 {
    if is_empty(column, keys, names) {
        return 0;
    }
    let biome = column[16];

    let mut final_naturality = None;

    for offset in 0..4 {
        let block_nr = (column[1 + offset * 4] as u16) << 8 | (column[2 + offset * 4] as u16);
        if block_nr != 0 {
            let block_name_full = &names[block_nr as usize];
            if block_name_full == "?UNKNOWN_BLOCK?" || block_name_full.ends_with(":air") {
                continue;
            }
            let block_name_prefixed = block_name_full.split("[").next().unwrap();
            let block_name_stem = block_name_prefixed.rsplit(":").next().unwrap();

            let block_naturality = CCNATURAL_COLORS_BLOCK_BIOME
                .get(&(block_name_stem, biome))
                .or_else(|| CCNATURAL_COLORS_BLOCK_DEFAULT.get(&block_name_stem))
                .unwrap_or(&Naturality::Unknown);
            if final_naturality.is_none() || final_naturality.unwrap() < *block_naturality {
                final_naturality = Some(*block_naturality);
            }
        }
    }

    final_naturality.map(|n| get_naturality_color(&n)).unwrap_or(0)
}

const S_WATER: u32 = 0xff_ff_c5_a6; // #a6c5ff
const S_LAND: u32 = 0xff_dc_e4_e7; // #e7e4dc

pub fn simple(column: &[u8], keys: &KeysMap, names: &NamesVec) -> u32 {
    if is_empty(column, keys, names) {
        return 0;
    }
    if is_water(column, keys, names) {
        return S_WATER;
    }
    return S_LAND;
}

pub fn light(column: &[u8], keys: &KeysMap, names: &NamesVec) -> u32 {
    if is_empty(column, keys, names) {
        return 0;
    }
    let bl = column[3] & 0xf;
    rgb(bl * 17, bl * 17, bl * 17)
}

pub fn height_bw(column: &[u8], keys: &KeysMap, names: &NamesVec) -> u32 {
    if is_empty(column, keys, names) {
        return 0;
    }
    let mut h = column[0];
    if h == 0 {
        h = 255 // 0 = 256 = actually at height limit
    }
    rgb(h, h, h)
}

const BLACK: u32 = 0xff_00_00_00;
const WHITE: u32 = 0xff_ff_ff_ff;
const SKY_COLOR: u32 = 0xff_88_00_88; // #880088 pink
const MTN_COLOR: u32 = 0xff_32_6e_9f; // #9f6e32 brown
const MID_COLOR: u32 = 0xff_00_ff_ff; // #ffff00 yellow
const COAST_COLOR: u32 = 0xff_00_b6_00; // #00b600 dark green
const SEA_COLOR: u32 = 0xff_ff_d9_00; // #00d9ff light blue

const HIGH_LEVEL: u8 = 240;
const MTN_LEVEL: u8 = 150;
const MID_LEVEL: u8 = 100;
const SEA_LEVEL: u8 = 64;

fn height(column: &[u8], keys: &KeysMap, names: &NamesVec) -> u32 {
    if is_empty(column, keys, names) {
        return 0; // unpopulated
    }

    if is_water(column, keys, names) {
        let sf = column[4]; // seafloor height
        if sf < SEA_LEVEL {
            interpolate(BLACK, SEA_COLOR, 0, SEA_LEVEL, sf)
        } else {
            SEA_COLOR
        }
    } else {
        // land

        // surface height
        let h = match column[0] {
            0 => 255, // wrapped around
            h => h,
        };

        if h < SEA_LEVEL {
            interpolate(BLACK, COAST_COLOR, 0, SEA_LEVEL, h)
        } else if h < MID_LEVEL {
            interpolate(COAST_COLOR, MID_COLOR, SEA_LEVEL, MID_LEVEL, h)
        } else if h < MTN_LEVEL {
            interpolate(MID_COLOR, MTN_COLOR, MID_LEVEL, MTN_LEVEL, h)
        } else if h < HIGH_LEVEL {
            interpolate(MTN_COLOR, WHITE, MTN_LEVEL, HIGH_LEVEL, h)
        } else {
            interpolate(WHITE, SKY_COLOR, HIGH_LEVEL, 255, h)
        }
    }
}

fn interpolate(col_start: u32, col_stop: u32, val_start: u8, val_stop: u8, val: u8) -> u32 {
    let r_st = col_start & 0xff;
    let g_st = col_start >> 8 & 0xff;
    let b_st = col_start >> 16 & 0xff;
    let r_sp = col_stop & 0xff;
    let g_sp = col_stop >> 8 & 0xff;
    let b_sp = col_stop >> 16 & 0xff;
    rgb(
        interpolate_color_component(r_st, r_sp, val_start, val_stop, val),
        interpolate_color_component(g_st, g_sp, val_start, val_stop, val),
        interpolate_color_component(b_st, b_sp, val_start, val_stop, val),
    )
}

fn interpolate_color_component(
    col_start: u32,
    col_stop: u32,
    val_start: u8,
    val_stop: u8,
    val: u8,
) -> u8 {
    let diff_start = val - val_start;
    let diff_stop = val_stop - val;
    let val_diff = val_stop - val_start;
    ((col_start * diff_stop as u32 + col_stop * diff_start as u32) / val_diff as u32) as u8
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    0xff000000 | ((b as u32) << 16) | ((g as u32) << 8) | (r as u32)
}

#[derive(Debug)]
pub enum Colorizer {
    Biome,
    Height,
    HeightBW,
    Light,
    Simple,
    Naturality,
}

impl Colorizer {
    pub fn get_column_color_fn(&self) -> Box<Fn(&[u8], &KeysMap, &NamesVec) -> u32> {
        Box::new(match *self {
            Colorizer::Biome => biome,
            Colorizer::Height => height,
            Colorizer::HeightBW => height_bw,
            Colorizer::Light => light,
            Colorizer::Simple => simple,
            Colorizer::Naturality => naturality,
        })
    }
}
