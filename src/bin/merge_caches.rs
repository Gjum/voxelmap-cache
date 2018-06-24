extern crate docopt;
extern crate rustc_serialize;
extern crate threadpool;
extern crate zip;

use docopt::Docopt;
use std::collections::{HashMap, LinkedList};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::{Instant, SystemTime};
use threadpool::ThreadPool;

const USAGE: &'static str = "
Usage: merge_caches [-q] [-t threads] [--between=<bounds>] <output-path> <cache-path>...

cache-path contains voxelmap caches in the format `<x>,<z>,<contrib-name>.zip`

Options:
    -q, --quiet  Do not output info messages.
    -t           Number of threads to use for parallel processing
    --between=<bounds>  Only merge tiles at least partially within this bounding box,
                        format: w,n,e,s [default: -99999,-99999,99999,99999]
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_quiet: bool,
    arg_threads: Option<usize>,
    flag_between: String,
    arg_output_path: String,
    arg_cache_path: Vec<String>,
}

fn main() {
    let start_time = Instant::now();

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    let verbose = !args.flag_quiet;

    let tile_paths = get_tile_paths_in_dirs(&args.arg_cache_path, verbose).unwrap_or_else(|e| {
        println!("Error while listing cache directory: {:?}", e);
        std::process::exit(1);
    });

    let bounds = args
        .flag_between
        .splitn(4, ",")
        .map(str::parse)
        .collect::<Result<Vec<i32>, _>>()
        .unwrap_or_else(|e| {
            println!(
                "Invalid arg: between: {} {}",
                &args.flag_between,
                e.to_string()
            );
            std::process::exit(1);
        });
    if bounds.len() != 4 || bounds[0] > bounds[2] || bounds[1] > bounds[3] {
        println!(
            "Invalid arg: between: {} should be: w,n,e,s",
            &args.flag_between
        );
        std::process::exit(1);
    }

    let tile_paths: Vec<PathBuf> = tile_paths
        .into_iter()
        .filter(|path| {
            let (tile_x, tile_z) = get_xz_from_tile_path(path).expect("getting pos from tile path");
            let tw = TILE_WIDTH as i32;
            let th = TILE_HEIGHT as i32;
            let x = tile_x * tw;
            let z = tile_z * th;
            let (w, n, e, s) = (bounds[0], bounds[1], bounds[2], bounds[3]);
            x + tw > w && x < e && z + th > n && z < s
        })
        .collect();

    // TODO box this? to prevent stack from overflowing
    let mut tile_paths_by_pos: HashMap<TilePos, Vec<PathBuf>> = HashMap::new();
    for tile_path in &tile_paths {
        let pos = get_xz_from_tile_path(&tile_path).expect("getting pos from tile path");
        tile_paths_by_pos
            .entry(pos)
            .or_insert_with(Vec::new)
            .push(tile_path.clone());
    }

    let total_work = tile_paths_by_pos.len();
    if verbose {
        println!(
            "Merging {:?} tiles across {:?} tile positions",
            tile_paths.len(),
            total_work
        )
    }

    // start with most intense tile positions first (most contribs per tile)
    let mut sorted_by_tiles_per_pos: Vec<(TilePos, Vec<PathBuf>)> =
        tile_paths_by_pos.into_iter().collect();
    sorted_by_tiles_per_pos.sort_by(|(_, a), (_, b)| b.len().cmp(&a.len()));

    fs::create_dir_all(&args.arg_output_path).unwrap_or_else(|e| {
        println!(
            "Failed to create output directory {:?} {:?}",
            &args.arg_output_path, e
        );
        std::process::exit(1);
    });

    let mut skipped_contribs = HashMap::new();
    let mut total_used = 0;

    let mut next_msg_elapsed = PROGRESS_INTERVAL;

    // TODO >>>>>>>>>>

    // let pool = ThreadPool::new(args.arg_threads.unwrap_or(4));
    // let (tx, rx) = channel();

    // for (pos, tile_paths) in sorted_by_tiles_per_pos.into_iter() {
    //     let tx = tx.clone();
    //     let (x, z) = pos;
    //     let out_path = PathBuf::from(format!("{}/{},{}.zip", args.arg_output_path, x, z));
    //     pool.execute(move || {
    //         let result = merge_tiles(out_path, tile_paths);
    //         tx.send(result).expect("Sending result");
    //     });
    // }

    // for work_done in 0..total_work {
    //     let (_out_path, used, skipped) = rx.recv().expect("Receiving next result");

    // TODO ==========

    for (work_done, (pos, tile_paths)) in sorted_by_tiles_per_pos.into_iter().enumerate() {
        let (x, z) = pos;
        let out_path = PathBuf::from(format!("{}/{},{}.zip", args.arg_output_path, x, z));
        let (_out_path, used, skipped) = merge_tiles(out_path, tile_paths);

        // TODO <<<<<<<<<<

        for (path, err) in skipped {
            let contrib = get_contrib_from_tile_path(&path).unwrap_or(String::new());
            *skipped_contribs.entry(contrib.clone()).or_insert_with(|| {
                println!("Skipping contrib {} {}", &contrib, &err);
                0
            }) += 1;
        }

        total_used += used.len();

        if verbose {
            print_progress(work_done, total_work, start_time, &mut next_msg_elapsed);
        }
    }

    if verbose {
        let time_total = start_time.elapsed();
        let total_min = time_total.as_secs() / 60;
        let total_sec = time_total.as_secs() % 60;
        let time_per_work_item = time_total / total_work as u32;
        let tile_ms = time_per_work_item.as_secs() * 1_000
            + time_per_work_item.subsec_nanos() as u64 / 1_000_000;
        println!(
            "Done merging. Took {}:{:02} for all {} tiles, {}ms per tile",
            total_min, total_sec, total_used, tile_ms,
        );
    };
}

