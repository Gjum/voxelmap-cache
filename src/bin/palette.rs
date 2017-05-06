extern crate lodepng;
extern crate voxelmap_cache;

use std::time;
use voxelmap_cache::colorizer::*;

fn main() {
    let height = Colorizer::Height.column_color_fn();

    let unix_time = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap().as_secs();
    let img_path = std::env::args().skip(1).next().unwrap_or("palette_{t}.png".to_string())
        .replace("{t}", &unix_time.to_string());

    let mut pixbuf = [0_u32; 100*256];
    for h in 1..257_usize {
        let column_land: &[u8] = &[
            h as u8, 0, 1, 0,
            0, 0, 0, 0,
            0, 0, 0, 0,
            0, 0, 0, 0,
            0
        ];
        let color_land = height(column_land);
        for x in 0..50 {
            pixbuf[x + 100 * (256 - h) as usize] = color_land;
        }

        let wh = std::cmp::min(255, std::cmp::max(h+3, 95)) as u8;
        let column_water: &[u8] = &[
            wh, 0, 8, 0,
            h as u8, 0, 1, 0,
            0, 0, 0, 0,
            0, 0, 0, 0,
            0
        ];
        let color_water = height(column_water);
        for x in 50..100 {
            pixbuf[x + 100 * (256 - h) as usize] = color_water;
        }
    }

    print!("Saving as {}\n", img_path);
    lodepng::encode32_file(img_path, &pixbuf, 100, 256).unwrap();
}
