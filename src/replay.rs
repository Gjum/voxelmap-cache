extern crate serde;
extern crate serde_json;
extern crate zip;

use super::buf_rw::{BufErr, BufReader};
use super::mc::packet::McPacket;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

error_chain!{
    types {
        ReplayErr, ReplayErrorKind, ReplayResult;
    }
    foreign_links {
        Buf(BufErr);
        Io(::std::io::Error);
        Zip(::zip::result::ZipError);
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
    data: BufReader, // TODO lazy read
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

pub fn read_info<P>(path: &P) -> Result<ReplayInfo, ReplayErr>
where
    P: AsRef<Path>,
{
    let zip_file = File::open(&path)?;
    let mut zip_archive = ZipArchive::new(zip_file)?;
    read_info_from_zip(&mut zip_archive)
}

pub fn read_replay<P>(path: &P) -> Result<Replay, ReplayErr>
where
    P: AsRef<Path>,
{
    let zip_file = File::open(&path)?;
    let mut zip_archive = ZipArchive::new(zip_file)?;

    let info = read_info_from_zip(&mut zip_archive)?;

    let data = {
        let mut data_file = zip_archive
            .by_name("recording.tmcpr")
            .chain_err(|| "No recording in mcpr")?;
        let mut data = vec![0; data_file.size() as usize];
        data_file.read_exact(&mut *data)?;
        data
    };

    Ok(Replay {
        info: info,
        data: BufReader::new(data),
    })
}

fn read_info_from_zip(zip_archive: &mut ZipArchive<File>) -> Result<ReplayInfo, ReplayErr> {
    let info_file = zip_archive
        .by_name("metaData.json")
        .chain_err(|| "No metadata in mcpr")?;
    let json: serde_json::Value =
        serde_json::from_reader(info_file).chain_err(|| "Malformed JSON in replay info")?;
    let o: &serde_json::Map<String, serde_json::Value> =
        json.as_object().ok_or("No object in replay info")?;

    Ok(ReplayInfo {
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
    })
}