pub fn merge_tiles(
    out_path: PathBuf,
    tile_paths: Vec<PathBuf>,
) -> (PathBuf, Vec<PathBuf>, Vec<(PathBuf, String)>) {
    if tile_paths.len() == 1 {
        let tile_path = tile_paths.into_iter().next().unwrap();
        println!("Hardlinking {:?}", &tile_path);
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
        keys: HashMap::new(),
        columns: [0; TILE_COLUMNS * COLUMN_BYTES],
    });

    let mut num_chunks_left = TILE_CHUNKS;
    let mut chunks_done = vec![false; num_chunks_left];

    // copy data for first tile, usually speeds up many things
    let mut sorted_paths_iter = sorted_paths.into_iter();
    let (_mtime, tile_path) = sorted_paths_iter
        .next()
        .expect("getting first path for pos");
    match read_tile(&tile_path) {
        Ok(tile) => {
            used.push(tile_path);
            out_tile.columns[..].copy_from_slice(&tile.columns[..]);
            out_tile.keys.extend(tile.keys);
        }
        Err(e) => {
            skipped.push((tile_path, e));
        }
    };

    for (_mtime, tile_path) in sorted_paths_iter {
        let result = read_tile(&tile_path).and_then(|tile| {
            let converter = merge_keys(&mut out_tile.keys, &tile.keys);

            for chunk_nr in 0..TILE_CHUNKS {
                if chunks_done[chunk_nr] || tile.is_chunk_unset(chunk_nr) {
                    continue;
                }

                copy_convert_chunk(chunk_nr, &converter, &tile.columns, &mut out_tile.columns)
                    .map_err(|e| e.to_string())?;

                chunks_done[chunk_nr] = true;
                num_chunks_left -= 1;
            }

            Ok(())
        });
        match result {
            Ok(()) => used.push(tile_path),
            Err(e) => skipped.push((tile_path, e)),
        }

        if num_chunks_left <= 0 {
            break;
        }
    }

    if let Err(e) = write_tile(&out_path, &out_tile) {
        println!("Failed writing {:?} {:?}", &out_path, e);
    }

    (out_path, used, skipped)
}

