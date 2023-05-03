use std::io::Write;

use byteorder::{LittleEndian, WriteBytesExt};

use super::{lump_indices::LumpIndex, BSPHeader, BSP_LUMP_COUNT};

pub struct BufferedBSP {
    pub header: BSPHeader,
    pub data_without_header: Vec<u8>,
}

impl BufferedBSP {
    pub fn lump_slice(&self, lump_index: LumpIndex) -> &[u8] {
        let lump = &self.header.lumps[lump_index as usize];

        &self.data_without_header[lump.off as usize..(lump.off + lump.len) as usize]
    }

    pub fn replace_lump(&mut self, lump_index: LumpIndex, new_lump: &[u8]) {
        let mut lump = &mut self.header.lumps[lump_index as usize];
        assert!(
            new_lump.len() < lump.len as usize,
            "new lump must be smaller than old lump due to how we replace it"
        );

        // zero old slice
        let old_slice =
            &mut self.data_without_header[lump.off as usize..(lump.off + lump.len) as usize];
        for i in old_slice {
            *i = 0;
        }

        // set new slice
        self.data_without_header[lump.off as usize..(lump.off) as usize + new_lump.len()]
            .copy_from_slice(new_lump);

        // set length in header
        lump.len = new_lump.len() as u32;
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<(), Box<dyn std::error::Error>> {
        writer.write_all(b"VBSP")?;
        writer.write_u32::<LittleEndian>(self.header.version)?;

        for i in 0..BSP_LUMP_COUNT {
            writer.write_u32::<LittleEndian>(self.header.lumps[i].off)?;
            writer.write_u32::<LittleEndian>(self.header.lumps[i].len)?;
            writer.write_u32::<LittleEndian>(self.header.lumps[i].version)?;
            writer.write_u32::<LittleEndian>(self.header.lumps[i].ident)?;
        }

        writer.write_u32::<LittleEndian>(self.header.map_revision)?;

        writer.write_all(&self.data_without_header)?;

        Ok(())
    }
}
