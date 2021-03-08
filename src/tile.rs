extern crate zip;

use std::collections::{HashMap, LinkedList};
use std::fmt;
use std::fs;
use std::num::ParseIntError;
use std::path::PathBuf;

pub const COLUMN_BYTES_OLD: usize = 17;
pub const COLUMN_BYTES_MODERN: usize = 18;

pub const TILE_WIDTH: usize = 256;
pub const TILE_HEIGHT: usize = 256;
pub const TILE_COLUMNS: usize = TILE_WIDTH * TILE_HEIGHT;

pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_HEIGHT: usize = 16;
pub const CHUNK_COLUMNS: usize = CHUNK_WIDTH * CHUNK_HEIGHT;

pub const TILE_CHUNKS: usize = TILE_COLUMNS / CHUNK_COLUMNS;

const HEIGHTPOS: usize = 0;
const BLOCKSTATEPOS: usize = 1;
const LIGHTPOS: usize = 3;
const OCEANFLOORHEIGHTPOS: usize = 4;
const OCEANFLOORBLOCKSTATEPOS: usize = 5;
const OCEANFLOORLIGHTPOS: usize = 7;
const TRANSPARENTHEIGHTPOS: usize = 8;
const TRANSPARENTBLOCKSTATEPOS: usize = 9;
const TRANSPARENTLIGHTPOS: usize = 11;
const FOLIAGEHEIGHTPOS: usize = 12;
const FOLIAGEBLOCKSTATEPOS: usize = 13;
const FOLIAGELIGHTPOS: usize = 15;
const BIOMEIDPOS: usize = 16;

pub type TilePos = (i32, i32);
pub type KeysMap = HashMap<String, u16>;
pub type NamesVec = Vec<String>;

pub struct Tile {
    // TODO make private
    /// in v2 format (layer-then-coords)
    pub data: Vec<u8>,
    pub keys: KeysMap,
    pub names: NamesVec,
    pub pos: Option<TilePos>,
    pub source: Option<PathBuf>,
}

impl Tile {
    pub fn is_chunk_empty(&self, chunk_nr: usize) -> bool {
        let column_nr = first_column_nr_of_chunk_nr(chunk_nr);
        self.is_col_empty(column_nr)
    }
    pub fn is_col_empty(&self, column_nr: usize) -> bool {
        self.get_height(column_nr) == 0
            && self.get_biome_id(column_nr) == 0
            && self.get_blockstate(column_nr) == 0
    }

    pub fn get_height(&self, column_nr: usize) -> u8 {
        self.get_u8(column_nr, HEIGHTPOS)
    }
    pub fn get_blockstate(&self, column_nr: usize) -> u16 {
        self.get_u16(column_nr, BLOCKSTATEPOS)
    }
    pub fn get_light(&self, column_nr: usize) -> u8 {
        self.get_u8(column_nr, LIGHTPOS)
    }
    pub fn get_ocean_floor_height(&self, column_nr: usize) -> u8 {
        self.get_u8(column_nr, OCEANFLOORHEIGHTPOS)
    }
    pub fn get_ocean_floor_blockstate(&self, column_nr: usize) -> u16 {
        self.get_u16(column_nr, OCEANFLOORBLOCKSTATEPOS)
    }
    pub fn get_ocean_floor_light(&self, column_nr: usize) -> u8 {
        self.get_u8(column_nr, OCEANFLOORLIGHTPOS)
    }
    pub fn get_transparent_height(&self, column_nr: usize) -> u8 {
        self.get_u8(column_nr, TRANSPARENTHEIGHTPOS)
    }
    pub fn get_transparent_blockstate(&self, column_nr: usize) -> u16 {
        self.get_u16(column_nr, TRANSPARENTBLOCKSTATEPOS)
    }
    pub fn get_transparent_light(&self, column_nr: usize) -> u8 {
        self.get_u8(column_nr, TRANSPARENTLIGHTPOS)
    }
    pub fn get_foliage_height(&self, column_nr: usize) -> u8 {
        self.get_u8(column_nr, FOLIAGEHEIGHTPOS)
    }
    pub fn get_foliage_blockstate(&self, column_nr: usize) -> u16 {
        self.get_u16(column_nr, FOLIAGEBLOCKSTATEPOS)
    }
    pub fn get_foliage_light(&self, column_nr: usize) -> u8 {
        self.get_u8(column_nr, FOLIAGELIGHTPOS)
    }
    pub fn get_biome_id(&self, column_nr: usize) -> u16 {
        self.get_u16(column_nr, BIOMEIDPOS)
    }

