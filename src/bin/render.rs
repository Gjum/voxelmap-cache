#![feature(slice_patterns)]

extern crate docopt;
extern crate rustc_serialize;
extern crate voxelmap_cache;

use docopt::Docopt;
use std::time::Instant;
use voxelmap_cache::render_parallelized;
use voxelmap_cache::*;
use voxelmap_cache::colorizer::*;
use voxelmap_cache::processor::*;

const USAGE: &'static str = "
Usage: rustmap [-q] [-t threads] <cache> <output> (simple | light | biome | height | terrain)

Options:
    -q, --quiet  Do not output info messages.
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_quiet: bool,
    arg_cache: String,
    arg_output: String,
    cmd_simple: bool,
    cmd_light: bool,
    cmd_biome: bool,
    cmd_height: bool,
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
        } else if self.cmd_terrain {
            Colorizer::Terrain
        } else {
            panic!("Unknown colorizer selected")
        }
    }

    fn get_processor(&self) -> Box<Processor> {
        if self.arg_output.contains("{x}") && self.arg_output.contains("{z}")
            || self.arg_output.contains("{tile}") {
            Box::new(TilesProcessor { tiles_pattern: self.arg_output.clone() })
        } else {
            Box::new(SingleImageProcessor::new(&self.arg_output))
        }
    }
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    let verbose = !args.flag_quiet;

    if verbose { println!("Finding regions from {} ...", args.arg_cache); }

    let regions = get_regions(args.arg_cache.as_ref());
    let colorizer = args.get_colorizer();

    if verbose {
        println!("Rendering {} regions to {} with {:?}",
                 regions.len(), args.arg_output, colorizer);
    }

    let processor = args.get_processor();

    let start_time = Instant::now();
    render_parallelized(
        processor,
        colorizer,
        regions,
        args.arg_threads.unwrap_or(4),
        verbose,
    );

    if verbose {
        let time_total = start_time.elapsed().as_secs();
        println!("Done after {}:{:02}", time_total / 60, time_total % 60);
    }
}
