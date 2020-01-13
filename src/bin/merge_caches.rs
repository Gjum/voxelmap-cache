extern crate docopt;
extern crate serde;
extern crate threadpool;
extern crate voxelmap_cache;
extern crate zip;

use docopt::Docopt;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use threadpool::ThreadPool;
use voxelmap_cache::tile::{
    get_chunk_start, get_contrib_from_tile_path, get_tile_paths_in_dirs, get_xz_from_tile_path,
    is_tile_pos_in_bounds, read_tile, write_tile, KeysMap, Tile, TilePos, COLUMN_BYTES,
};
use voxelmap_cache::{
    get_block_name_from_voxelmap, get_mtime_or_0, parse_bounds, ProgressTracker, CHUNK_HEIGHT,
    CHUNK_WIDTH, TILE_CHUNKS, TILE_COLUMNS, TILE_WIDTH,
};

const USAGE: &'static str = "
Usage: merge_caches [-q] [-t threads] [--between=<bounds>] <output-path> <cache-path>...

cache-path contains voxelmap caches in the format `<x>,<z>,<contrib-name>.zip`

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

    // start with most intense tile positions first (most contribs per tile)
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
            let result = merge_all_tiles(out_path, tile_paths);
            tx.send(result).expect("Sending result");
        });
    }

    for work_done in 0..total_work {
        let (_out_path, used, skipped) = rx.recv().expect("Receiving next result");

        for (path, err) in skipped {
            let contrib = get_contrib_from_tile_path(&path).unwrap_or(String::new());
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
        let time_per_work_item = time_total / total_used as u32;
        let tile_ms = time_per_work_item.as_secs() * 1_000
            + time_per_work_item.subsec_nanos() as u64 / 1_000_000;
        println!(
            "Done merging. Took {}:{:02} for all {} used tiles, {}ms per tile",
            total_min, total_sec, total_used, tile_ms,
        );
    };
}

