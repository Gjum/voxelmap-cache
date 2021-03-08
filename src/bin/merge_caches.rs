extern crate docopt;
extern crate filetime;
extern crate serde;
extern crate threadpool;
extern crate voxelmap_cache;
extern crate zip;

use docopt::Docopt;
use filetime::{set_file_times, FileTime};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};
use threadpool::ThreadPool;
use voxelmap_cache::tile::{
    first_column_nr_of_chunk_nr, get_contrib_from_tile_path, get_tile_paths_in_dirs,
    get_xz_from_tile_path, is_tile_pos_in_bounds, read_tile, write_tile, KeysMap, Tile, TilePos,
    COLUMN_BYTES_MODERN,
};
use voxelmap_cache::{
    parse_bounds, ProgressTracker, CHUNK_HEIGHT, CHUNK_WIDTH, TILE_CHUNKS, TILE_COLUMNS, TILE_WIDTH,
};

const USAGE: &'static str = "
Usage: merge_caches [-q] [-t threads] [--between=<bounds>] <output-path> <cache-path>...

cache-path contains voxelmap caches in the format
`<x>,<z>,<contrib-name>.zip` or just `<x>,<z>.zip`

Options:
    -q, --quiet         Do not output info messages.
    -t, --threads       Number of threads to use for parallel processing
    --between=<bounds>  Only merge tiles at least partially within this bounding box,
                        format: w,n,e,s [default: -99999,-99999,99999,99999]
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_quiet: bool,
    arg_threads: Option<usize>,
    flag_between: String,
    arg_output_path: String,
    arg_cache_path: Vec<String>,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    let verbose = !args.flag_quiet;

    let tile_paths = get_tile_paths_in_dirs(&args.arg_cache_path, verbose).unwrap_or_else(|e| {
        println!("Error while listing cache directory: {:?}", e);
        std::process::exit(1);
    });

    let bounds = parse_bounds(&args.flag_between).unwrap_or_else(|e| {
        println!("Invalid arg: --between={} {}", &args.flag_between, e);
        std::process::exit(1);
    });

    let tile_paths: Vec<PathBuf> = tile_paths
        .into_iter()
        .filter(|path| is_tile_pos_in_bounds(get_xz_from_tile_path(path).unwrap(), &bounds))
        .collect();

    let mut tile_paths_by_pos = Box::new(HashMap::new());
    for tile_path in &tile_paths {
        let pos = get_xz_from_tile_path(&tile_path).expect("getting pos from tile path");
        tile_paths_by_pos
            .entry(pos)
            .or_insert_with(Vec::new)
            .push(tile_path.clone());
    }

    // start with most intense tile positions first (most contribs per tile pos)
    let mut paths_sorted: Vec<(TilePos, Vec<PathBuf>)> = tile_paths_by_pos.into_iter().collect();
    paths_sorted.sort_by(|(_, a), (_, b)| b.len().cmp(&a.len()));

    fs::create_dir_all(&args.arg_output_path).unwrap_or_else(|e| {
        println!(
            "Failed to create output directory {:?} {:?}",
            &args.arg_output_path, e
        );
        std::process::exit(1);
    });

    let total_work = paths_sorted.len();
    let mut progress = ProgressTracker::new(total_work);
    if verbose {
        println!(
            "Merging {:?} tiles across {:?} tile positions into {:?}",
            tile_paths.len(),
            total_work,
            &args.arg_output_path
        )
    }

    let mut skipped_contribs = HashMap::new();
    let mut total_used = 0;

    let pool = ThreadPool::new(args.arg_threads.unwrap_or(4));
    let (tx, rx) = channel();

    for (pos, tile_paths) in paths_sorted.into_iter() {
        let tx = tx.clone();
        let (x, z) = pos;
        let out_path = PathBuf::from(format!("{}/{},{}.zip", args.arg_output_path, x, z));
        pool.execute(move || {
            let result = merge_tile_from_contribs(out_path, tile_paths);
            tx.send(result).expect("Sending result");
        });
    }

    for work_done in 0..total_work {
        let (_out_path, used, skipped) = rx.recv().expect("Receiving next result");

        for (path, err) in skipped {
            let contrib = get_contrib_from_tile_path(&path)
                .unwrap_or(path.parent().unwrap().to_string_lossy().into());
            *skipped_contribs.entry(contrib.clone()).or_insert_with(|| {
                println!("Skipping contrib {:?} {}", &path, &err);
                0
            }) += 1;
        }

        total_used += used.len();

        progress.progress_to(work_done);
        if verbose {
            progress.print_progress();
        }
    }

    if verbose {
        let time_total = progress.elapsed();
        let total_min = time_total.as_secs() / 60;
        let total_sec = time_total.as_secs() % 60;
        let time_per_work_item = if total_used == 0 {
            Duration::from_millis(0)
        } else {
            time_total / total_used as u32
        };
        let tile_ms = time_per_work_item.as_secs() * 1_000
            + time_per_work_item.subsec_nanos() as u64 / 1_000_000;
        println!(
            "Done merging. Took {}:{:02} for all {} used tiles, {}ms per tile",
            total_min, total_sec, total_used, tile_ms,
        );
    };
}

