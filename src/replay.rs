extern crate serde;
extern crate serde_json;
extern crate zip;

use std::io::Read;
use std::path::Path;

use super::buf_rw::{BufErr, BufReader, UUID};

#[derive(Debug)]
pub enum McPacket {
    SpawnPlayer {
        eid: i32,
        uuid: UUID,
        x: f64,
        y: f64,
        z: f64,
        yaw: u8,
        pitch: u8,
        metadata: Vec<u8>,
    },
    Chat {
        message: String,
        position: i8,
    },
    ChunkData {
        x: i32,
        z: i32,
        is_new: bool,
        bitmask: u16,
        chunk_data: Vec<u8>,
        tile_entities: Vec<u8>,
    },
    EntityRelativeMove {
        eid: i32,
        dx: f64,
        dy: f64,
        dz: f64,
        on_ground: bool,
    },
    EntityLookAndRelativeMove {
        eid: i32,
        dx: f64,
        dy: f64,
        dz: f64,
        yaw: u8,
        pitch: u8,
        on_ground: bool,
    },
    EntityTeleport {
        eid: i32,
        x: f64,
        y: f64,
        z: f64,
        yaw: u8,
        pitch: u8,
        on_ground: bool,
    },
    Unimplemented,
}

impl McPacket {
    fn decode(data: &Vec<u8>) -> Result<McPacket, BufErr> {
        let mut data = BufReader::new(data.clone()); // TODO operate on data ref directly, no clone
        match data.read_u8()? {
            0x05 => {
                Ok(McPacket::SpawnPlayer {
                    eid: data.read_varint()?,
                    uuid: data.read_uuid()?,
                    x: data.read_f64()?,
                    y: data.read_f64()?,
                    z: data.read_f64()?,
                    yaw: data.read_u8()?,
                    pitch: data.read_u8()?,
                    metadata: data.read_remainder()?, // TODO custom format
                })
            }

            0x0f => {
                let message_bytes = data.read_bytes_len_varint()?;
                let message = ::std::str::from_utf8(&message_bytes)?.to_owned();
                let position = data.read_i8()?;

                Ok(McPacket::Chat { message, position })
            }

            0x20 => {
                Ok(McPacket::ChunkData {
                    x: data.read_i32()?,
                    z: data.read_i32()?,
                    is_new: data.read_bool()?,
                    bitmask: data.read_u16()?,
                    chunk_data: data.read_bytes_len_varint()?,
                    tile_entities: data.read_remainder()?, // TODO read this many NBTs
                })
            }

            0x26 => {
                Ok(McPacket::EntityRelativeMove {
                    eid: data.read_varint()?,
                    dx: (data.read_i16()? as f64) / 4096_f64,
                    dy: (data.read_i16()? as f64) / 4096_f64,
                    dz: (data.read_i16()? as f64) / 4096_f64,
                    on_ground: data.read_bool()?,
                })
            }

            0x27 => {
                Ok(McPacket::EntityLookAndRelativeMove {
                    eid: data.read_varint()?,
                    dx: (data.read_i16()? as f64) / 4096_f64,
                    dy: (data.read_i16()? as f64) / 4096_f64,
                    dz: (data.read_i16()? as f64) / 4096_f64,
                    yaw: data.read_u8()?,
                    pitch: data.read_u8()?,
                    on_ground: data.read_bool()?,
                })
            }

            0x4C => {
                Ok(McPacket::EntityTeleport {
                    eid: data.read_varint()?,
                    x: data.read_f64()?,
                    y: data.read_f64()?,
                    z: data.read_f64()?,
                    yaw: data.read_u8()?,
                    pitch: data.read_u8()?,
                    on_ground: data.read_bool()?,
                })
            }

            // TODO other packets
            _ => Ok(McPacket::Unimplemented),
        }
    }
}

#[derive(Debug)]
pub struct ReplayPacket {
    pub date: usize,
    pub size: usize,
    pub id: u8,
    pub data: Vec<u8>,
    decoded: Option<McPacket>,
}

impl ReplayPacket {
    pub fn get_packet(&self) -> &Option<McPacket> {
        &self.decoded
    }
    pub fn parse_packet(&mut self) -> Result<(), BufErr> {
        if let None = self.decoded {
            self.decoded = Some(McPacket::decode(&self.data)?);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ReplayInfo {
    pub date: usize,          // milliseconds since UNIX epoch
    pub duration: usize,      // duration of the recording in milliseconds
    pub mc_version: String,   // for example "1.12.2"
    pub players: Vec<String>, // UUIDs of all seen players
    pub server_name: String,
}

pub struct Replay {
    pub info: ReplayInfo,
    data: BufReader,
}

impl Iterator for Replay {
    type Item = ReplayPacket;

    fn next(&mut self) -> Option<ReplayPacket> {
        if self.data.position() as usize + 9 > self.data.len() {
            return None;
        }

        let time_offset = self.data.read_u32().expect("malformed tmcpr") as usize;
        let size = self.data.read_u32().expect("malformed tmcpr") as usize;

        let packet_data = self.data.read_bytes(size).expect("malformed tmcpr");

        let id = packet_data[0];
        let date = self.info.date + time_offset;

        Some(ReplayPacket {
            date: date,
            size: size,
            id: id,
            data: packet_data,
            decoded: None,
        })
    }
}

pub fn read_replay<P>(path: &P) -> Result<Replay, String>
where
    P: AsRef<Path>,
{
    use std::fs;

    let zip_file = fs::File::open(&path).map_err(|e| e.to_string())?;
    let mut zip_archive = zip::ZipArchive::new(zip_file).map_err(|e| e.to_string())?;

    let info = {
        let mut info_file = zip_archive
            .by_name("metaData.json")
            .map_err(|_e| "No info file in tile zip")?;
        let json: serde_json::Value =
            serde_json::from_reader(info_file).map_err(|_e| "Malformed JSON in replay info")?;
        let o: &serde_json::Map<String, serde_json::Value> =
            json.as_object().ok_or("No object in replay info")?;

        ReplayInfo {
            date: o
                .get("date")
                .ok_or("No date in replay info")?
                .as_i64()
                .ok_or("Malformed date in replay info")? as usize,
            duration: o
                .get("duration")
                .ok_or("No duration in replay info")?
                .as_i64()
                .ok_or("Malformed duration in replay info")? as usize,
            mc_version: String::from(
                o.get("mcversion")
                    .ok_or("No mcversion in replay info")?
                    .as_str()
                    .ok_or("Malformed mcversion in replay info")?,
            ),
            server_name: String::from(
                o.get("serverName")
                    .ok_or("No serverName in replay info")?
                    .as_str()
                    .ok_or("Malformed serverName in replay info")?,
            ),
            players: o
                .get("players")
                .ok_or("No players in replay info")?
                .as_array()
                .ok_or("Malformed players in replay info")?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(String::from)
                        .ok_or("Malformed serverName in replay info")
                })
                .collect::<Result<Vec<String>, &str>>()?,
        }
    };

    let data = {
        let mut data_file = zip_archive
            .by_name("recording.tmcpr")
            .map_err(|_e| "No data file in tile zip")?;
        let mut data = vec![0; data_file.size() as usize];
        data_file.read_exact(&mut *data).map_err(|e| e.to_string())?;
        data
    };

    Ok(Replay {
        info: info,
        data: BufReader::new(data),
    })
}