    pub fn set_height(&mut self, column_nr: usize, value: u8) {
        self.set_u8(column_nr, HEIGHTPOS, value);
    }
    pub fn set_blockstate(&mut self, column_nr: usize, id: u16) {
        self.set_u16(column_nr, BLOCKSTATEPOS, id);
    }
    pub fn set_light(&mut self, column_nr: usize, value: u8) {
        self.set_u8(column_nr, LIGHTPOS, value);
    }
    pub fn set_ocean_floor_height(&mut self, column_nr: usize, value: u8) {
        self.set_u8(column_nr, OCEANFLOORHEIGHTPOS, value);
    }
    pub fn set_ocean_floor_blockstate(&mut self, column_nr: usize, id: u16) {
        self.set_u16(column_nr, OCEANFLOORBLOCKSTATEPOS, id);
    }
    pub fn set_ocean_floor_light(&mut self, column_nr: usize, value: u8) {
        self.set_u8(column_nr, OCEANFLOORLIGHTPOS, value);
    }
    pub fn set_transparent_height(&mut self, column_nr: usize, value: u8) {
        self.set_u8(column_nr, TRANSPARENTHEIGHTPOS, value);
    }
    pub fn set_transparent_blockstate(&mut self, column_nr: usize, id: u16) {
        self.set_u16(column_nr, TRANSPARENTBLOCKSTATEPOS, id);
    }
    pub fn set_transparent_light(&mut self, column_nr: usize, value: u8) {
        self.set_u8(column_nr, TRANSPARENTLIGHTPOS, value);
    }
    pub fn set_foliage_height(&mut self, column_nr: usize, value: u8) {
        self.set_u8(column_nr, FOLIAGEHEIGHTPOS, value);
    }
    pub fn set_foliage_blockstate(&mut self, column_nr: usize, id: u16) {
        self.set_u16(column_nr, FOLIAGEBLOCKSTATEPOS, id);
    }
    pub fn set_foliage_light(&mut self, column_nr: usize, value: u8) {
        self.set_u8(column_nr, FOLIAGELIGHTPOS, value);
    }
    pub fn set_biome_id(&mut self, column_nr: usize, value: u16) {
        self.set_u16(column_nr, BIOMEIDPOS, value);
    }

    fn get_u8(&self, column_nr: usize, layer_nr: usize) -> u8 {
        let index = column_nr + TILE_COLUMNS * layer_nr;
        self.data[index]
    }
    fn get_u16(&self, column_nr: usize, layer_nr: usize) -> u16 {
        let index = column_nr + TILE_COLUMNS * layer_nr;
        (self.data[index] as u16) << 8 | (self.data[index + TILE_COLUMNS] as u16)
    }

    fn set_u8(&mut self, column_nr: usize, layer_offset: usize, value: u8) {
        let index = column_nr + TILE_COLUMNS * layer_offset;
        self.data[index] = value;
    }
    fn set_u16(&mut self, column_nr: usize, layer_offset: usize, value: u16) {
        let index = column_nr + TILE_COLUMNS * layer_offset;
        self.data[index] = (value >> 8) as u8;
        self.data[index + TILE_COLUMNS] = value as u8;
    }
}

impl fmt::Debug for Tile {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            fmt,
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
        )
    }
}

pub fn column_nr_of_pos(x: usize, z: usize) -> usize {
    x + z * TILE_WIDTH
}

pub fn first_column_nr_of_chunk_nr(chunk_nr: usize) -> usize {
    (chunk_nr * CHUNK_WIDTH) % TILE_WIDTH
        + (chunk_nr * CHUNK_WIDTH / TILE_WIDTH) * TILE_WIDTH * CHUNK_HEIGHT
}

pub fn read_tile(tile_path: &PathBuf) -> Result<Box<Tile>, String> {
    use std::io::{BufRead, BufReader, Read};

    let zip_file = fs::File::open(&tile_path).map_err(|e| e.to_string())?;
    let mut zip_archive = zip::ZipArchive::new(zip_file).map_err(|e| e.to_string())?;

    let mut max_key = 0;
    let keys = zip_archive
        .by_name("key")
        .ok()
        .map(|key_file| {
            let mut keys = Box::new(HashMap::new());
            for line in BufReader::new(key_file).lines() {
                let line = line.unwrap();
                if line.is_empty() {
                    continue;
                }
                let mut split = line.split(" ");
                let block_id = split
                    .next()
                    .expect("getting block num from key line split")
                    .parse::<u16>()
                    .expect("converting block num to int");
                let block_name = split
                    .next()
                    .expect("getting block name from key line split")
                    .to_string();
                if max_key < block_id {
                    max_key = block_id;
                }
                keys.insert(block_name, block_id);
            }
            *keys
        })
        .expect("XXX support old keyless format");

    let unknown_name = "?".to_string();
    let mut names: NamesVec = vec![unknown_name; 1 + max_key as usize];
    for (name, nr) in keys.iter() {
        names[*nr as usize] = name.clone();
    }

    let mut data = vec![0; TILE_COLUMNS * COLUMN_BYTES_MODERN];
    {
        let mut data_file = zip_archive
            .by_name("data")
            .map_err(|_e| "No data file in tile zip")?;
        data_file
            .read_exact(&mut *data)
            .map_err(|e| e.to_string())?;
    }

    let tile = Box::new(Tile {
        source: Some(tile_path.clone()),
        pos: get_xz_from_tile_path(tile_path).ok(),
        data: data,
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

    // we can only write version 2

    zip_archive
        .start_file("control", options)
        .map_err(|e| e.to_string())?;
    zip_archive
        .write_all("version:2\r\n".as_bytes())
        .map_err(|e| e.to_string())?;

    zip_archive
        .start_file("data", options)
        .map_err(|e| e.to_string())?;
    zip_archive
        .write_all(&tile.data)
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
        .ok_or("No contrib in tile name")?
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
