extern crate docopt;
extern crate lodepng;
extern crate rustc_serialize;
extern crate threadpool;
extern crate voxelmap_cache;
extern crate zip;

use docopt::Docopt;
use voxelmap_cache::buf_rw::UUID;
use voxelmap_cache::mc::packet::{ChunkData, McPacket};
use voxelmap_cache::replay::read_replay;

const USAGE: &'static str = "
Usage: replay [-q] [--filter=<ids>] [--follow=<uuid>] [--server=<address>] <path>

path points to a (date).mcpr file like found in .minecraft/replay_recordings/

Options:
    -q, --quiet         Do not output info messages.
    --filter=<ids>      Only print packets with the given comma-separated ids.
    --follow=<uuid>     Track player position for this uuid (usually the recording player).
    --server=<address>  Ignore replays on other servers.
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_filter: Option<String>,
    flag_follow: Option<String>,
    flag_server: Option<String>,
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

#[derive(Debug)]
struct Player {
    uuid: UUID,
    x: f64,
    y: f64,
    z: f64,
}

impl Player {
    fn new(uuid: UUID) -> Self {
        Self {
            uuid,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
    fn set_pos(&mut self, x: f64, y: f64, z: f64) {
        self.x = x;
        self.y = y;
        self.z = z;
    }
    fn move_by(&mut self, dx: f64, dy: f64, dz: f64) {
        self.x += dx;
        self.y += dy;
        self.z += dz;
    }
    fn on_packet(&mut self, packet: &McPacket) {
        match packet {
            McPacket::SpawnPlayer { x, y, z, .. } => self.set_pos(*x, *y, *z),
            McPacket::EntityTeleport { x, y, z, .. } => self.set_pos(*x, *y, *z),
            McPacket::EntityLookAndRelativeMove { dx, dy, dz, .. } => self.move_by(*dx, *dy, *dz),
            McPacket::EntityRelativeMove { dx, dy, dz, .. } => self.move_by(*dx, *dy, *dz),
            _ => (),
        }
    }
}

fn is_player_id(id: u8) -> bool {
    id == 0x05 || id == 0x26 || id == 0x27 || id == 0x4C
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    let verbose = !args.flag_quiet;
    let filter_ids = args.get_id_filter().expect("Malformed filter argument");

    let mut followed = args.flag_follow.as_ref().map(|ref uuid_str| {
        match UUID::from_str(uuid_str) {
            Ok(uuid) => Player::new(uuid),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    });

    if let Some(ref player) = followed {
        if verbose {
            eprintln!(
                "Following player {} {:?}",
                &args.flag_follow.unwrap(),
                player.uuid
            );
        }
    }

    if verbose {
        eprint!("Reading replay {} ... ", &args.arg_path);
    }
    let replay = read_replay(&args.arg_path).expect("Reading replay file");
    if verbose {
        let info = &replay.info;
        eprintln!(
            "from {} for {}ms with {} on {}",
            info.date, info.duration, info.mc_version, info.server_name,
        );
    }

    if let Some(ref address) = args.flag_server {
        if address != &replay.info.server_name {
            if verbose {
                eprintln!(
                    "Skipping replay on wrong server {}",
                    &replay.info.server_name
                );
            }
            return;
        }
    }

    for mut packet in replay {
        if followed.is_some() && is_player_id(packet.id) {
            if let Err(_e) = packet.parse_packet() {
                eprintln!("{} {} failed to parse", &packet.date, &packet.id);
                std::process::exit(1);
            }

            if let Some(ref mc_packet) = packet.get_packet() {
                if let Some(player) = followed.as_mut() {
                    player.on_packet(mc_packet);
                }
            }
        }

        if let Some(ref ids) = filter_ids {
            if ids.iter().all(|id| *id != packet.id) {
                continue;
            }
        }

        if let Err(_e) = packet.parse_packet() {
            eprintln!("{} {} failed to parse", &packet.date, &packet.id);
            std::process::exit(1);
        }

        match packet.get_packet() {
            Some(McPacket::Chat { message, position }) => {
                println!("{} chat {}: {}", &packet.date, position, message)
            }
            Some(McPacket::ChunkDataHack(ChunkData { x, z, is_new, .. })) => {
                println!("{} chunk {},{} {}", &packet.date, x, z, is_new)
            }

            Some(McPacket::SpawnPlayer {
                eid, uuid, x, y, z, ..
            }) => println!(
                "{} player {},{},{} {} {:?}",
                &packet.date, x, y, z, eid, uuid
            ),

            _ => println!("{} {}", &packet.date, &packet.id),
        }
    }
}
