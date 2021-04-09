use crate::biomes::BIOME_COLOR_TABLE;
use crate::ccnatural::{
    get_naturality_color, Naturality, CCNATURAL_COLORS_BLOCK_BIOME, CCNATURAL_COLORS_BLOCK_DEFAULT,
};
use crate::tile::Tile;
use std::convert::TryInto;
use std::u16;

pub fn colorize_biome(tile: &Tile, column_nr: usize) -> u32 {
    if tile.is_col_empty(column_nr) {
        return 0;
    }
    let b = tile.get_biome_id(column_nr);
    BIOME_COLOR_TABLE[b as usize]
}

pub fn colorize_naturality(tile: &Tile, column_nr: usize) -> u32 {
    if tile.is_col_empty(column_nr) {
        return 0;
    }
    let biome: u8 = tile.get_biome_id(column_nr).try_into().unwrap();

    let mut final_naturality = None;

    let steps_block_getters: Vec<fn(&Tile, usize) -> u16> = vec![
        Tile::get_blockstate,
        Tile::get_ocean_floor_blockstate,
        Tile::get_transparent_blockstate,
        Tile::get_foliage_blockstate,
    ];
    for get_block_nr in steps_block_getters {
        let block_nr = get_block_nr(tile, column_nr);
        if block_nr != 0 {
            let block_name_full = &tile.names[block_nr as usize];
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

    final_naturality
        .map(|n| get_naturality_color(&n))
        .unwrap_or(0)
}

const S_WATER: u32 = 0xff_ff_c5_a6; // #a6c5ff
const S_LAND: u32 = 0xff_dc_e4_e7; // #e7e4dc

pub fn colorize_simple(tile: &Tile, column_nr: usize) -> u32 {
    if tile.is_col_empty(column_nr) {
        return 0;
    }
    if is_water(tile, column_nr) {
        return S_WATER;
    }
    return S_LAND;
}

pub fn colorize_light(tile: &Tile, column_nr: usize) -> u32 {
    if tile.is_col_empty(column_nr) {
        return 0;
    }
    let bl = tile.get_light(column_nr) & 0xf;
    rgb(bl * 17, bl * 17, bl * 17)
}

pub fn colorize_height_bw(tile: &Tile, column_nr: usize) -> u32 {
    if tile.is_col_empty(column_nr) {
        return 0;
    }
    let mut h = tile.get_height(column_nr);
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

pub fn colorize_height(tile: &Tile, column_nr: usize) -> u32 {
    if tile.is_col_empty(column_nr) {
        return 0; // unpopulated
    }
    if is_water(tile, column_nr) {
        get_sea_color(tile.get_ocean_floor_height(column_nr))
    } else {
        get_land_color(tile.get_height(column_nr))
    }
}

pub fn get_sea_color(ocean_floor_height: u8) -> u32 {
    if ocean_floor_height < SEA_LEVEL {
        interpolate(BLACK, SEA_COLOR, 0, SEA_LEVEL, ocean_floor_height)
    } else {
        SEA_COLOR
    }
}

pub fn get_land_color(surface_height: u8) -> u32 {
    let h = match surface_height {
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

fn is_water(tile: &Tile, column_nr: usize) -> bool {
    let block_nr = tile.get_blockstate(column_nr);
    return block_nr
        == *tile
            .keys
            .get("minecraft:water[level=0]")
            .unwrap_or(&u16::MAX);
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
    pub fn get_column_color_fn(&self) -> Box<dyn Fn(&Tile, usize) -> u32> {
        Box::new(match *self {
            Colorizer::Biome => colorize_biome,
            Colorizer::Height => colorize_height,
            Colorizer::HeightBW => colorize_height_bw,
            Colorizer::Light => colorize_light,
            Colorizer::Naturality => colorize_naturality,
            Colorizer::Simple => colorize_simple,
        })
    }
}