fn merge_keys(keys_out: &mut HashMap<String, u16>, keys_in: &HashMap<String, u16>) -> Vec<u16> {
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
    converter: &Vec<u16>,
    in_data: &TileData,
    out_data: &mut TileData,
) -> Result<(), String> {
    let chunk_start = get_chunk_start(chunk_nr);
    for line_nr in 0..CHUNK_HEIGHT {
        let line_start = chunk_start + line_nr * TILE_WIDTH * COLUMN_BYTES;

        {
            let in_slice = &in_data[line_start..line_start + CHUNK_WIDTH * COLUMN_BYTES];
            let out_slice = &mut out_data[line_start..line_start + CHUNK_WIDTH * COLUMN_BYTES];
            out_slice.copy_from_slice(in_slice);
        }

        for column_nr in 0..CHUNK_WIDTH {
            let column_start = line_start + column_nr * COLUMN_BYTES;
            for block_nr in 0..4 {
                let block_start = column_start + block_nr * 4;
                let in_block_id =
                    (out_data[block_start + 1] as u16) << 8 | (out_data[block_start + 2] as u16);

                if in_block_id as usize >= converter.len() {
                    return Err(format!(
                        "Block id {} outside range {} - file corrupted?",
                        in_block_id,
                        converter.len()
                    ));
                }

                let out_block_id = converter[in_block_id as usize];
                // TODO is branching slower than blindly writing?
                if out_block_id != in_block_id {
                    out_data[block_start + 1] = (out_block_id >> 8) as u8;
                    out_data[block_start + 2] = (out_block_id & 0xff) as u8;
                }
            }
        }
    }

    Ok(())
}

const PROGRESS_INTERVAL: u64 = 3;

// TODO put more weight on recent measurements
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
    *next_msg_elapsed += PROGRESS_INTERVAL;

    let work_left = total - done;
    let sec_left = elapsed as usize * work_left / done;
    let min = sec_left / 60;
    let sec = sec_left % 60;
    println!("{}/{} processed, {}:{:02?} left", done, total, min, sec);
}

pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_HEIGHT: usize = 16;
pub const CHUNK_BLOCKS: usize = CHUNK_WIDTH * CHUNK_HEIGHT;
pub const CHUNKS_PER_TILE_WIDTH: usize = 16;
pub const CHUNKS_PER_TILE_HEIGHT: usize = 16;
pub const TILE_WIDTH: usize = CHUNKS_PER_TILE_WIDTH * CHUNK_WIDTH;
pub const TILE_HEIGHT: usize = CHUNKS_PER_TILE_HEIGHT * CHUNK_HEIGHT;
pub const TILE_COLUMNS: usize = TILE_WIDTH * TILE_HEIGHT;
pub const TILE_CHUNKS: usize = CHUNKS_PER_TILE_WIDTH * CHUNKS_PER_TILE_HEIGHT;
pub const COLUMN_BYTES: usize = 17;

pub type TilePos = (i32, i32);
pub type TileData = [u8; TILE_COLUMNS * COLUMN_BYTES];

pub struct Tile {
    source: Option<PathBuf>,
    pos: Option<TilePos>,
    columns: TileData,
    keys: HashMap<String, u16>,
}

const AIR_STR: &str = "minecraft:air";

fn get_chunk_start(chunk_nr: usize) -> usize {
    let chunk_start_col = (chunk_nr * CHUNK_WIDTH) % TILE_WIDTH
        + (chunk_nr * CHUNK_WIDTH / TILE_WIDTH) * TILE_WIDTH * CHUNK_HEIGHT;
    chunk_start_col * COLUMN_BYTES
}

impl Tile {
    fn is_chunk_unset(&self, chunk_nr: usize) -> bool {
        let chunk_start = get_chunk_start(chunk_nr);
        let height = self.columns[chunk_start];
        let block_nr =
            (self.columns[chunk_start + 1] as u16) << 8 | (self.columns[chunk_start + 2] as u16);
        let is_air = self
            .keys
            .get(AIR_STR)
            .map_or(true, |air_nr| *air_nr == block_nr);
        return height == 0 && is_air;
    }
}

impl std::fmt::Debug for Tile {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt.write_str(&format!(
            "Tile with {} keys{}{}",
            self.keys.len(),
            match &self.pos {
                Some(pos) => format!(" at {:?}", pos),
                None => "".to_owned(),
            },
            match &self.source {
                Some(source) => format!(" from {:?}", source),
                None => "".to_owned(),
            },
        ))
    }
}