pub fn merge_all_tiles(
    out_path: PathBuf,
    tile_paths: Vec<PathBuf>,
) -> (PathBuf, Vec<PathBuf>, Vec<(PathBuf, String)>) {
    // XXX set out_path mtime to newest used tile_path

    if tile_paths.len() == 1 {
        // just one contrib, no merging needed, hardlink it to destination
        let tile_path = tile_paths.into_iter().next().unwrap();
        return match std::fs::hard_link(&tile_path, &out_path) {
            Ok(()) => (out_path, vec![tile_path], Vec::new()),
            Err(e) => (out_path, Vec::new(), vec![(tile_path, e.to_string())]),
        };
    }

    let mut sorted_paths: Vec<(u64, PathBuf)> = tile_paths
        .into_iter()
        .map(|path| (get_mtime_or_0(&path), path))
        .collect();
    // sort most recent first
    sorted_paths.sort_by(|(mtime_a, _), (mtime_b, _)| mtime_b.cmp(mtime_a));

    let mut used = Vec::new();
    let mut skipped = Vec::new();

    let mut out_tile = Box::new(Tile {
        // source: Some(out_path.clone()),
        // pos: get_xz_from_tile_path(&out_path).ok(),
        source: None,
        pos: None,
        keys: Some(HashMap::new()),
        names: None, // dummy
        columns: vec![0; TILE_COLUMNS * COLUMN_BYTES],
    });

    let mut num_chunks_left = TILE_CHUNKS;
    let mut chunks_done = vec![false; num_chunks_left];

    for (_mtime, tile_path) in sorted_paths {
        let result = read_tile(&tile_path)
            .and_then(|tile| merge_two_tiles(&tile, &mut out_tile, &mut chunks_done));
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

    (out_path, used, skipped)
}

fn merge_two_tiles(
    tile: &Tile,
    out_tile: &mut Tile,
    chunks_done: &mut Vec<bool>,
) -> Result<usize, String> {
    let mut converter = match tile.keys {
        Some(ref keys) => {
            merge_keys_and_build_converter(&mut out_tile.keys.as_mut().unwrap(), &keys)
        }
        None => vec![0; 4096],
    };

    let mut chunks_processed = 0;

    for chunk_nr in 0..TILE_CHUNKS {
        if chunks_done[chunk_nr] || tile.is_unset(get_chunk_start(chunk_nr)) {
            continue;
        }

        copy_convert_chunk(chunk_nr, &tile, out_tile, &mut converter).map_err(|e| e.to_string())?;

        chunks_done[chunk_nr] = true;
        chunks_processed += 1;
    }

    Ok(chunks_processed)
}

fn merge_keys_and_build_converter(keys_out: &mut KeysMap, keys_in: &KeysMap) -> Vec<u16> {
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
    chunk_nr: usize,
    in_tile: &Tile,
    out_tile: &mut Tile,
    converter: &mut Vec<u16>,
) -> Result<(), String> {
    let chunk_start = get_chunk_start(chunk_nr);
    for line_nr in 0..CHUNK_HEIGHT {
        let line_start = chunk_start + line_nr * TILE_WIDTH * COLUMN_BYTES;

        {
            let in_slice = &in_tile.columns[line_start..line_start + CHUNK_WIDTH * COLUMN_BYTES];
            let out_slice =
                &mut out_tile.columns[line_start..line_start + CHUNK_WIDTH * COLUMN_BYTES];
            out_slice.copy_from_slice(in_slice);
        }

        for column_nr in 0..CHUNK_WIDTH {
            let column_start = line_start + column_nr * COLUMN_BYTES;
            for block_nr in 0..4 {
                let block_start = column_start + block_nr * 4;

                let out_block_id = match in_tile.keys {
                    Some(ref _keys) => {
                        let in_block_id = (out_tile.columns[block_start + 1] as u16) << 8
                            | (out_tile.columns[block_start + 2] as u16);

                        if in_block_id == 0 {
                            continue; // no data here
                        }

                        if in_block_id as usize >= converter.len() {
                            return Err(format!(
                                "Block id {} outside range {} - file corrupted?",
                                in_block_id,
                                converter.len(),
                            ));
                        }

                        match converter[in_block_id as usize] {
                            0 => {
                                // return Err(format!(
                                panic!(format!(
                                    "Block id {} not in converter - logic error",
                                    in_block_id,
                                ));
                            }
                            out_block_id => out_block_id,
                        }
                    }
                    None => {
                        let in_block_id = (out_tile.columns[block_start + 2] as u16) << 4
                            | (out_tile.columns[block_start + 1] as u16) >> 4;

                        if in_block_id == 0 {
                            continue; // no data here
                        }

                        match converter[in_block_id as usize] {
                            0 => {
                                let name = get_block_name_from_voxelmap(
                                    out_tile.columns[block_start + 1],
                                    out_tile.columns[block_start + 2],
                                ).to_string();

                                let out_keys = out_tile.keys.as_mut().unwrap();
                                let next_id = 1 + out_keys.len() as u16;
                                let out_block_id = out_keys.entry(name).or_insert(next_id);

                                converter[in_block_id as usize] = *out_block_id;

                                *out_block_id
                            }
                            out_block_id => out_block_id,
                        }
                    }
                };

                // TODO is branching faster than blindly writing?
                // if out_block_id != in_block_id {
                out_tile.columns[block_start + 1] = (out_block_id >> 8) as u8;
                out_tile.columns[block_start + 2] = (out_block_id & 0xff) as u8;
                // }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_convert_chunk_works_for_global_key() {
        let mut in_tile = Tile {
            source: None,
            pos: None,
            columns: vec![0_u8; TILE_COLUMNS * COLUMN_BYTES],
            keys: None,
            names: None,
        };
        let mut out_tile = Tile {
            source: None,
            pos: None,
            columns: vec![0_u8; TILE_COLUMNS * COLUMN_BYTES],
            keys: Some(HashMap::new()),
            names: None, // dummy
        };
        let mut converter = vec![0; 4096];

        let foo = 17 * (256 * 2 * 16 + 16);
        in_tile.columns[foo + 0] = 2; // height
        in_tile.columns[foo + 1] = 1;
        in_tile.columns[foo + 2] = 1;
        in_tile.columns[foo + 3] = 14; // light
        in_tile.columns[foo + 16] = 23; // biome

        let bar = foo + 17 * (256 + 2);
        in_tile.columns[bar + 1] = 1;
        in_tile.columns[bar + 2] = 1;

        let baz = foo + 17 * (256 + 3);
        in_tile.columns[baz + 1] = 0;
        in_tile.columns[baz + 2] = 2;

        assert_eq!(
            Ok(()),
            copy_convert_chunk(33, &in_tile, &mut out_tile, &mut converter)
        );

        // biome, height, light are copied
        assert_eq!(23, out_tile.columns[foo + 16]);
        assert_eq!(2, out_tile.columns[foo + 0]);
        assert_eq!(14, out_tile.columns[foo + 3]);

        // foo block is first in out key
        assert_eq!(0, out_tile.columns[foo + 1]);
        assert_eq!(1, out_tile.columns[foo + 2]);

        // null is still null
        assert_eq!(0, out_tile.columns[foo + 1 + 17]);
        assert_eq!(0, out_tile.columns[foo + 2 + 17]);

        // ids get reused
        assert_eq!(0, out_tile.columns[bar + 1]);
        assert_eq!(1, out_tile.columns[bar + 2]);

        // baz block is second entry after foo/bar
        assert_eq!(0, out_tile.columns[baz + 1]);
        assert_eq!(2, out_tile.columns[baz + 2]);
    }

    #[test]
    fn merge_two_tiles_works_for_tile_key() {
        let mut in_keys = HashMap::new();
        in_keys.insert("test id 42".to_string(), 42);
        in_keys.insert("test id 123".to_string(), 123);

        let mut in_tile = Tile {
            source: None,
            pos: None,
            columns: vec![0_u8; TILE_COLUMNS * COLUMN_BYTES],
            keys: Some(in_keys),
            names: None, // dummy
        };
        let mut out_tile = Tile {
            source: None,
            pos: None,
            columns: vec![0_u8; TILE_COLUMNS * COLUMN_BYTES],
            keys: Some(HashMap::new()),
            names: None, // dummy
        };
        let mut chunks_done = vec![false; TILE_CHUNKS];

        let foo = 17 * (256 * 2 * 16 + 16);
        in_tile.columns[foo + 0] = 2; // height
        in_tile.columns[foo + 1] = 0;
        in_tile.columns[foo + 2] = 42;
        in_tile.columns[foo + 3] = 14; // light
        in_tile.columns[foo + 16] = 23; // biome

        let bar = foo + 17 * (256 + 2);
        in_tile.columns[bar + 1] = 0;
        in_tile.columns[bar + 2] = 42;

        let baz = foo + 17 * (256 + 3);
        in_tile.columns[baz + 1] = 0;
        in_tile.columns[baz + 2] = 123;

        assert_eq!(
            Ok(1),
            merge_two_tiles(&in_tile, &mut out_tile, &mut chunks_done)
        );

        // TODO update tests

        // // biome, height, light are copied
        // assert_eq!(23, out_tile.columns[foo + 16]);
        // assert_eq!(2, out_tile.columns[foo + 0]);
        // assert_eq!(14, out_tile.columns[foo + 3]);

        // // foo block is first in out key
        // assert_eq!(0, out_tile.columns[foo + 1]);
        // assert_eq!(1, out_tile.columns[foo + 2]);

        // // null is still null
        // assert_eq!(0, out_tile.columns[foo + 1 + 17]);
        // assert_eq!(0, out_tile.columns[foo + 2 + 17]);

        // // ids get reused
        // assert_eq!(0, out_tile.columns[bar + 1]);
        // assert_eq!(1, out_tile.columns[bar + 2]);

        // // baz block is second entry after foo/bar
        // assert_eq!(0, out_tile.columns[baz + 1]);
        // assert_eq!(2, out_tile.columns[baz + 2]);
    }
}
