extern crate zip;

use std::collections::{HashMap, LinkedList};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

pub mod blocks;

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
pub type TileDataBytes = Vec<u8>;
pub type KeysMap = HashMap<String, u16>;

pub struct Tile {
    pub source: Option<PathBuf>,
    pub pos: Option<TilePos>,
    pub columns: TileDataBytes,
    pub keys: Option<KeysMap>,
}

const AIR_STR: &str = "minecraft:air";

pub fn get_chunk_start(chunk_nr: usize) -> usize {
    let chunk_start_col = (chunk_nr * CHUNK_WIDTH) % TILE_WIDTH
        + (chunk_nr * CHUNK_WIDTH / TILE_WIDTH) * TILE_WIDTH * CHUNK_HEIGHT;
    chunk_start_col * COLUMN_BYTES
}

impl Tile {
    pub fn is_chunk_unset(&self, chunk_nr: usize) -> bool {
        let chunk_start = get_chunk_start(chunk_nr);
        let height = self.columns[chunk_start];
        let block_nr =
            (self.columns[chunk_start + 1] as u16) << 8 | (self.columns[chunk_start + 2] as u16);
        let is_air = block_nr == 0 || match self.keys {
            Some(ref keys) => keys.get(AIR_STR).map_or(true, |air_nr| *air_nr == block_nr),
            None => false,
        };
        return height == 0 && is_air;
    }
}

impl std::fmt::Debug for Tile {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt.write_str(&format!(
            "Tile with {} keys{}{}",
            self.keys.as_ref().map_or(0, |k| k.len()),
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

    let keys = zip_archive.by_name("key").ok().map(|key_file| {
        let mut keys = Box::new(HashMap::new());
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
            keys.insert(block_name, block_nr);
        }
        *keys
    });

    let mut columns = vec![0; TILE_COLUMNS * COLUMN_BYTES];
    {
        let mut data_file = zip_archive
            .by_name("data")
            .map_err(|_e| "No data file in tile zip")?;
        data_file
            .read_exact(&mut *columns)
            .map_err(|e| e.to_string())?;
    }

    let tile = Box::new(Tile {
        source: Some(tile_path.clone()),
        pos: get_xz_from_tile_path(tile_path).ok(),
        columns: columns,
        keys: keys,
    });

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

    if let Some(keys) = &tile.keys {
        for (name, nr) in keys {
            zip_archive
                .write_fmt(format_args!("{} {}\r\n", nr, name))
                .map_err(|e| e.to_string())?;
        }
    }

    // Optionally finish the zip. (this is also done on drop)
    zip_archive.finish().map_err(|e| e.to_string())?;

    Ok(())
}

pub fn get_xz_from_tile_path(tile_path: &PathBuf) -> Result<TilePos, String> {
    let fname = tile_path.file_name().unwrap().to_str().unwrap();
    if fname.len() <= 4 {
        return Err("file name too short".to_owned());
    }
    let (coords_part, _) = fname.split_at(fname.len() - 4);
    let mut it = coords_part.splitn(3, ',');
    let x = it
        .next()
        .ok_or("no x coord in filename".to_owned())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let z = it
        .next()
        .ok_or("no z coord in filename".to_owned())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    Ok((x, z))
}

pub fn get_contrib_from_tile_path(tile_path: &PathBuf) -> Result<String, String> {
    let fname = tile_path.file_name().unwrap().to_str().unwrap();
    if fname.len() <= 4 {
        return Err("no contrib in filename".to_owned());
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
                Ok(_pos) => {
                    if tile_path.to_string_lossy().ends_with(".zip") {
                        tile_paths.push_back(tile_path)
                    } else {
                        println!("Ignoring non-tile file {:?}", &tile_path);
                    }
                }
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

pub fn get_mtime_or_0(path: &PathBuf) -> u64 {
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
