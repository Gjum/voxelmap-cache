extern crate rustc_serialize;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::fmt;
use std::io::{Cursor, Read, Write};

error_chain!{
    types {
        BufErr, BufErrorKind, BufResult;
    }
    foreign_links {
        Io(::std::io::Error);
        Utf8(::std::str::Utf8Error);
        FromHex(rustc_serialize::hex::FromHexError);
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct UUID(u64, u64);

impl UUID {
    pub fn from_str(s: &str) -> Result<UUID, BufErr> {
        use self::rustc_serialize::hex::FromHex;
        let s = match s.len() {
            36 => s.replace("-", ""),
            32 => s.to_owned(),
            _ => return Err("Invalid UUID format")?,
        };
        let parts = s.from_hex()?;
        let mut high = 0u64;
        let mut low = 0u64;
        for i in 0..8 {
            high |= (parts[i] as u64) << (56 - i * 8);
            low |= (parts[i + 8] as u64) << (56 - i * 8);
        }
        Ok(UUID(high, low))
    }
}

impl fmt::Debug for UUID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02x}{:02x}", self.0, self.1)
    }
}

pub struct BufReader {
    len: usize,
    c: Cursor<Vec<u8>>,
}

impl BufReader {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            len: data.len(),
            c: Cursor::new(data),
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn position(&self) -> u64 {
        self.c.position()
    }

    pub fn read_bytes(&mut self, size: usize) -> Result<Vec<u8>, BufErr> {
        let mut v = vec![0; size];
        self.c.read_exact(&mut v)?;
        Ok(v)
    }

    pub fn read_varint_prefixed_bytes(&mut self) -> Result<Vec<u8>, BufErr> {
        let size = self.read_varint()? as usize;
        let mut v = vec![0; size];
        self.c.read_exact(&mut v)?;
        Ok(v)
    }

    pub fn read_remainder(&mut self) -> Result<Vec<u8>, BufErr> {
        let size = self.len() - self.c.position() as usize;
        self.read_bytes(size)
    }

    pub fn read_u8(&mut self) -> Result<u8, BufErr> {
        Ok(self.c.read_u8()?)
    }

    pub fn read_i8(&mut self) -> Result<i8, BufErr> {
        Ok(self.c.read_i8()?)
    }

    pub fn read_bool(&mut self) -> Result<bool, BufErr> {
        Ok(self.read_u8()? != 0)
    }

    pub fn read_u16(&mut self) -> Result<u16, BufErr> {
        Ok(self.c.read_u16::<BigEndian>()?)
    }

    pub fn read_i16(&mut self) -> Result<i16, BufErr> {
        Ok(self.c.read_i16::<BigEndian>()?)
    }

    pub fn read_u32(&mut self) -> Result<u32, BufErr> {
        Ok(self.c.read_u32::<BigEndian>()?)
    }

    pub fn read_i32(&mut self) -> Result<i32, BufErr> {
        Ok(self.c.read_i32::<BigEndian>()?)
    }

    pub fn read_f64(&mut self) -> Result<f64, BufErr> {
        Ok(self.c.read_f64::<BigEndian>()?)
    }

    pub fn read_uuid(&mut self) -> Result<UUID, BufErr> {
        Ok(UUID(
            self.c.read_u64::<BigEndian>()?,
            self.c.read_u64::<BigEndian>()?,
        ))
    }

    pub fn read_varint(&mut self) -> Result<i32, BufErr> {
        const PART: u32 = 0x7F;
        let mut size = 0;
        let mut val = 0u32;
        loop {
            let b = self.c.read_u8()? as u32;
            val |= (b & PART) << (size * 7);
            size += 1;
            if size > 5 {
                return Err("VarInt too big")?;
            }
            if (b & 0x80) == 0 {
                break;
            }
        }
        Ok(val as i32)
    }
}
