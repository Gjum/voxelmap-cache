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

const SEA_LEVEL: u32 = 95;

fn height(column: &[u8]) -> u32 {
    if is_empty(column) {
        return 0; // unpopulated
    }

    // height
    let h = match column[0] {
        0 => 256, // wrapped around
        h => h as u32,
    };

    // TODO look at biome too
    let b = column[2];
    if b != 9 && b != 8 { // land
        if h < SEA_LEVEL { // dug out
            let c = h * 255 / SEA_LEVEL;
            rgb(0, c as u8, 0)
        } else { // normal terrain
            let c = (h - SEA_LEVEL) * 255 / (255 - SEA_LEVEL);
            rgb(c as u8, 255, c as u8)
        }
    } else { // water
        let d = h - column[4] as u32; // depth
        if d > SEA_LEVEL {
            rgb(0, 0, 127)
        } else {
            let c = 255 - d * 255 / SEA_LEVEL;
            rgb(0, c as u8, 255 - (c as u8) / 2)
        }
    }
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
