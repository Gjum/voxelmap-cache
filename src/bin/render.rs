extern crate docopt;
extern crate lodepng;
extern crate serde;
extern crate threadpool;
extern crate voxelmap_cache;
extern crate zip;

use docopt::Docopt;
use serde::Deserialize;
use std::iter::FromIterator;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::{fs, time::Duration};
use threadpool::ThreadPool;
use voxelmap_cache::colorizer::Colorizer;
use voxelmap_cache::mc::blocks::BLOCK_STRINGS_ARR;
use voxelmap_cache::tile::{
    get_tile_paths_in_dirs, get_xz_from_tile_path, is_tile_pos_in_bounds, read_tile, KeysMap,
    NamesVec,
};
use voxelmap_cache::{parse_bounds, ProgressTracker, TILE_COLUMNS, TILE_HEIGHT, TILE_WIDTH};

const USAGE: &'static str = "
Usage: render [-q] [-t threads] [--between=<bounds>] <cache-path> <output-path> (simple | light | biome | height | height-bw | naturality | terrain)

cache-path contains voxelmap caches in the format `<x>,<z>.zip`

output-path is a directory that will contain the rendered tiles

Options:
    -q, --quiet         Do not output info messages.
    -t, --threads       Number of threads to use for parallel processing
    --between=<bounds>  Only render tiles at least partially within this bounding box,
                        format: w,n,e,s [default: -99999,-99999,99999,99999]
";

// TODO allow output to be a .png (single output image)

#[derive(Debug, Deserialize)]
struct Args {
    flag_between: String,
    arg_output_path: String,
    arg_cache_path: String,
    flag_quiet: bool,
    cmd_simple: bool,
    cmd_light: bool,
    cmd_biome: bool,
    cmd_height: bool,
    cmd_height_bw: bool,
    cmd_naturality: bool,
    cmd_terrain: bool,
    arg_threads: Option<usize>,
}

impl Args {
    fn get_colorizer(&self) -> Colorizer {
        if self.cmd_simple {
            Colorizer::Simple
        } else if self.cmd_light {
            Colorizer::Light
        } else if self.cmd_biome {
            Colorizer::Biome
        } else if self.cmd_height {
            Colorizer::Height
        } else if self.cmd_height_bw {
            Colorizer::HeightBW
        } else if self.cmd_naturality {
            Colorizer::Naturality
        } else {
            panic!("Unknown colorizer selected")
        }
    }
}

#[derive(Debug)]
struct RenderConfig {
    colorizer: Colorizer,
    global_keys: KeysMap,
    global_names: NamesVec,
}

#[derive(Debug)]
struct OutputConfig<'a> {
    output_path: &'a String,
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

    let bounds = parse_bounds(&args.flag_between).unwrap_or_else(|e| {
        eprintln!("Invalid arg: --between={} {}", &args.flag_between, e);
        std::process::exit(1);
    });

    let tile_paths: Vec<PathBuf> = tile_paths
        .into_iter()
        .filter(|path| is_tile_pos_in_bounds(get_xz_from_tile_path(path).unwrap(), &bounds))
        .collect();

    fs::create_dir_all(&args.arg_output_path).unwrap_or_else(|e| {
        eprintln!(
            "Failed to create output directory {:?} {:?}",
            &args.arg_output_path, e
        );
        std::process::exit(1);
    });

    let total_work = tile_paths.len();
    let mut progress = ProgressTracker::new(total_work);
    if verbose {
        eprintln!(
            "Rendering {:?} tiles to {:?}",
            total_work, &args.arg_output_path
        )
    }

    let render_config = Arc::new(RenderConfig {
        colorizer: args.get_colorizer(),
        global_keys: build_global_keys_map(),
        global_names: BLOCK_STRINGS_ARR.iter().map(|x| x.to_string()).collect(),
    });

    let pool = ThreadPool::new(args.arg_threads.unwrap_or(4));
    let (tx, rx) = channel();

    for tile_path in tile_paths.into_iter() {
        let tx = tx.clone();
        let render_config = render_config.clone();
        pool.execute(move || {
            let result = render_tile(&tile_path, &render_config);
            tx.send((tile_path, result)).expect("Sending result");
        });
    }

    let output_config = OutputConfig {
        output_path: &args.arg_output_path,
    };

    for work_done in 0..total_work {
        let result = rx.recv().expect("Receiving next result");

        process_result(result, &output_config);

        progress.progress_to(work_done);
        if verbose {
            progress.print_progress();
        }
    }

    if verbose {
        let time_total = progress.elapsed();
        let total_min = time_total.as_secs() / 60;
        let total_sec = time_total.as_secs() % 60;
        let time_per_work_item = if total_work == 0 {
            Duration::from_millis(0)
        } else {
            time_total / total_work as u32
        };
        let tile_ms = time_per_work_item.as_secs() * 1_000
            + time_per_work_item.subsec_nanos() as u64 / 1_000_000;
        eprintln!(
            "Done rendering. Took {}:{:02} for all {} tiles, {}ms per tile",
            total_min, total_sec, total_work, tile_ms,
        );
    };
}

fn process_result(
    result_with_path: (PathBuf, Result<Vec<u32>, String>),
    config: &OutputConfig,
) -> () {
    let (tile_path, result) = result_with_path;
    if let Err(msg) = result {
        eprintln!("Failed rendering tile {:?} {}", tile_path, msg);
        return;
    }

    let (x, z) = get_xz_from_tile_path(&tile_path).expect("Getting tile position");
    let img_path = format!("{}/{:?},{:?}.png", config.output_path, x, z);

    fs::create_dir_all(config.output_path).expect(&format!(
        "Creating containing directory for tile {}",
        img_path
    ));

    let pixbuf = result.expect("error already handled");
    lodepng::encode32_file(&img_path, &pixbuf[..], TILE_WIDTH, TILE_HEIGHT)
        .expect(&format!("Encoding tile {}", img_path));
}

fn render_tile(tile_path: &PathBuf, config: &RenderConfig) -> Result<Vec<u32>, String> {
    let tile = read_tile(tile_path).map_err(|e| e.to_string())?;
    let mut pixbuf = vec![0_u32; TILE_COLUMNS];

    let get_column_color = config.colorizer.get_column_color_fn();
    for i in 0..TILE_COLUMNS {
        pixbuf[i] = get_column_color(&tile, i);
    }

    Ok(pixbuf)
}

fn build_global_keys_map() -> KeysMap {
    KeysMap::from_iter(
        BLOCK_STRINGS_ARR
            .iter()
            .enumerate()
            .rev()
            .map(|(i, s)| (s.to_string(), i as u16)),
    )
}
