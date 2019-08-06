extern crate zip;

use super::{CHUNK_HEIGHT, CHUNK_WIDTH, TILE_COLUMNS, TILE_HEIGHT, TILE_WIDTH};
use mc::blocks::AIR_STR;
use std::collections::{HashMap, LinkedList};
use std::fmt;
use std::fs;
use std::num::ParseIntError;
use std::path::PathBuf;

pub const COLUMN_BYTES: usize = 17;

pub type TilePos = (i32, i32);
pub type TileDataBytes = Vec<u8>;
pub type KeysMap = HashMap<String, u16>;
pub type NamesVec = Vec<String>;

pub struct Tile {
    pub source: Option<PathBuf>,
    pub pos: Option<TilePos>,
    pub columns: TileDataBytes,
    pub keys: Option<KeysMap>,
    pub names: Option<NamesVec>,
}

pub fn is_empty(column: &[u8], keys: Option<&KeysMap>) -> bool {
    let height = column[0];
    let block_nr = (column[1] as u16) << 8 | (column[2] as u16);
    let is_air = block_nr == 0 || match keys {
        Some(ref keys) => keys.get(AIR_STR).map_or(true, |air_nr| *air_nr == block_nr),
        None => false,
    };
    return height == 0 && is_air;
}

impl Tile {
    pub fn is_unset(&self, column_start: usize) -> bool {
        let column = &self.columns[column_start..column_start + COLUMN_BYTES];
        return is_empty(column, self.keys.as_ref());
    }
}

impl fmt::Debug for Tile {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            fmt,
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
        )
    }
}

pub fn get_chunk_start(chunk_nr: usize) -> usize {
    let chunk_start_col = (chunk_nr * CHUNK_WIDTH) % TILE_WIDTH
        + (chunk_nr * CHUNK_WIDTH / TILE_WIDTH) * TILE_WIDTH * CHUNK_HEIGHT;
    chunk_start_col * COLUMN_BYTES
}

pub fn read_tile(tile_path: &PathBuf) -> Result<Box<Tile>, String> {
    use std::io::{BufRead, BufReader, Read};

    let zip_file = fs::File::open(&tile_path).map_err(|e| e.to_string())?;
    let mut zip_archive = zip::ZipArchive::new(zip_file).map_err(|e| e.to_string())?;

    let mut max_key = 0;
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
            if max_key < block_nr {
                max_key = block_nr;
            }
            keys.insert(block_name, block_nr);
        }
        *keys
    });
    let names = keys.as_ref().map(|keys| {
        let unknown_name = "?".to_string();
        let mut names: NamesVec = vec![unknown_name; 1 + max_key as usize];
        for (name, nr) in keys.iter() {
            names[*nr as usize] = name.clone();
        }
        names
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
        names: names,
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
        .map_err(|e: ParseIntError| e.to_string())?;
    let z = it
        .next()
        .ok_or("no z coord in filename".to_owned())?
        .parse()
        .map_err(|e: ParseIntError| e.to_string())?;
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
                        eprintln!("Ignoring non-tile file {:?}", &tile_path);
                    }
                }
                Err(e) => {
                    if tile_path.to_string_lossy().ends_with("_chunk-times.gz") {
                        // ignore chunk timestamp info file
                    } else {
                        if verbose {
                            eprintln!("Ignoring non-tile file {:?} {:?}", &tile_path, e);
                        }
                    }
                }
            }
        }
    }
    Ok(tile_paths)
}

pub fn is_tile_pos_in_bounds((tile_x, tile_z): (i32, i32), bounds: &Vec<i32>) -> bool {
    let tw = TILE_WIDTH as i32;
    let th = TILE_HEIGHT as i32;
    let x = tile_x * tw;
    let z = tile_z * th;
    let (w, n, e, s) = (bounds[0], bounds[1], bounds[2], bounds[3]);

    x + tw > w && x < e && z + th > n && z < s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_unset_works_for_global_key() {
        let mut in_tile = Tile {
            source: None,
            pos: None,
            columns: vec![0_u8; TILE_COLUMNS * COLUMN_BYTES],
            keys: None,
            names: None,
        };

        let foo = 17 * (256 * 2 * 16 + 16);
        in_tile.columns[foo + 0] = 2; // height
        in_tile.columns[foo + 1] = 0;
        in_tile.columns[foo + 2] = 42;
        in_tile.columns[foo + 3] = 14; // light
        in_tile.columns[foo + 16] = 23; // biome

        assert_eq!(true, in_tile.is_unset(get_chunk_start(0)));
        assert_eq!(false, in_tile.is_unset(get_chunk_start(33)));
    }

    #[test]
    fn is_unset_works_for_tile_key() {
        let mut in_keys = HashMap::new();
        in_keys.insert("test id 42".to_string(), 42);
        in_keys.insert("minecraft:air".to_string(), 123);
        let mut in_names = vec!["?".to_string(); 124];
        in_names[42] = "test id 42".to_string();
        in_names[123] = "minecraft:air".to_string();

        let mut in_tile = Tile {
            source: None,
            pos: None,
            columns: vec![0_u8; TILE_COLUMNS * COLUMN_BYTES],
            keys: Some(in_keys),
            names: Some(in_names),
        };

        let foo = 17 * (256 * 2 * 16 + 16);
        in_tile.columns[foo + 0] = 2; // height
        in_tile.columns[foo + 1] = 0;
        in_tile.columns[foo + 2] = 42;
        in_tile.columns[foo + 3] = 14; // light
        in_tile.columns[foo + 16] = 23; // biome

        let bar = foo + 32;
        in_tile.columns[bar + 1] = 0;
        in_tile.columns[bar + 2] = 123;

        assert_eq!(true, in_tile.is_unset(get_chunk_start(0))); // all-zeroes
        assert_eq!(false, in_tile.is_unset(get_chunk_start(33))); // foo is set
        assert_eq!(true, in_tile.is_unset(get_chunk_start(35))); // bar has air block
    }
}
