mod read;
pub use read::read_gma;

pub struct GMAFile {
    pub name: String,
    pub description: String,
    pub author: String,
    pub entries: Vec<GMAEntry>
}
pub struct GMAEntry {
    pub name: String,
    pub size: u64,
    pub crc: u32,
    pub contents: Option<Vec<u8>>
}
