extern crate lodepng;
extern crate voxelmap_cache;

use std::time;
use voxelmap_cache::colorizer::*;

fn main() {
    let unix_time = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let img_path = std::env::args()
        .skip(1)
        .next()
        .unwrap_or("palette_{t}.png".to_string())
        .replace("{t}", &unix_time.to_string());

    let mut pixbuf = [0_u32; 100 * 256];
    for h in 1..257_usize {
        let color_land = get_land_color(h as u8);
        for x in 0..50 {
            pixbuf[x + 100 * (256 - h) as usize] = color_land;
        }
        let color_water = get_sea_color(h as u8);
        for x in 50..100 {
            pixbuf[x + 100 * (256 - h) as usize] = color_water;
        }
    }

    print!("Saving as {}\n", img_path);
    lodepng::encode32_file(img_path, &pixbuf, 100, 256).unwrap();
}
