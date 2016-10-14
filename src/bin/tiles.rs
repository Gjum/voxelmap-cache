extern crate voxelmap_cache;

extern crate lodepng;
extern crate threadpool;
extern crate zip;

use std::collections::LinkedList;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Instant;
use threadpool::ThreadPool;

use voxelmap_cache::{get_regions, rgba, xz_from_zip_path};

const TILE_SIZE: usize = 256;
const TILE_PIXELS: usize = TILE_SIZE * TILE_SIZE;

fn heightmap(column: &[u8; 17]) -> u32 {
    let h = column[0];
    rgba(h, h, h, 255)
}

fn lightmap(column: &[u8; 17]) -> u32 {
    let bl = column[3] & 0xf;
    rgba(bl * 17, bl * 17, bl * 17, 255)
}

fn sandmap(column: &[u8; 17]) -> u32 {
    let b = column[2];
    let s = column[2 + 4];
    return if b == 12 {
        rgba(255, 240, 100, 255)
    } else if s == 12 {
        rgba(128, 120, 200, 255)
    } else if column == &[0; 17] {
        rgba(0, 0, 0, 0)
    } else {
        rgba(0, 0, 0, 255)
    };
}

fn do_work(zip_path: PathBuf) -> Result<(), io::Error> {
    let zip_file = try!(File::open(&zip_path));
    let mut zip_archive = try!(zip::ZipArchive::new(zip_file));
    let mut data_file = try!(zip_archive.by_index(0));
    let mut pixbuf: [u32; TILE_PIXELS] = [0; TILE_PIXELS];
    let column = &mut [0; 17];

    for i in 0..TILE_PIXELS {
        try!(data_file.read(column));
        pixbuf[i] = sandmap(column);
    }

    let (x, z) = xz_from_zip_path(&zip_path);
    let img_path = format!("img/{},{}.png", x, z);
    lodepng::encode32_file(img_path, &pixbuf, TILE_SIZE, TILE_SIZE);
    Ok(())
}

fn main() {
    let num_threads = 4;
    let work_items = get_regions("../devotedmap/local/cache/world/").unwrap();
    let total_work = work_items.len();
    println!("found {} regions", total_work);

    let pool = ThreadPool::new(num_threads);
    let (tx, rx) = channel();

    let start_time = Instant::now();

    for work_item in &work_items {
        let tx = tx.clone();
        let my_work_item: PathBuf = work_item.clone();

        pool.execute(move || {
            let result = do_work(my_work_item);
            tx.send(result).unwrap();
        });
    }

    let mut next_msg_elapsed = 1; // for progress meter
    for work_done in 0..total_work {
        match rx.recv() {
            Ok(r) => {}
            Err(e) => { println!("ERROR {:?}", e) }
        }

        print_progress(work_done, total_work, start_time, &mut next_msg_elapsed);
    }

    let time_per_work_item = start_time.elapsed() / total_work as u32;
    let region_sec = time_per_work_item.as_secs();
    let region_ms = time_per_work_item.subsec_nanos() / 1_000_000;
    println!("Done. {}.{:03?} per region", region_sec, region_ms);
}

fn print_progress(done: usize, total: usize, start_time: Instant, next_msg_elapsed: &mut u64) {
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
    *next_msg_elapsed += 1;

    let work_left = total - done;
    let sec_left = elapsed as usize * work_left / done;
    let min = sec_left / 60;
    let sec = sec_left % 60;
    println!("{}/{} processed, {}:{:02?} left",
        done, total, min, sec);
}
