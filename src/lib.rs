pub mod bsp;
pub mod dependency;
pub mod gma;
pub mod mdl;
pub mod vmt;
pub mod workshop;

pub fn human_readable_size(size: u64) -> String {
    if size < 1000 {
        format!("{}B", size)
    } else if size < 1_000_000 {
        format!("{:.2}K", size as f64 / 1_000f64)
    } else {
        format!("{:.2}M", size as f64 / 1_000_000f64)
    }
}
