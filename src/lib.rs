extern crate zip;
extern crate threadpool;

use colorizer::*;
use processor::*;
use std::collections::LinkedList;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Instant;
use threadpool::ThreadPool;

pub mod biomes;
pub mod colorizer;
pub mod processor;

pub fn render_parallelized(
    mut processor: Box<Processor>,
    colorizer: Colorizer,
    regions: LinkedList<PathBuf>,
    num_threads: usize,
    verbose: bool,
) {
    let colorizer_arc = Arc::new(colorizer);
    let pool = ThreadPool::new(num_threads);
    let (tx, rx) = channel();

    let start_time = Instant::now();

    for work_item in &regions {
        let tx = tx.clone();
        let my_work_item = work_item.clone();
        let my_colorizer = colorizer_arc.clone();
        pool.execute(move || {
            let result = render_region(my_work_item, my_colorizer.as_ref());
            tx.send(result).unwrap();
        });
    }

    let mut next_msg_elapsed = 1; // for progress meter
    let total_work = regions.len();
    for work_done in 0..total_work {
        match rx.recv().unwrap() {
            Err(e) => { println!("ERROR {:?}", e) }
            Ok((region_pos, region_pixels)) => {
                processor.process_region(region_pos, region_pixels);
            }
        }

        if verbose { print_progress(work_done, total_work, start_time, &mut next_msg_elapsed); }
    }

    if verbose {
        let time_per_work_item = start_time.elapsed() / total_work as u32;
        let region_sec = time_per_work_item.as_secs();
        let region_ms = time_per_work_item.subsec_nanos() / 1_000_000;
        println!("Took {}.{:03?} per region", region_sec, region_ms);
        println!("Post-processing...");
    };

    processor.post_process();

    if verbose { println!("Done."); }
}

fn render_region(zip_path: PathBuf, colorizer: &Colorizer) -> Result<(RegionPos, Box<RegionPixels>), String> {
    let (rx, rz) = try!(xz_from_zip_path(&zip_path).map_err(|e| e.to_string()));

    let zip_file = try!(fs::File::open(&zip_path)
        .map_err(|e| e.to_string()));
    let mut zip_archive = try!(zip::ZipArchive::new(zip_file)
        .map_err(|e| e.to_string()));
    let mut data_file = try!(zip_archive.by_index(0)
        .map_err(|e| e.to_string()));

    let mut pixbuf = Box::new([0_u32; REGION_BLOCKS]);
    let column = &mut [0; 17];
    let get_column_color = colorizer.column_color_fn();

    for i in 0..REGION_BLOCKS {
        try!(data_file.read(column).map_err(|e| e.to_string()));
        pixbuf[i] = get_column_color(column);
    }

    Ok(((rx, rz), pixbuf))
}

fn xz_from_zip_path(zip_path: &PathBuf) -> Result<(i32, i32), std::num::ParseIntError> {
    let fname = zip_path.file_name().unwrap().to_str().unwrap();
    let (coords_part, _) = fname.split_at(fname.len() - 4);
    let mut it = coords_part.splitn(2, ',');
    let x = try!(it.next().unwrap().parse());
    let z = try!(it.next().unwrap().parse());
    Ok((x, z))
}

pub fn get_regions(dir: &str) -> Result<LinkedList<PathBuf>, String> {
    let mut region_paths = LinkedList::new();
    for zip_dir_entry in try!(fs::read_dir(dir).map_err(|e| e.to_string())) {
        let zip_path = try!(zip_dir_entry.map_err(|e| e.to_string())).path();
        if xz_from_zip_path(&zip_path).is_ok() {
            region_paths.push_back(zip_path);
        } else if zip_path.to_string_lossy().ends_with("_chunk-times.gz") {
        } else {
            println!("Ignoring non-region file {:?}", &zip_path);
        }
    }
    Ok(region_paths)
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
