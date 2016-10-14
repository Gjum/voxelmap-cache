extern crate zip;

use std::collections::LinkedList;
use std::fs;
use std::io;
use std::io::Read;
use std::path::PathBuf;
use zip::read::ZipFile;
use zip::result::ZipError;

pub fn xz_from_zip_path(zip_path: &PathBuf) -> (i32, i32) {
    let fname = zip_path.file_name().unwrap().to_str().unwrap();
    let (coords_part, _) = fname.split_at(fname.len() - 4);
    let mut it = coords_part.splitn(2, ',');
    let x = it.next().unwrap().parse().unwrap();
    let z = it.next().unwrap().parse().unwrap();
    return (x, z);
}

pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (r as u32)
    | ((g as u32) << 8)
    | ((b as u32) << 16)
    | ((a as u32) << 24)
}

pub fn get_regions(dir: &str) -> Result<LinkedList<PathBuf>, io::Error> {
    let mut region_paths = LinkedList::new();
    for zip_dir_entry in try!(fs::read_dir(dir)) {
        let zip_path = try!(zip_dir_entry).path();
        region_paths.push_back(zip_path);
    }
    Ok(region_paths)
}
