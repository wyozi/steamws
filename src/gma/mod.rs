mod read;
pub use read::read_gma;

mod write;
pub use write::write_gma;

use serde::Deserialize;

pub const GMA_HEADER: &'static [u8; 4] = b"GMAD";
pub const SUPPORTED_GMA_VERSION: u8 = 3;

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

#[derive(Deserialize, Debug)]
pub struct AddonJson {
    pub title: String,
    #[serde(rename = "type")]
	pub addon_type: String,
    pub tags: Vec<String>,
    pub ignore: Vec<String>,
}