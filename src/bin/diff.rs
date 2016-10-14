// XXX change arg to zip_add + context(main_cache)
extern crate voxelmap_cache;

extern crate lodepng;
extern crate threadpool;
extern crate zip;

use std::fs::File;
use std::io;
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Instant;
use threadpool::ThreadPool;

use voxelmap_cache::{get_regions, rgba, xz_from_zip_path};

const TILE_SIZE: usize = 256;
const TILE_BLOCKS: usize = TILE_SIZE * TILE_SIZE;

fn diff(col_main: &[u8; 17], col_add: &[u8; 17]) -> u32 {
    let h = col_main[0];
    rgba(h, h, h, 255)
}

fn do_work(args: (PathBuf, PathBuf))
-> Result<((i32, i32), [u32; TILE_BLOCKS]), io::Error> {
    // XXX check if file exists
    let (zip_main, zip_add) = args;
    let zip_file_main = try!(File::open(&zip_main));
    let mut zip_archive_main = try!(zip::ZipArchive::new(zip_file_main));
    let mut file_main = try!(zip_archive_main.by_index(0));
    let zip_file_add = try!(File::open(&zip_add));
    let mut zip_archive_add = try!(zip::ZipArchive::new(zip_file_add));
    let mut file_add = try!(zip_archive_add.by_index(0));

    let mut pixbuf: [u32; TILE_BLOCKS] = [0; TILE_BLOCKS];
    let col_main = &mut [0; 17];
    let col_add = &mut [0; 17];

    for i in 0..TILE_BLOCKS {
        try!(file_main.read(col_main));
        try!(file_add.read(col_add));
        pixbuf[i] = diff(col_main, col_add);
    }

    let region_pos = xz_from_zip_path(&zip_main);
    Ok((region_pos, pixbuf))
}

fn main() {
    let num_threads = 4;
    let work_items = get_regions("../devotedmap/local/cache/world/").unwrap();
    let total_work = work_items.len();
    println!("found {} regions", total_work);

    let pool = ThreadPool::new(num_threads);
    let (send, recv) = channel();

    let start_time = Instant::now();

    for work_item in &work_items {
        let send = send.clone();
        let my_work_item = work_item.clone();

        pool.execute(move || {
            let result = do_work((my_work_item.clone(), my_work_item)); // XXX fix args
            send.send(result).unwrap();
        });
    }

    let img_width = TILE_SIZE;
    let img_height = TILE_SIZE;
    let img_x_off = 0;
    let img_z_off = 0;
    let mut pixbuf = vec![0_u32; img_width*img_height].into_boxed_slice();

    let mut next_msg_elapsed = 1; // for progress meter
    for work_done in 0..total_work {
        match recv.recv().unwrap() {
            Err(e) => { println!("ERROR {:?}", e) }
            Ok(((rx, rz), data)) => {
                let x_off = rx as usize * TILE_SIZE + img_x_off;
                let z_off = rz as usize * TILE_SIZE + img_z_off;
                for i in 0..TILE_SIZE {
                    let data_line = (rz as usize + i) * TILE_SIZE;
                    let img_line = x_off + (z_off + i) * img_width;
                    let mut slice: &mut [u32] = &mut pixbuf[img_line .. img_line + img_width];
                    slice.clone_from_slice(&data[data_line .. data_line + TILE_SIZE]);
                }
            }
        }

        print_progress(work_done, total_work, start_time, &mut next_msg_elapsed);
    }

    let time_per_work_item = start_time.elapsed() / total_work as u32;
    let region_sec = time_per_work_item.as_secs();
    let region_ms = time_per_work_item.subsec_nanos() / 1_000_000;
    println!("Took {}.{:03?} per region", region_sec, region_ms);

    let img_path = format!("diff_{}.png", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    lodepng::encode32_file(img_path, &pixbuf, TILE_SIZE, TILE_SIZE);
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
