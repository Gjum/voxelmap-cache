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

    processor.pre_process();

    let start_time_regions = Instant::now();

    let total_work = regions.len();
    for work_item in &regions {
        let tx = tx.clone();
        let my_work_item = work_item.clone();
        let my_colorizer = colorizer_arc.clone();
        pool.execute(move || {
            let result = render_region(&my_work_item, my_colorizer.as_ref());
            tx.send(result).expect(&format!("Sending result from {:?}", my_work_item));;
        });
    }

    let mut next_msg_elapsed = 3; // for progress meter
    for work_done in 0..total_work {
        match rx.recv().expect("Receiving next result") {
            Err((region_pos, error)) => {
                println!("Error rendering region {:?}: {:?}", region_pos, error);
            }
            Ok((region_pos, region_pixels)) => {
                processor.process_region(region_pos, region_pixels);
            }
        }

        if verbose { print_progress(work_done, total_work, start_time_regions, &mut next_msg_elapsed); }
    }

    if verbose {
        let time_total = start_time_regions.elapsed();
        let total_min = time_total.as_secs() / 60;
        let total_sec = time_total.as_secs() % 60;
        let time_per_work_item = time_total / total_work as u32;
        let region_sec = time_per_work_item.as_secs();
        let region_ms = time_per_work_item.subsec_nanos() / 1_000_000;
        println!("Took {}:{:02} total, {}.{:03?} per region",
                 total_min, total_sec, region_sec, region_ms);
        println!("Post-processing ...");
    };

    processor.post_process();
}

fn render_region(zip_path: &PathBuf, colorizer: &Colorizer) -> Result<(RegionPos, Box<RegionPixels>), (RegionPos, String)> {
    let region_pos = xz_from_zip_path(&zip_path)
        .expect(&format!("Getting region position of {:?}", zip_path));

    let zip_file = try!(fs::File::open(&zip_path)
        .map_err(|e| (region_pos, e.to_string())));
    let mut zip_archive = try!(zip::ZipArchive::new(zip_file)
        .map_err(|e| (region_pos, e.to_string())));
    let mut data_file = try!(zip_archive.by_index(0)
        .map_err(|e| (region_pos, e.to_string())));

    let mut pixbuf = Box::new([0_u32; REGION_BLOCKS]);
    let columns = &mut [0; 17*REGION_BLOCKS];
    let get_column_color = colorizer.column_color_fn();

    try!(data_file.read(columns).map_err(|e| (region_pos, e.to_string())));

    for (i, column) in columns.chunks(17).enumerate() {
        pixbuf[i] = get_column_color(column);
    }

    Ok((region_pos, pixbuf))
}

fn xz_from_zip_path(zip_path: &PathBuf) -> Result<(i32, i32), std::num::ParseIntError> {
    let fname = zip_path.file_name().unwrap().to_str().unwrap();
    let (coords_part, _) = fname.split_at(fname.len() - 4);
    let mut it = coords_part.splitn(2, ',');
    let x = try!(it.next().unwrap().parse());
    let z = try!(it.next().unwrap().parse());
    Ok((x, z))
}

pub fn get_regions(dir: &str, verbose: bool) -> LinkedList<PathBuf> {
    let mut region_paths = LinkedList::new();
    for zip_dir_entry in fs::read_dir(dir).expect("Listing region files") {
        let zip_path = zip_dir_entry.expect("Getting region directory entry").path();
        let xz_result = xz_from_zip_path(&zip_path);
        if xz_result.is_ok() {
            let (x, z) = xz_result.unwrap();
            if -20 <= x && x < 20 && -20 <= z && z < 20 {
                region_paths.push_back(zip_path);
            } else {
                if verbose { println!("Ignoring region file outside world border: {:?}", &zip_path); }
            }
        } else if zip_path.to_string_lossy().ends_with("_chunk-times.gz") {
            // ignore chunk timestamp info file
        } else {
            if verbose { println!("Ignoring non-region file {:?}", &zip_path); }
        }
    }
    region_paths
}

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
    *next_msg_elapsed += 3;

    let work_left = total - done;
    let sec_left = elapsed as usize * work_left / done;
    let min = sec_left / 60;
    let sec = sec_left % 60;
    println!("{}/{} processed, {}:{:02?} left",
        done, total, min, sec);
}
