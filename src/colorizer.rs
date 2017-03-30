
pub fn simple(column: &[u8; 17]) -> u32 {
    if *column == [0_u8; 17] {
        return 0;
    }
    let b = column[0+2];
    if b == 8 || b == 9 {
        return rgba(166, 197, 255, 255); // water: #a6c5ff
    }
    return rgba(231, 228, 220, 255); // land: #e7e4dc
}

static BIOME_COLORS: [u32; 256] = [0x88ff88ff; 256];

pub fn biome(column: &[u8; 17]) -> u32 {
    if *column == [0_u8; 17] {
        return 0;
    }
    let b = column[16];
    BIOME_COLORS[b as usize]
}

pub fn lightmap(column: &[u8; 17]) -> u32 {
    if *column == [0_u8; 17] {
        return 0;
    }
    if column[0] < 50 {
        return rgba(0, 0, 0, 255);
    }
    let bl = column[3] & 0xf;
    rgba(bl * 17, bl * 17, bl * 17, 255)
}

pub fn heightmap_grayscale(column: &[u8; 17]) -> u32 {
    if *column == [0_u8; 17] {
        return 0;
    }
    let h = column[0];
    rgba(h, h, h, 255)
}

const SEA_LEVEL: u32 = 95;

fn heightmap(column: &[u8; 17]) -> u32 {
    let h = column[0] as u32; // height

    if h == 0 { // unpopulated
        return rgba(0, 0, 0, 0);
    }

    // TODO look at biome too
    let b = column[2];
    return if b != 9 && b != 8 { // land
        if h < SEA_LEVEL { // dug out
            let c = h * 255 / SEA_LEVEL;
            rgba(0, c as u8, 0, 255)
        } else { // normal terrain
            let c = (h - SEA_LEVEL) * 255 / (255 - SEA_LEVEL);
            rgba(c as u8, 255, c as u8, 255)
        }
    } else {
        let d = h - column[4] as u32; // depth
        if d > SEA_LEVEL {
            return rgba(0, 0, 127, 255);
        }
        let c = 255 - d * 255 / SEA_LEVEL;
        rgba(0, c as u8, 255 - (c as u8) / 2, 255)
    }
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (r as u32)
    | ((g as u32) << 8)
    | ((b as u32) << 16)
    | ((a as u32) << 24)
}

#[derive(Debug)]
pub enum Colorizer {
    Biome { todo_colors: u8 },
    Height,
    Light,
    Simple,
    Terrain,
    Unknown,
}

impl Colorizer {
    pub fn column_color_fn(&self) -> Box<Fn(&[u8; 17]) -> u32> {
        Box::new(match *self {
            Colorizer::Simple => simple,
            Colorizer::Light => lightmap,
            Colorizer::Biome {..} => biome,
            Colorizer::Height => heightmap,
            Colorizer::Terrain => simple, // XXX
            Colorizer::Unknown => simple, // XXX
        })
    }
}
