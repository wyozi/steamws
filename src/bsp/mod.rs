use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{self, Error};
use std::io::{ErrorKind, Read};

use self::buffered_bsp::BufferedBSP;
use self::lump_indices::LumpIndex;

mod buffered_bsp;
mod counting_read;
pub mod lump_indices;

pub const BSP_HEADER_LEN: u32 = 1036;
const BSP_LUMP_COUNT: usize = 64;

#[derive(Default)]
pub struct Lump {
    pub off: u32,
    pub len: u32,
    pub version: u32,
    pub ident: u32,
}

pub struct BSPHeader {
    pub version: u32,
    pub lumps: [Lump; BSP_LUMP_COUNT],
    pub map_revision: u32,
}

pub struct BSPReader<R> {
    reader: R,
    header: BSPHeader,
}

impl<R: Read> BSPReader<R> {
    pub fn from_reader(mut reader: R) -> Result<BSPReader<R>, Box<dyn std::error::Error>> {
        let mut ident_buf = [0; 4];
        reader.read_exact(&mut ident_buf)?;
        if &ident_buf != b"VBSP" {
            eprintln!("ident_buf = {:?}", ident_buf);
            return Err(Box::new(Error::new(
                ErrorKind::Other,
                "header ident is not VBSP",
            )));
        }

        let version = reader.read_u32::<LittleEndian>()?;

        let mut lumps: [Lump; BSP_LUMP_COUNT] =
            unsafe { std::mem::MaybeUninit::uninit().assume_init() };

        for i in 0..BSP_LUMP_COUNT {
            let off = reader.read_u32::<LittleEndian>()?;
            let len = reader.read_u32::<LittleEndian>()?;
            let vers = reader.read_u32::<LittleEndian>()?;
            let ident = reader.read_u32::<LittleEndian>()?;

            lumps[i] = Lump {
                off: off,
                len: len,
                version: vers,
                ident: ident,
            };
        }
        let map_revision = reader.read_u32::<LittleEndian>()?;

        Ok(BSPReader {
            reader: reader,
            header: BSPHeader {
                version: version,
                lumps: lumps,
                map_revision: map_revision,
            },
        })
    }

    pub fn header(&self) -> &BSPHeader {
        &self.header
    }

    pub fn into_buffered_bsp(mut self) -> BufferedBSP {
        let mut buf = Vec::new();
        self.reader.read_to_end(&mut buf).unwrap();
        BufferedBSP {
            header: self.header,
            data_without_header: buf,
        }
    }

    pub fn into_reader_for_lump(
        mut self,
        lump_index: lump_indices::LumpIndex,
    ) -> Result<R, Box<dyn std::error::Error>> {
        let lump = &self.header.lumps[lump_index as usize];

        // skip over off bytes
        io::copy(
            &mut self
                .reader
                .by_ref()
                .take((lump.off - BSP_HEADER_LEN).into()),
            &mut io::sink(),
        )?;

        Ok(self.reader.take(lump.len.into()).into_inner())
    }
}
