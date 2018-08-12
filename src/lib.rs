extern crate zip;

use blocks::BLOCK_STRINGS_ARR;
use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime};

pub mod blocks;
pub mod colorizer;
pub mod tile;

pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_HEIGHT: usize = 16;
pub const CHUNK_COLUMNS: usize = CHUNK_WIDTH * CHUNK_HEIGHT;

pub const TILE_WIDTH: usize = 256;
pub const TILE_HEIGHT: usize = 256;
pub const TILE_COLUMNS: usize = TILE_WIDTH * TILE_HEIGHT;
pub const TILE_CHUNKS: usize = TILE_COLUMNS / CHUNK_COLUMNS;

pub const REGION_WIDTH: usize = 512;
pub const REGION_HEIGHT: usize = 512;
pub const REGION_COLUMNS: usize = REGION_WIDTH * REGION_HEIGHT;
pub const REGION_CHUNKS: usize = REGION_COLUMNS / CHUNK_COLUMNS;

pub fn get_block_name_from_voxelmap(vm_a: u8, vm_b: u8) -> &'static str {
    // BLOCK_STRINGS_ARR is id << 4 | meta
    // voxelmap is meta << 12 | id
    BLOCK_STRINGS_ARR[(vm_b as usize) << 4 | (vm_a as usize) >> 4]
}

pub fn get_mtime_or_0(path: &PathBuf) -> u64 {
    fs::metadata(path)
        .map(|metadata| match metadata.modified() {
            Ok(time) => time
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|x| x.as_secs())
                .unwrap_or(0),
            _ => 0,
        })
        .unwrap_or(0)
}

pub fn parse_bounds(bounds_str: &str) -> Result<Vec<i32>, String> {
    let bounds = bounds_str
        .splitn(4, ",")
        .map(|s| match &s[0..1] {
            "c" => s[1..].parse::<i32>().map(|c| c * CHUNK_WIDTH as i32 + 1),
            "t" => s[1..].parse::<i32>().map(|c| c * TILE_WIDTH as i32 + 1),
            "r" => s[1..].parse::<i32>().map(|c| c * REGION_WIDTH as i32 + 1),
            _ => s.parse(),
        })
        .collect::<Result<Vec<i32>, _>>()
        .map_err(|e| e.to_string())?;

    if bounds.len() != 4 || bounds[0] > bounds[2] || bounds[1] > bounds[3] {
        Err("should be: w,n,e,s".to_string())
    } else {
        Ok(bounds)
    }
}

pub const PROGRESS_INTERVAL: u64 = 3;

// TODO put more weight on recent measurements
pub fn print_progress(done: usize, total: usize, start_time: Instant, next_msg_elapsed: &mut u64) {
    if total <= 0 || done == 0 {
        return;
    }

    let elapsed = start_time.elapsed().as_secs();
    if elapsed < *next_msg_elapsed {
        return;
    }

    if *next_msg_elapsed < elapsed {
        *next_msg_elapsed = elapsed;
    }
    *next_msg_elapsed += PROGRESS_INTERVAL;

    let work_left = total - done;
    let sec_left = elapsed as usize * work_left / done;
    let min = sec_left / 60;
    let sec = sec_left % 60;
    println!("{}/{} processed, {}:{:02?} left", done, total, min, sec);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bounds_with_world_coords() {
        let bounds_str = "1,-22222,33333,-4";
        let bounds = parse_bounds(bounds_str);
        assert_eq!(Ok(vec![1, -22222, 33333, -4]), bounds);
    }

    #[test]
    fn parse_bounds_with_tile_coords() {
        let bounds_str = "t-2,t-33,t4,t5";
        let bounds = parse_bounds(bounds_str);
        let bounds_regions = bounds.map(|b| b.iter().map(|c| *c >> 8).collect::<Vec<i32>>());
        assert_eq!(Ok(vec![-2, -33, 4, 5]), bounds_regions);
    }
}
