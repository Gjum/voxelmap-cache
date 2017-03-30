extern crate lodepng;

use std::fs;
use std::path::Path;
use std::time;

pub const REGION_WIDTH: usize = 256;
pub const REGION_HEIGHT: usize = 256;
pub const REGION_BLOCKS: usize = REGION_WIDTH * REGION_HEIGHT;

pub type RegionPos = (i32, i32);
pub type RegionPixels = [u32; REGION_BLOCKS];

pub trait Processor {
    fn process_region(&mut self, region_pos: RegionPos, region_pixels: Box<RegionPixels>);
    fn pre_process(&mut self) {}
    fn post_process(&mut self) {}
}

pub struct TilesProcessor {
    pub tiles_pattern: String,
}

impl Processor for TilesProcessor {
    fn process_region(&mut self, region_pos: RegionPos, region_pixels: Box<RegionPixels>) {
        let (rx, rz) = region_pos;
        let img_path = self.tiles_pattern
            .replace("{tile}", &(format!("{},{}", rx, rz)))
            .replace("{x}", &rx.to_string())
            .replace("{z}", &rz.to_string());
        let dir = Path::new(&img_path).parent().unwrap();
        fs::create_dir_all(dir).unwrap();
        lodepng::encode32_file(&img_path, &region_pixels[..], REGION_WIDTH, REGION_WIDTH).unwrap();
    }
}

pub struct SingleImageProcessor {
    pixbuf: Box<[u32]>,
    img_path: String,
}

const IMG_WIDTH: usize = 45 * REGION_WIDTH;
const IMG_HEIGHT: usize = 45 * REGION_HEIGHT;
const IMG_WEST: i32 = -21 * REGION_WIDTH as i32;
const IMG_NORTH: i32 = -21 * REGION_HEIGHT as i32;

impl SingleImageProcessor {
    pub fn new(img_pattern: &String) -> SingleImageProcessor {
        SingleImageProcessor {
            pixbuf: Box::new([0_u32; IMG_WIDTH * IMG_HEIGHT]),
            img_path: SingleImageProcessor::replace_timestamp(img_pattern),
        }
    }

    pub fn replace_timestamp(img_pattern: &String) -> String {
        let unix_time = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap().as_secs();
        img_pattern.replace("{t}", &unix_time.to_string())
    }
}

impl Processor for SingleImageProcessor {
    fn process_region(&mut self, region_pos: RegionPos, region_pixels: Box<RegionPixels>) {
        let (rx, rz) = region_pos;
        let x_off: i32 = rx * REGION_WIDTH as i32 - IMG_WEST;
        let z_off: i32 = rz * REGION_HEIGHT as i32 - IMG_NORTH;
        for (line_z, region_line) in region_pixels.chunks(REGION_WIDTH).enumerate() {
            let img_line = (x_off + (z_off + line_z as i32)) as usize * IMG_WIDTH;
            let img_slice = &mut self.pixbuf[img_line..img_line + REGION_WIDTH];
            img_slice.copy_from_slice(region_line);
            // img_slice.clone_from_slice(region_line); // TODO compare speed of clone vs copy
        }
    }

    fn pre_process(&mut self) {
        let dir = Path::new(&self.img_path).parent().unwrap();
        fs::create_dir_all(dir).unwrap();
    }

    fn post_process(&mut self) {
        println!("Saving image as {}", self.img_path);
        lodepng::encode32_file(&self.img_path, &self.pixbuf[..], IMG_WIDTH, IMG_HEIGHT).unwrap();
    }
}
