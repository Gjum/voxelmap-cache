extern crate lodepng;
extern crate zip;

use std::collections::LinkedList;
use std::fs::read_dir;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

const IMG_SIZE: usize = 256;
const IMG_PIXELS: usize = IMG_SIZE * IMG_SIZE;

fn xz_from_zip_path(zip_path: &PathBuf) -> (i32, i32) {
    let fname = zip_path.file_name().unwrap().to_str().unwrap();
    let (coords_part, _) = fname.split_at(fname.len() - 4);
    let mut it = coords_part.splitn(2, ',');
    let x = it.next().unwrap().parse().unwrap();
    let z = it.next().unwrap().parse().unwrap();
    return (x, z);
}

fn heightmap(column: &[u8; 17]) -> (u8, u8, u8, u8) {
    let h = column[0];
    (h, h, h, 255)
}

fn do_work(zip_path: &PathBuf) -> Result<(), io::Error> {
    let zip_file = try!(File::open(&zip_path));
    let mut zip_archive = try!(zip::ZipArchive::new(zip_file));
    let mut data_file = try!(zip_archive.by_index(0));
    let mut pixbuf: [u32; IMG_PIXELS] = [0xff789789; IMG_PIXELS];
    let column = &mut [0; 17];

    for i in 0..IMG_PIXELS {
        try!(data_file.read(column));
        let (r, g, b, a) = heightmap(column);
        pixbuf[i] = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
    }

    let (x, z) = xz_from_zip_path(&zip_path);
    let img_path = format!("img/{},{}.png", x, z);
    lodepng::encode32_file(img_path, &pixbuf, IMG_SIZE, IMG_SIZE);
    Ok(())
}

fn get_regions() -> Result<LinkedList<PathBuf>, io::Error> {
    let mut region_paths = LinkedList::new();
    for zip_dir_entry in try!(read_dir("../devotedmap/local/cache/world/")) {
        let zip_path = try!(zip_dir_entry).path();
        region_paths.push_back(zip_path);
    }
    Ok(region_paths)
}

fn main() {
    let num_threads = 4;
    let mut workers = Vec::with_capacity(num_threads);
    let region_paths = get_regions().unwrap();
    let total_regions = region_paths.len();
    println!("found {} regions", total_regions);
    let tasks = Arc::new(Mutex::new(region_paths));

    for worker_nr in 0..num_threads {
        let thread_tasks = tasks.clone();
        workers.push(thread::spawn(move || {
            loop {
                let task = { // extra scope to limit lock duration
                    let mut tasks = thread_tasks.lock().unwrap();
                    tasks.pop_front()
                };
                match task {
                    None => break,
                    Some(zip_path) => {
                        match do_work(&zip_path) {
                            Ok(_) => {},
                            Err(e) => println!("worker {} failed at {:?} {}",
                                worker_nr, zip_path, e),
                        }
                    },
                };
            }
        }));
    }

    for worker in workers.into_iter() {
        worker.join().unwrap();
    }
}
