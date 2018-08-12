use buf_rw::{BufErr, BufReader, UUID};

// TODO relace enum with structs implementing a de/encode trait

#[derive(Debug)]
pub struct ChunkData {
    pub x: i32,
    pub z: i32,
    pub is_new: bool,
    pub sections_mask: u16,
    pub chunk_data: Vec<u8>,
    pub tile_entities: Vec<u8>,
}

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
    ChunkDataHack(ChunkData),
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
    pub fn decode(data: &Vec<u8>) -> Result<McPacket, BufErr> {
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
                let message_bytes = data.read_varint_prefixed_bytes()?;
                let message = ::std::str::from_utf8(&message_bytes)?.to_owned();
                let position = data.read_i8()?;

                Ok(McPacket::Chat { message, position })
            }

            0x20 => {
                Ok(McPacket::ChunkDataHack(ChunkData {
                    x: data.read_i32()?,
                    z: data.read_i32()?,
                    is_new: data.read_bool()?,
                    sections_mask: data.read_u16()?,
                    chunk_data: data.read_varint_prefixed_bytes()?,
                    tile_entities: data.read_remainder()?, // TODO read this many NBTs
                }))
            }

            0x26 => Ok(McPacket::EntityRelativeMove {
                eid: data.read_varint()?,
                dx: (data.read_i16()? as f64) / 4096_f64,
                dy: (data.read_i16()? as f64) / 4096_f64,
                dz: (data.read_i16()? as f64) / 4096_f64,
                on_ground: data.read_bool()?,
            }),

            0x27 => Ok(McPacket::EntityLookAndRelativeMove {
                eid: data.read_varint()?,
                dx: (data.read_i16()? as f64) / 4096_f64,
                dy: (data.read_i16()? as f64) / 4096_f64,
                dz: (data.read_i16()? as f64) / 4096_f64,
                yaw: data.read_u8()?,
                pitch: data.read_u8()?,
                on_ground: data.read_bool()?,
            }),

            0x4C => Ok(McPacket::EntityTeleport {
                eid: data.read_varint()?,
                x: data.read_f64()?,
                y: data.read_f64()?,
                z: data.read_f64()?,
                yaw: data.read_u8()?,
                pitch: data.read_u8()?,
                on_ground: data.read_bool()?,
            }),

            // TODO other packets
            _ => Ok(McPacket::Unimplemented),
        }
    }
}
