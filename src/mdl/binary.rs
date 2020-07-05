use std::io::Read;
use std::io;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Cursor;
use std::ffi::CStr;
use byteorder::{LittleEndian, ReadBytesExt};
use err_derive::Error;
use std::collections::HashSet;

#[derive(Debug)]
pub struct PartialMDL {
    pub name: String,

    pub texture_names: Vec<String>,
    pub texture_dirs: Vec<String>,

    pub skin_count: u32,
    pub texture_slot_count: u32,
    pub skins: Vec<MDLSkin>
}

#[derive(Debug)]
pub struct MDLSkin(pub Vec<u16>);

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "IO error: {}", _0)]
    Io(#[error(source)] std::io::Error),
    #[error(display = "File does not have a valid mdl header")]
    InvalidHeader
}

trait ReadPaddedCStrExt: Read {
    fn read_padded_cstr(&mut self, size: usize) -> Result<String, io::Error> {
        let mut buf = Vec::with_capacity(size);
        self.take(size as u64).read_to_end(&mut buf)?;
        
        let end = buf.iter().position(|&b| b == 0).map_or(0, |i| i + 1);
        Ok(CStr::from_bytes_with_nul(&buf[..end]).unwrap().to_str().unwrap().to_owned())
    }
}
impl<W: io::Read> ReadPaddedCStrExt for W {}


pub fn read(bytes: &mut Vec<u8>) -> Result<PartialMDL, Box<dyn std::error::Error>> {
    let mut reader = Cursor::new(bytes);

    let mut magic_buf = [0; 4];
    reader.read_exact(&mut magic_buf)?;
    if &magic_buf != b"IDST" {
        return Err(Box::new(Error::InvalidHeader));
    }

    reader.read_u32::<LittleEndian>()?; // version
    reader.read_u32::<LittleEndian>()?; // checksum

    let name = reader.read_padded_cstr(64)?;

    // skip to texture_count
    reader.seek(SeekFrom::Current((
        // size + vectors + flags
        4+12+12+12+12+12+12+4 +
        // misc offsets
        8+8+8+8+8+8
    ) as i64))?;

    let texture_count = reader.read_u32::<LittleEndian>()?;
    let texture_offset = reader.read_u32::<LittleEndian>()?;

    let texturedir_count = reader.read_u32::<LittleEndian>()?;
    let texturedir_offset = reader.read_u32::<LittleEndian>()?;

    let skinreference_count = reader.read_u32::<LittleEndian>()?;
    let skinrfamily_count = reader.read_u32::<LittleEndian>()?;
    let skinreference_index = reader.read_u32::<LittleEndian>()?;

    let mut texture_names = vec!();
    for i in 0..texture_count {
        let off: u64 = (texture_offset + i*16*4).into();
        reader.seek(SeekFrom::Start(off))?;

        let name_offset = reader.read_u32::<LittleEndian>()?;
        reader.seek(SeekFrom::Current(name_offset as i64 - 4))?;

        let tex_name = reader.read_padded_cstr(256)?;
        texture_names.push(tex_name);
    }

    let mut texture_dirs = vec!();
    for i in 0..texturedir_count {
        let off: u64 = (texturedir_offset + i*4).into();
        reader.seek(SeekFrom::Start(off))?;

        let abs_offset = reader.read_u32::<LittleEndian>()?;
        reader.seek(SeekFrom::Start(abs_offset as u64))?;

        let texdir_name = reader.read_padded_cstr(256)?;

        texture_dirs.push(texdir_name);
    }

    reader.seek(SeekFrom::Start(skinreference_index.into()))?;

    let width = skinrfamily_count as usize;
    let height = skinreference_count as usize;

    // Base 1d array
    // Vector of 'width' elements slices
    // Final 2d array `&mut [&mut [_]]`
    let mut skin_table_raw = vec![0; width * height];
    let mut skin_table_base: Vec<_> = skin_table_raw.as_mut_slice().chunks_mut(width).collect();
    let skin_table = skin_table_base.as_mut_slice();

    for family in 0..width {
        for reference in 0..height {
            let texture_id = reader.read_u16::<LittleEndian>()?;
            skin_table[reference][family] = texture_id;
        }
    }

    // Extraneous column culling algorithm
    // Credits https://github.com/cannon/quickpack/blob/d96d9f87556536893283fac8d2ef82240e41a9c9/QuickPack.py#L314
    let last_column = {
        let mut last_different_column = 0;
        let mut last_unique_column = 0;
        let mut unseen_indexes: HashSet<u16> = (0..(skinreference_count as u16)).collect();
        for x in 0..height {
            for y in 0..width {
                if skin_table[x][0] != skin_table[x][y] {
                    last_different_column = x;
                }
                if unseen_indexes.contains(&skin_table[x][y]) {
                    last_unique_column = x;
                    unseen_indexes.remove(&skin_table[x][y]);
                }
            }
        }
        last_different_column.max(last_unique_column)
    };

    let mut skins = vec!();
    for skin_index in 0..skinrfamily_count {
        let mut skin = vec!();
        for replaced_tex_id in 0..=last_column {
            skin.push(skin_table[replaced_tex_id as usize][skin_index as usize]);
        }
        skins.push(MDLSkin(skin));
    }

    Ok(PartialMDL {
        name: name,
        texture_names: texture_names,
        texture_dirs: texture_dirs,
        skin_count: skinrfamily_count,
        texture_slot_count: last_column as u32 + 1,
        skins: skins
    })
}