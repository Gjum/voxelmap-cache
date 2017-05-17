extern crate docopt;
extern crate glob;
extern crate rustc_serialize;
extern crate threadpool;
extern crate voxelmap_cache;

use docopt::Docopt;
use glob::glob;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Instant;
use threadpool::ThreadPool;
use voxelmap_cache::*;
use voxelmap_cache::processor::*;
use voxelmap_cache::colorizer::is_empty;

const USAGE: &'static str = "
Usage: diff-all [-q] [-t threads] <cache-path> <output>

cache-path contains voxelmap caches in the format `<x>,<z>,<contrib-name>.zip`

Options:
    -q, --quiet  Do not output info messages.
    -t           Number of threads to use for parallel processing
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_quiet: bool,
    arg_threads: Option<usize>,
    arg_cache_path: String,
    arg_output: String,
}

fn main() {
    let start_time = Instant::now();

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    let verbose = !args.flag_quiet;

    if verbose { println!("Analyzing tiles from {} ...", args.arg_cache_path); }

    let pool = ThreadPool::new(args.arg_threads.unwrap_or(4));
    let (tx, rx) = channel();

    let start_time_regions = Instant::now();

    let total_work = 40*40;
    for tile_z in -20..20 {
        for tile_x in -20..20 {
            let tx = tx.clone();
            let world_dir = args.arg_cache_path.clone();
            pool.execute(move || {
                let result = analyze_tile((tile_x, tile_z), world_dir);
                tx.send(result).unwrap();
            });
        }
    }

    let mut processor = get_processor(&args.arg_output);

    processor.pre_process();

    let mut next_msg_elapsed = 3; // for progress meter
    for work_done in 0..total_work {
        match rx.recv().unwrap() {
            Err((tile_pos, error)) => {
                println!("Error processing tile {:?}: {:?}", tile_pos, error);
            }
            Ok((tile_pos, pixbuf)) => {
                processor.process_region(tile_pos, pixbuf);
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
        println!("Took {}:{:02} total, {}.{:03?} per tile",
                 total_min, total_sec, region_sec, region_ms);
        println!("Post-processing ...");
    };

    processor.post_process();

    if verbose {
        let time_total = start_time.elapsed().as_secs();
        println!("Done after {}:{:02}", time_total / 60, time_total % 60);
    }
}

fn analyze_tile(tile_pos: RegionPos, world_path: String) -> Result<(RegionPos, Box<RegionPixels>), (RegionPos, String)> {
    let (tile_x, tile_z) = tile_pos;
    let tiles_glob = format!("{}/{},{}*.zip", world_path, tile_x, tile_z);
    let matching_dir_entries = glob(&tiles_glob).expect("Parsing glob pattern");
    let tiles = matching_dir_entries
        .map(|path_result|
            match path_result {
                Err(e) => {
                    println!("{} in {:?}", e, e.path());
                    None::<(Box<TileCache>, _)>
                }
                Ok(path) => {
                    read_tile_cache(&path)
                    .map_err(|e| {
                        println!("{} in {:?}", e, path);
                        ()
                    })
                    .ok()
                    .map(|tile_cache| (tile_cache, path))
                }
            }
        )
        .filter_map(|x| x)
        .collect::<Vec<(Box<TileCache>, _)>>();

    let num_caches_found = tiles.len();
    if num_caches_found <= 0 {
        return Err((tile_pos, format!("No tiles for {}", tiles_glob)));
    }

    let reference_cache = try!(read_tile_cache(&PathBuf::from(format!("/home/gjum/data/devoted3-voxelmap/contrib/new/jecowa_full_wdl_2017-05-02/world/Overworld (dimension 0)/{},{}.zip", tile_x, tile_z)))
        .map_err(|e| (tile_pos, e.to_string())));

    let mut pixbuf = Box::new([0_u32; REGION_BLOCKS]);
    for i in 0..REGION_BLOCKS {
        let mut color = 0xff_000000; // black: unchanged
        let reference_column = &reference_cache[i*17 .. (i+1)*17];
        let mut prev = reference_column;
        for &(ref columns, ref tile_path) in &tiles {
            let column = &columns[i*17 .. (i+1)*17];
            if is_empty(column) {
                continue;
            }
            // compare with known biome from reference cache
            if column[16] != reference_column[16] {
                continue;
            }
            if column != prev {
                color |= 0xff_ff0000; // blue
                if column[0] != prev[0] {
                    color |= 0xff_0000ff; // red
                }
                if column[1] != prev[1] || column[2] != prev[2] {
                    color |= 0xff_00ff00; // green
                }
            }
            prev = column;
        }
        pixbuf[i] = color;
    }

    Ok((tile_pos, pixbuf))
}
