use std::io::Write;

use byteorder::{LittleEndian, WriteBytesExt};

use super::{lump_indices::LumpIndex, BSPHeader, BSP_HEADER_LEN, BSP_LUMP_COUNT};

pub struct BufferedBSP {
    pub header: BSPHeader,
    pub data_without_header: Vec<u8>,
}

impl BufferedBSP {
    pub fn lump_slice(&self, lump_index: LumpIndex) -> &[u8] {
        let lump = &self.header.lumps[lump_index as usize];
        if lump.off < BSP_HEADER_LEN {
            return &[];
        }

        &self.data_without_header
            [(lump.off - BSP_HEADER_LEN) as usize..(lump.off + lump.len - BSP_HEADER_LEN) as usize]
    }

    pub fn replace_lump(&mut self, lump_index: LumpIndex, new_lump: Vec<u8>) {
        let mut lump = &mut self.header.lumps[lump_index as usize];
        let new_lump_len = new_lump.len() as u32;
        let lump_len_diff = (new_lump_len as i32) - (lump.len as i32);

        // replace range in data
        self.data_without_header
            .splice(
                (lump.off - BSP_HEADER_LEN) as usize
                    ..(lump.off + lump.len - BSP_HEADER_LEN) as usize,
                new_lump,
            )
            .count();

        // set new lump length in header
        lump.len = new_lump_len;

        // fix offset of all lumps with offsets higher than this lump's offset
        let lump_off = lump.off;
        for mut some_lump in &mut self.header.lumps {
            if some_lump.off > lump_off {
                some_lump.off = (some_lump.off as i32 + lump_len_diff) as u32;
            }
        }
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<(), Box<dyn std::error::Error>> {
        writer.write_all(b"VBSP")?;
        writer.write_u32::<LittleEndian>(self.header.version)?;

        for i in 0..BSP_LUMP_COUNT {
            writer.write_u32::<LittleEndian>(self.header.lumps[i].off)?;
            writer.write_u32::<LittleEndian>(self.header.lumps[i].len)?;
            writer.write_u32::<LittleEndian>(self.header.lumps[i].version)?;
            writer.write_all(&self.header.lumps[i].ident)?;
        }

        writer.write_u32::<LittleEndian>(self.header.map_revision)?;

        writer.write_all(&self.data_without_header)?;

        Ok(())
    }
}
