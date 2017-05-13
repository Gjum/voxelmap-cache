extern crate docopt;
extern crate nbtrs;
extern crate rustc_serialize;
extern crate threadpool;
extern crate voxelmap_cache;

use docopt::Docopt;
use nbtrs::RegionFile;
use nbtrs::Taglike;
use std::collections::LinkedList;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Instant;
use threadpool::ThreadPool;
use voxelmap_cache::print_progress;
use voxelmap_cache::processor::RegionPos;

const USAGE: &'static str = "
Usage: worldstats [-q] [-t threads] <world-dir>

Options:
    -q, --quiet  Do not output info messages.
    -t           Number of threads to use for parallel processing
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_quiet: bool,
    arg_threads: Option<usize>,
    arg_world_dir: String,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    let verbose = !args.flag_quiet;

    if verbose { println!("Finding regions from {} ...", args.arg_world_dir); }

    let regions = get_regions(&args.arg_world_dir);

    if verbose { println!("analyzing {} regions", regions.len()); }

    let start_time = Instant::now();
    analyze_parallelized(
        regions,
        args.arg_threads.unwrap_or(4),
        verbose,
    );

    if verbose {
        let time_total = start_time.elapsed().as_secs();
        println!("Done after {}:{:02}", time_total / 60, time_total % 60);
    }

}

fn analyze_region(region_path: PathBuf) -> Result<(RegionPos, u64), (RegionPos, String)> {
    let region_pos = xz_from_region_path(&region_path)
        .expect(&format!("Getting region position of {:?}", region_path));

    let region_file = try!(fs::File::open(&region_path)
        .map_err(|e| (region_pos, e.to_string())));
    let mut region = try!(RegionFile::new(&region_file)
        .map_err(|e| (region_pos, e.to_string())));

    let mut count = 0;

    for z in 0..32 {
        for x in 0..32 {
            if region.chunk_exists(x, z) {
                let chunk = try!(region.load_chunk(x, z)
                    .map_err(|e| (region_pos, e.to_string())));
                let sections = chunk.key("Level").expect("Accessing Level")
                    .key("Sections").expect("Accessing Sections")
                    .as_list().expect("Accessing sections as list");
                for section in sections {
                    let blocks = section.key("Blocks").expect("Accessing Blocks")
                        .as_bytes().expect("Accessing Blocks as bytes");
                    let data = section.key("Data").expect("Accessing Data")
                        .as_bytes().expect("Accessing Data as bytes");

                    for block in blocks {
                        if *block != 0 {
                            count += 1;
                        }
                    }
                }
            }
        }
    }

    Ok((region_pos, count))
}

pub fn analyze_parallelized(
    regions: LinkedList<PathBuf>,
    num_threads: usize,
    verbose: bool,
) {
    let pool = ThreadPool::new(num_threads);
    let (tx, rx) = channel();

    let start_time_regions = Instant::now();

    for work_item in &regions {
        let tx = tx.clone();
        let my_work_item = work_item.clone();
        pool.execute(move || {
            let result = analyze_region(my_work_item);
            tx.send(result).unwrap();
        });
    }

    let mut count_total = 0;

    let mut next_msg_elapsed = 3; // for progress meter
    let total_work = regions.len();
    for work_done in 0..total_work {
        match rx.recv().unwrap() {
            Err((region_pos, error)) => {
                println!("Error processing region {:?}: {:?}", region_pos, error);
            }
            Ok((region_pos, count_region)) => {
                count_total += count_region;
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
    };

    println!("Found {:?} blocks in the world", count_total);
}

fn xz_from_region_path(region_path: &PathBuf) -> Result<(i32, i32), std::num::ParseIntError> {
    let fname = region_path.file_name().unwrap().to_str().unwrap();
    // r.<x>.<z>.mca
    let (coords_part, _) = fname.split_at(fname.len() - 4);
    let mut it = coords_part.splitn(3, '.');
    it.next().unwrap();
    let x = try!(it.next().unwrap().parse());
    let z = try!(it.next().unwrap().parse());
    Ok((x, z))
}

pub fn get_regions(dir: &str) -> LinkedList<PathBuf> {
    let mut region_paths = LinkedList::new();
    for region_dir_entry in fs::read_dir(dir).expect("Listing region files") {
        let region_path = region_dir_entry.expect("Getting region directory entry").path();
        let xz_result = xz_from_region_path(&region_path);
        if xz_result.is_ok() {
            let (x, z) = xz_result.unwrap();
            if -20 <= x && x < 20 && -20 <= z && z < 20 {
                region_paths.push_back(region_path);
            } else {
                println!("Ignoring region file outside world border: {:?}", &region_path);
            }
        } else if region_path.to_string_lossy().ends_with("_chunk-times.gz") {
            // ignore chunk timestamp info file
        } else {
            println!("Ignoring non-region file {:?}", &region_path);
        }
    }
    region_paths
}
