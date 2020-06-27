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
    name: String,
    pub materials: Vec<String>
}

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

    let mut texture_names = vec!();
    for i in 0..texture_count {
        let off: u64 = (texture_offset + i*16*4).into();
        reader.seek(SeekFrom::Start(off))?;

        let name_offset = reader.read_u32::<LittleEndian>()?;
        reader.seek(SeekFrom::Current(name_offset as i64 - 4))?;

        let tex_name = reader.read_padded_cstr(256)?;
        texture_names.push(tex_name);
    }

    let mut texture_paths = vec!();
    for i in 0..texturedir_count {
        let off: u64 = (texturedir_offset + i*4).into();
        reader.seek(SeekFrom::Start(off))?;

        let abs_offset = reader.read_u32::<LittleEndian>()?;
        reader.seek(SeekFrom::Start(abs_offset as u64))?;

        let texdir_name = reader.read_padded_cstr(256)?;

        for tex in &texture_names {
            texture_paths.push(format!("{}{}", texdir_name, tex));
        }
    }

    Ok(PartialMDL {
        name: name,
        materials: texture_paths
    })
}