pub fn read_tile(tile_path: &PathBuf) -> Result<Box<Tile>, String> {
    use std::io::{BufRead, BufReader, Read};

    let zip_file = fs::File::open(&tile_path).map_err(|e| e.to_string())?;
    let mut zip_archive = zip::ZipArchive::new(zip_file).map_err(|e| e.to_string())?;

    let mut tile = Box::new(Tile {
        source: Some(tile_path.clone()),
        pos: get_xz_from_tile_path(tile_path).ok(),
        columns: [0; TILE_COLUMNS * COLUMN_BYTES],
        keys: HashMap::new(),
    });

    {
        // TODO convert key from old VoxelMap format
        let key_file = zip_archive
            .by_name("key")
            .map_err(|_e| "Old VoxelMap format (no key file in zip)")?;

        // TODO which one is faster?
        // let mut key_text = String::new();
        // key_file.read_to_string(&mut key_text);
        // for line in key_text.split("\r\n") {
        for line in BufReader::new(key_file).lines() {
            let line = line.unwrap();
            if line.is_empty() {
                continue;
            }
            let mut split = line.split(" ");
            let block_nr = split
                .next()
                .expect("getting block num from key line split")
                .parse::<u16>()
                .expect("converting block num to int");
            let block_name = split
                .next()
                .expect("getting block name from key line split")
                .to_string();
            tile.keys.insert(block_name, block_nr);
        }
    }
    {
        let mut data_file = zip_archive
            .by_name("data")
            .map_err(|_e| "No data file in tile zip")?;
        data_file
            .read_exact(&mut tile.columns)
            .map_err(|e| e.to_string())?;
    }

    Ok(tile)
}

pub fn write_tile(tile_path: &PathBuf, tile: &Tile) -> Result<(), String> {
    use std::io::Write;

    let zip_file = fs::File::create(&tile_path).map_err(|e| e.to_string())?;
    let mut zip_archive = zip::ZipWriter::new(zip_file);

    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip_archive
        .start_file("data", options)
        .map_err(|e| e.to_string())?;
    zip_archive
        .write_all(&tile.columns)
        .map_err(|e| e.to_string())?;

    zip_archive
        .start_file("key", options)
        .map_err(|e| e.to_string())?;
    for (name, nr) in &tile.keys {
        zip_archive
            .write_fmt(format_args!("{} {}\r\n", nr, name))
            .map_err(|e| e.to_string())?;
    }

    // Optionally finish the zip. (this is also done on drop)
    zip_archive.finish().map_err(|e| e.to_string())?;

    Ok(())
}

fn get_xz_from_tile_path(tile_path: &PathBuf) -> Result<TilePos, String> {
    let fname = tile_path.file_name().unwrap().to_str().unwrap();
    if fname.len() <= 4 {
        return Err("file name too short".to_owned());
    }
    let (coords_part, _) = fname.split_at(fname.len() - 4);
    let mut it = coords_part.splitn(3, ',');
    let x = it
        .next()
        .unwrap()
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let z = it
        .next()
        .unwrap()
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    Ok((x, z))
}

fn get_contrib_from_tile_path(tile_path: &PathBuf) -> Result<String, String> {
    let fname = tile_path.file_name().unwrap().to_str().unwrap();
    if fname.len() <= 4 {
        return Err("file name too short".to_owned());
    }
    let (coords_part, _) = fname.split_at(fname.len() - 4);
    Ok(coords_part
        .splitn(3, ',')
        .skip(2)
        .next()
        .unwrap()
        .to_string())
}

pub fn get_tile_paths_in_dirs(
    dirs: &Vec<String>,
    verbose: bool,
) -> Result<LinkedList<PathBuf>, String> {
    let mut tile_paths = LinkedList::new();
    for dir in dirs {
        for zip_dir_entry in fs::read_dir(dir.as_str()).map_err(|e| e.to_string())? {
            let tile_path = zip_dir_entry.map_err(|e| e.to_string())?.path();
            match get_xz_from_tile_path(&tile_path) {
                Ok(_pos) => tile_paths.push_back(tile_path),
                Err(e) => {
                    if tile_path.to_string_lossy().ends_with("_chunk-times.gz") {
                        // ignore chunk timestamp info file
                    } else {
                        if verbose {
                            println!("Ignoring non-tile file {:?} {:?}", &tile_path, e);
                        }
                    }
                }
            }
        }
    }
    Ok(tile_paths)
}

fn get_mtime_or_0(path: &PathBuf) -> u64 {
    fs::metadata(path)
        .map(|metadata| match metadata.modified() {
            Ok(time) => time
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|x| x.as_secs())
                .unwrap_or(0),
            _ => 0,
        })
        .unwrap_or(0)
}
