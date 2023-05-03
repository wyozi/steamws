use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{self, Error};
use std::io::{ErrorKind, Read};

pub mod lump_indices;

const BSP_HEADER_LEN: u32 = 1036;
const BSP_LUMP_COUNT: usize = 64;

#[derive(Default)]
struct Lump {
    off: u32,
    len: u32,
}

struct BSPHeader {
    version: u32,
    lumps: [Lump; BSP_LUMP_COUNT],
    map_revision: u32,
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
            let _vers = reader.read_u32::<LittleEndian>()?;
            let _lump_ident = reader.read_u32::<LittleEndian>()?;

            lumps[i] = Lump { off: off, len: len };
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

    pub fn lump_size(&self, lump_index: lump_indices::LumpIndex) -> u32 {
        self.header.lumps[lump_index as usize].len
    }

    pub fn reader_for_lump(
        mut self,
        lump_index: lump_indices::LumpIndex,
    ) -> Result<R, Box<dyn std::error::Error>> {
        let off = self.header.lumps[lump_index as usize].off;

        // skip over off bytes
        io::copy(
            &mut self.reader.by_ref().take((off - BSP_HEADER_LEN).into()),
            &mut io::sink(),
        )?;

        Ok(self.reader)
    }
}