pub fn merge_tile_from_contribs(
    out_path: PathBuf,
    tile_paths: Vec<PathBuf>,
) -> (PathBuf, Vec<PathBuf>, Vec<(PathBuf, String)>) {
    if tile_paths.len() == 1 {
        // just one contrib, no merging needed, hardlink it to destination
        let tile_path = tile_paths.into_iter().next().unwrap();
        return match std::fs::hard_link(&tile_path, &out_path) {
            Ok(()) => (out_path, vec![tile_path], Vec::new()),
            Err(e) => (out_path, Vec::new(), vec![(tile_path, e.to_string())]),
        };
    }

    let mut sorted_paths: Vec<(SystemTime, PathBuf)> = tile_paths
        .into_iter()
        .map(|path| (fs::metadata(&path).unwrap().modified().unwrap(), path))
        .collect();
    // sort most recent first
    sorted_paths.sort_by(|(mtime_a, _), (mtime_b, _)| mtime_b.cmp(mtime_a));
    // earliest/least recent mtime
    let min_mtime = sorted_paths.last().expect("contribs non-empty").0;

    let mut used = Vec::new();
    let mut skipped = Vec::new();

    let mut out_tile = Box::new(Tile {
        // source: Some(out_path.clone()),
        // pos: get_xz_from_tile_path(&out_path).ok(),
        pos: None,
        data: vec![0; TILE_COLUMNS * COLUMN_BYTES_MODERN],
        keys: HashMap::new(),
        names: vec![], // HACK: unused
        source: None,
    });

    let mut num_chunks_left = TILE_CHUNKS;
    let mut chunks_done = vec![false; num_chunks_left];

    // most recent to least recent
    for (_mtime, tile_path) in sorted_paths {
        let result = read_tile(&tile_path)
            .and_then(|under_tile| merge_two_tiles(&mut out_tile, &under_tile, &mut chunks_done));
        num_chunks_left -= match result {
            Ok(chunks_processed) => {
                used.push(tile_path);
                chunks_processed
            }
            Err(e) => {
                skipped.push((tile_path, e));
                0
            }
        };

        if num_chunks_left <= 0 {
            break;
        }
    }

    if let Err(e) = write_tile(&out_path, &out_tile) {
        println!("Failed writing {:?} {:?}", &out_path, e);
    }

    set_file_times(
        &out_path,
        FileTime::from(min_mtime),
        FileTime::from(min_mtime),
    )
    .expect("Setting mtime");

    (out_path, used, skipped)
}

fn merge_two_tiles(
    out_tile: &mut Tile,
    under_tile: &Tile,
    chunks_done: &mut Vec<bool>,
) -> Result<usize, String> {
    let mut converter = merge_keys_and_build_converter(&mut out_tile.keys, &under_tile.keys);

    let mut chunks_processed = 0;

    for chunk_nr in 0..TILE_CHUNKS {
        if chunks_done[chunk_nr] || under_tile.is_chunk_empty(chunk_nr) {
            continue;
        }

        copy_convert_chunk(&mut converter, out_tile, under_tile, chunk_nr)
            .map_err(|e| e.to_string())?;

        chunks_done[chunk_nr] = true;
        chunks_processed += 1;
    }

    Ok(chunks_processed)
}

type BlockIdConverter = Vec<u16>;

fn merge_keys_and_build_converter(keys_out: &mut KeysMap, keys_in: &KeysMap) -> BlockIdConverter {
    let len_in = 1 + *keys_in.values().max().unwrap_or(&0) as usize;
    let mut next_id = keys_out.len() as u16;
    let mut converter = vec![0; len_in];
    for (name, in_id) in keys_in {
        let out_id = keys_out.entry(name.clone()).or_insert_with(|| {
            next_id += 1; // voxelmap starts at 1
            next_id
        });
        converter[*in_id as usize] = *out_id;
    }
    converter
}

fn copy_convert_chunk(
    converter: &mut BlockIdConverter,
    out_tile: &mut Tile,
    under_tile: &Tile,
    chunk_nr: usize,
) -> Result<(), String> {
    let first_chunk_column = first_column_nr_of_chunk_nr(chunk_nr);
    for z_in_chunk in 0..CHUNK_HEIGHT {
        let line_start = first_chunk_column + z_in_chunk * TILE_WIDTH;
        for x_in_chunk in 0..CHUNK_WIDTH {
            let column_nr = line_start + x_in_chunk;

            out_tile.set_height(column_nr, under_tile.get_height(column_nr));
            out_tile.set_light(column_nr, under_tile.get_light(column_nr));
            out_tile
                .set_ocean_floor_height(column_nr, under_tile.get_ocean_floor_height(column_nr));
            out_tile.set_ocean_floor_light(column_nr, under_tile.get_ocean_floor_light(column_nr));
            out_tile
                .set_transparent_height(column_nr, under_tile.get_transparent_height(column_nr));
            out_tile.set_transparent_light(column_nr, under_tile.get_transparent_light(column_nr));
            out_tile.set_foliage_height(column_nr, under_tile.get_foliage_height(column_nr));
            out_tile.set_foliage_light(column_nr, under_tile.get_foliage_light(column_nr));
            out_tile.set_biome_id(column_nr, under_tile.get_biome_id(column_nr));

            out_tile.set_blockstate(
                column_nr,
                converter[under_tile.get_blockstate(column_nr) as usize],
            );
            out_tile.set_ocean_floor_blockstate(
                column_nr,
                converter[under_tile.get_ocean_floor_blockstate(column_nr) as usize],
            );
            out_tile.set_transparent_blockstate(
                column_nr,
                converter[under_tile.get_transparent_blockstate(column_nr) as usize],
            );
            out_tile.set_foliage_blockstate(
                column_nr,
                converter[under_tile.get_foliage_blockstate(column_nr) as usize],
            );
        }
    }
    Ok(())
}
