use biomes::BIOME_COLOR_TABLE;

fn is_empty(column: &[u8]) -> bool {
    return column[1] == 0 && column[2] == 0 // block is air
        || column[0] > 0 && column[0] < 6; // inside bedrock (specific to Devoted)
}

pub fn biome(column: &[u8]) -> u32 {
    if is_empty(column) {
        return 0;
    }
    let b = column[16];
    BIOME_COLOR_TABLE[b as usize]
}

pub fn simple(column: &[u8]) -> u32 {
    if is_empty(column) {
        return 0;
    }
    let b = column[0+2];
    if b == 8 || b == 9 {
        return rgb(166, 197, 255); // water: #a6c5ff
    }
    return rgb(231, 228, 220); // land: #e7e4dc
}

pub fn light(column: &[u8]) -> u32 {
    if is_empty(column) {
        return 0;
    }
    let bl = column[3] & 0xf;
    rgb(bl * 17, bl * 17, bl * 17)
}

pub fn heightmap_grayscale(column: &[u8]) -> u32 {
    if is_empty(column) {
        return 0;
    }
    let h = column[0];
    rgb(h, h, h)
}

const       BLACK: u32 = 0xff_00_00_00;
const       WHITE: u32 = 0xff_ff_ff_ff;
const   MTN_COLOR: u32 = 0xff_57_8a_e1; // #e18a57
const   MID_COLOR: u32 = 0xff_00_ff_ff; // #ffff00
const COAST_COLOR: u32 = 0xff_00_b6_00; // #00b600
const   SEA_COLOR: u32 = 0xff_ff_d9_00; // #00d9ff

const MTN_LEVEL: u8 = 180;
const MID_LEVEL: u8 = 115;
const SEA_LEVEL: u8 = 95;

fn height(column: &[u8]) -> u32 {
    if is_empty(column) {
        return 0; // unpopulated
    }

    // surface height
    let h = match column[0] {
        0 => 255, // wrapped around
        h => h,
    };

    // seafloor height
    let sf = column[4];

    if sf == 0 { // land
        if h < SEA_LEVEL {
            interpolate(BLACK, COAST_COLOR, 0, SEA_LEVEL, h)
        } else if h < MID_LEVEL {
            interpolate(COAST_COLOR, MID_COLOR, SEA_LEVEL, MID_LEVEL, h)
        } else if h < MTN_LEVEL {
            interpolate(MID_COLOR, MTN_COLOR, MID_LEVEL, MTN_LEVEL, h)
        } else {
            interpolate(MTN_COLOR, WHITE, MTN_LEVEL, 255, h)
        }
    } else { // water
        if sf < SEA_LEVEL {
            interpolate(BLACK, SEA_COLOR, 0, SEA_LEVEL, sf)
        } else {
            SEA_COLOR
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
    rgb(interpolate_color_component(r_st, r_sp, val_start, val_stop, val),
        interpolate_color_component(g_st, g_sp, val_start, val_stop, val),
        interpolate_color_component(b_st, b_sp, val_start, val_stop, val))
}

fn interpolate_color_component(col_start: u32, col_stop: u32, val_start: u8, val_stop: u8, val: u8) -> u8 {
    let diff_start = val - val_start;
    let diff_stop = val_stop - val;
    let val_diff = val_stop - val_start;
    ( ( col_start * diff_stop as u32
      + col_stop * diff_start as u32
      ) / val_diff as u32
    ) as u8
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    0xff000000
        | ((b as u32) << 16)
        | ((g as u32) << 8)
        | (r as u32)
}

#[derive(Debug)]
pub enum Colorizer {
    Biome,
    Height,
    Light,
    Simple,
    Terrain,
}

impl Colorizer {
    pub fn column_color_fn(&self) -> Box<Fn(&[u8]) -> u32> {
        Box::new(match *self {
            Colorizer::Biome => biome,
            Colorizer::Height => height,
            Colorizer::Light => light,
            Colorizer::Simple => simple,
            Colorizer::Terrain => unimplemented!(),
        })
    }
}
