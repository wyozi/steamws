use std::io::Read;
use std::io;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Cursor;
use std::ffi::CStr;
use byteorder::{LittleEndian, ReadBytesExt};
use err_derive::Error;

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
    // https://gitlab.h08.us/puff/project-spahget/-/blob/fb558f2c425c63623180f864192442b766218e44/mdl/valve.py#L243
    let table_width = skinreference_count / skinrfamily_count;

    let mut skins = vec!();
    for _skin in 0..skinreference_count {
        let mut skin = vec!();
        for _replaced_tex_id in 0..table_width {
            let texture_id = reader.read_u16::<LittleEndian>()?;
            skin.push(texture_id);
        }
        skins.push(MDLSkin(skin));
    }

    Ok(PartialMDL {
        name: name,
        texture_names: texture_names,
        texture_dirs: texture_dirs,
        skin_count: skinrfamily_count,
        texture_slot_count: table_width,
        skins: skins
    })
}