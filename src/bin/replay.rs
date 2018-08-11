extern crate docopt;
extern crate lodepng;
extern crate rustc_serialize;
extern crate threadpool;
extern crate voxelmap_cache;
extern crate zip;

use docopt::Docopt;
use voxelmap_cache::replay::{read_replay, McPacket};

const USAGE: &'static str = "
Usage: replay [-q] [--filter=<ids>] <path>

path points to a (date).mcpr file like found in .minecraft/replay_recordings/

Options:
    -q, --quiet     Do not output info messages.
    --filter=<ids>  Only print packets with the given comma-separated ids.
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_filter: Option<String>,
    arg_path: String,
    flag_quiet: bool,
}

impl Args {
    fn get_id_filter(&self) -> Result<Option<Vec<u8>>, String> {
        if let Some(ref s) = self.flag_filter {
            Ok(Some(
                s.as_str()
                    .split(",")
                    .map(str::parse)
                    .collect::<Result<Vec<u8>, _>>()
                    .map_err(|e| e.to_string())?,
            ))
        } else {
            Ok(None)
        }
    }
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    let verbose = !args.flag_quiet;
    let filter_ids = args.get_id_filter().expect("Malformed filter argument");

    if verbose {
        eprint!("Reading replay {} ... ", &args.arg_path);
    }
    let replay = read_replay(&args.arg_path).expect("Reading replay file");
    if verbose {
        let info = &replay.info;
        eprintln!(
            "from {} for {}ms with {} on {}",
            info.date,
            info.duration,
            info.mc_version,
            info.server_name,
        );
    }

    for mut packet in replay {
        if let Some(ref ids) = filter_ids {
            if ids.iter().all(|id| *id != packet.id) {
                continue;
            }
        }
        if let Err(_e) = packet.parse_packet() {
            println!("{} {} failed to parse", &packet.date, &packet.id);
            std::process::exit(1);
        }
        match packet.get_packet() {
            Some(McPacket::Chat { message, position }) => {
                println!("{} chat {}: {}", &packet.date, position, message)
            }
            Some(McPacket::ChunkData { x, z, is_new, .. }) => {
                println!("{} chunk {},{} {}", &packet.date, x, z, is_new)
            }
            _ => println!("{} {}", &packet.date, &packet.id),
        }
    }
}
