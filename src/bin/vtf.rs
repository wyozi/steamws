use std::io::Read;
use std::path::Path;
use std::vec::Vec;
use std::{fs::File, io::Write};

use clap::{Args, Parser, Subcommand};
use image::DynamicImage;
use steamws::vtf::ImageFormat;

#[derive(Parser)]
#[command(author, about, version)]
struct Opts {
    #[command(subcommand)]
    subcmd: SubCommand,
}

#[derive(Subcommand)]
enum SubCommand {
    /// Prints metadata about given vtf
    #[command()]
    Info(InfoCommand),

    /// Converts vtf to png
    #[command()]
    Convert(ConvertCommand),

    /// Resize given vtf
    #[command()]
    Resize(ResizeCommand),
}

#[derive(Args)]
struct InfoCommand {
    /// Source vtf
    input: String,
}

#[derive(Args)]
struct ConvertCommand {
    /// Source vtf
    input: String,

    output: String,
}

#[derive(Args)]
struct ResizeCommand {
    /// Source vtf
    input: String,

    width: u32,

    height: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Info(t) => {
            let path = Path::new(&t.input);
            let mut file = File::open(path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            let vtf = steamws::vtf::from_bytes(&mut buf)?;
            println!("{:#?}", vtf.header);

            Ok(())
        }
        SubCommand::Convert(t) => {
            let path = Path::new(&t.input);
            let mut file = File::open(path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            let vtf = steamws::vtf::from_bytes(&mut buf)?;
            let image = vtf.highres_image.decode(0)?;

            // rgb and rgba images we can save directly, for other formats we convert to rgba
            match image {
                DynamicImage::ImageRgb8(_) => image.save(t.output)?,
                DynamicImage::ImageRgba8(_) => image.save(t.output)?,
                //DynamicImage::ImageBgra8(_) => image.to_rgba8().save(t.output)?,
                _ => image.to_rgb8().save(t.output)?,
            };

            Ok(())
        }
        SubCommand::Resize(t) => {
            let path = Path::new(&t.input);
            let mut file = File::open(path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            let vtf = steamws::vtf::from_bytes(&mut buf)?;

            let image = vtf.highres_image.decode(0)?;

            let out = "out.vtf";

            let resized = image.resize(t.width, t.height, image::imageops::FilterType::CatmullRom);

            let vtf_data = steamws::vtf::create(resized, ImageFormat::Dxt5)?;

            let path = Path::new(out);
            let mut file = File::create(path)?;
            file.write(&vtf_data)?;

            Ok(())
        }
    }
}
