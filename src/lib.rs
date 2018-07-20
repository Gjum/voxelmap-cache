extern crate zip;

use blocks::BLOCK_STRINGS_ARR;
use std::collections::LinkedList;
use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime};
use tile::{TilePos, TILE_HEIGHT, TILE_WIDTH};

pub mod blocks;
pub mod colorizer;
pub mod tile;

pub fn get_block_name_from_voxelmap(vm_a: u8, vm_b: u8) -> &'static str {
    // BLOCK_STRINGS_ARR is id << 4 | meta
    // voxelmap is meta << 12 | id
    BLOCK_STRINGS_ARR[(vm_b as usize) << 4 | (vm_a as usize) >> 4]
}

pub fn get_xz_from_tile_path(tile_path: &PathBuf) -> Result<TilePos, String> {
    let fname = tile_path.file_name().unwrap().to_str().unwrap();
    if fname.len() <= 4 {
        return Err("file name too short".to_owned());
    }
    let (coords_part, _) = fname.split_at(fname.len() - 4);
    let mut it = coords_part.splitn(3, ',');
    let x = it
        .next()
        .ok_or("no x coord in filename".to_owned())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let z = it
        .next()
        .ok_or("no z coord in filename".to_owned())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    Ok((x, z))
}

pub fn get_contrib_from_tile_path(tile_path: &PathBuf) -> Result<String, String> {
    let fname = tile_path.file_name().unwrap().to_str().unwrap();
    if fname.len() <= 4 {
        return Err("no contrib in filename".to_owned());
    }
    let (coords_part, _) = fname.split_at(fname.len() - 4);
    Ok(coords_part
        .splitn(3, ',')
        .skip(2)
        .next()
        .unwrap()
        .to_string())
}

pub fn get_tile_paths_in_dirs(
    dirs: &Vec<String>,
    verbose: bool,
) -> Result<LinkedList<PathBuf>, String> {
    let mut tile_paths = LinkedList::new();
    for dir in dirs {
        for zip_dir_entry in fs::read_dir(dir.as_str()).map_err(|e| e.to_string())? {
            let tile_path = zip_dir_entry.map_err(|e| e.to_string())?.path();
            match get_xz_from_tile_path(&tile_path) {
                Ok(_pos) => {
                    if tile_path.to_string_lossy().ends_with(".zip") {
                        tile_paths.push_back(tile_path)
                    } else {
                        println!("Ignoring non-tile file {:?}", &tile_path);
                    }
                }
                Err(e) => {
                    if tile_path.to_string_lossy().ends_with("_chunk-times.gz") {
                        // ignore chunk timestamp info file
                    } else {
                        if verbose {
                            println!("Ignoring non-tile file {:?} {:?}", &tile_path, e);
                        }
                    }
                }
            }
        }
    }
    Ok(tile_paths)
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
            "t" => s[1..].parse::<i32>().map(|c| c * TILE_WIDTH as i32 + 42), // convert tile coords to world coords
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

pub fn is_tile_pos_in_bounds((tile_x, tile_z): (i32, i32), bounds: &Vec<i32>) -> bool {
    let tw = TILE_WIDTH as i32;
    let th = TILE_HEIGHT as i32;
    let x = tile_x * tw;
    let z = tile_z * th;
    let (w, n, e, s) = (bounds[0], bounds[1], bounds[2], bounds[3]);

    x + tw > w && x < e && z + th > n && z < s
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
        assert_eq!(Ok(vec![-470, -8406, 1066, 1322]), bounds);
    }
}
