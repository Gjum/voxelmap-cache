extern crate docopt;
extern crate serde;
extern crate threadpool;
extern crate voxelmap_cache;
extern crate zip;

use docopt::Docopt;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use threadpool::ThreadPool;
use voxelmap_cache::tile::{get_tile_paths_in_dirs, read_tile, Tile};
use voxelmap_cache::ProgressTracker;
use voxelmap_cache::{biomes::BIOME_NAMES, tile::TILE_COLUMNS};

const USAGE: &'static str = "
Usage: blockcount [-q] [-t threads] <cache-path>

cache-path contains voxelmap caches in the format `<x>,<z>.zip`

Options:
    -q, --quiet         Do not output info messages.
    -t, --threads       Number of threads to use for parallel processing
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_cache_path: String,
    flag_quiet: bool,
    arg_threads: Option<usize>,
}

/// (biome, block) -> count
type BiomeBlockCounts = HashMap<(u16, String), usize>;

fn new_biome_block_counts() -> BiomeBlockCounts {
    HashMap::new()
}
fn merge_biome_block_counts_into(counts: &mut BiomeBlockCounts, other: &BiomeBlockCounts) {
    for (key, val) in other.iter() {
        *counts.entry(key.clone()).or_insert(0) += val;
    }
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    let verbose = !args.flag_quiet;

    let tile_paths = get_tile_paths_in_dirs(&vec![args.arg_cache_path.clone()], verbose)
        .unwrap_or_else(|e| {
            eprintln!("Error while listing cache directory: {:?}", e);
            std::process::exit(1);
        });

    let tile_paths: Vec<PathBuf> = tile_paths.into_iter().collect();

    let total_work = tile_paths.len();
    let mut progress = ProgressTracker::new(total_work);
    if verbose {
        eprintln!("Counting blocks in {:?} tiles", total_work)
    }

    let pool = ThreadPool::new(args.arg_threads.unwrap_or(4));
    let (tx, rx) = channel();

    // let global_keys_map = Arc::new(build_global_keys_map());

    for tile_path in tile_paths.into_iter() {
        let tx = tx.clone();
        // let global_keys_map = global_keys_map.clone();
        pool.execute(move || {
            let result = count_tile(&tile_path); //, &global_keys_map);
            tx.send((tile_path, result)).expect("Sending result");
        });
    }

    let mut counts = new_biome_block_counts();

    for work_done in 0..total_work {
        let result_with_path = rx.recv().expect("Receiving next result");
        let (tile_path, result) = result_with_path;
        if let Err(msg) = result {
            eprintln!("Failed counting tile {:?} {}", tile_path, msg);
            return;
        }
        let tile_counts = result.unwrap();

        merge_biome_block_counts_into(&mut counts, &tile_counts);

        progress.progress_to(work_done);
        if verbose {
            progress.print_progress();
        }
    }

    let mut biome_counts = [0; 256];
    for ((biome_id, _block_name), count) in counts.iter() {
        biome_counts[*biome_id as usize] += count;
    }

    // let counts_array = counts.iter().array();
    // counts_array.sort_unstable_by_key(|((biome_id, block_name), count)| (biome_id, block_name, count));
    // for (biome_id, block_name, count) in counts_array.iter() {
    for ((biome_id, block_name), count) in counts.iter() {
        let biome_name = BIOME_NAMES[*biome_id as usize];
        let rel_count = *count as f32 / biome_counts[*biome_id as usize] as f32;
        // println!("{}\t{}\t{:10}\t{}", biome_name, biome_id, count, block_name);
        println!(
            "{}\t{}\t{}\t{}\t{}",
            rel_count, count, block_name, biome_name, biome_id
        );
    }

    if verbose {
        let time_total = progress.elapsed();
        let total_min = time_total.as_secs() / 60;
        let total_sec = time_total.as_secs() % 60;
        let time_per_work_item = time_total / total_work as u32;
        let tile_ms = time_per_work_item.as_secs() * 1_000
            + time_per_work_item.subsec_nanos() as u64 / 1_000_000;
        eprintln!(
            "Done counting. Took {}:{:02} for all {} tiles, {}ms per tile",
            total_min, total_sec, total_work, tile_ms,
        );
    };
}

// fn count_tile(tile_path: &PathBuf, global_keys_map: &KeysMap) -> Result<BiomeBlockCounts, String> {
fn count_tile(tile_path: &PathBuf) -> Result<BiomeBlockCounts, String> {
    let tile = read_tile(tile_path).map_err(|e| e.to_string())?;

    let mut counts = new_biome_block_counts();

    let steps_block_getters: Vec<fn(&Tile, usize) -> u16> = vec![
        Tile::get_blockstate,
        Tile::get_ocean_floor_blockstate,
        Tile::get_transparent_blockstate,
        Tile::get_foliage_blockstate,
    ];

    for column_nr in 0..TILE_COLUMNS {
        let biome = tile.get_biome_id(column_nr);
        for get_block_nr in &steps_block_getters {
            let block_nr = get_block_nr(&tile, column_nr) as usize;
            if block_nr != 0 {
                let block_name_full = tile.names.get(block_nr).unwrap().to_string();
                let block_name_stem = block_name_full.split("[").next().unwrap().to_string();

                *counts.entry((biome, block_name_stem)).or_insert(0) += 1;
            }
        }
    }

    Ok(counts)
}
