use steamws;
use steamworks;
use std::str::FromStr;
use std::{thread, time};
use std::fs;
use std::fs::File;
use std::fs::metadata;
use std::io;
use std::io::Read;
use clap::Clap;
use lzma::LzmaReader;

#[derive(Clap)]
#[clap()]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    #[clap()]
    Get(GetCommand),

    #[clap()]
    Info(InfoCommand),
}

#[derive(Clap)]
struct GetCommand {
    input: String
}

#[derive(Clap)]
struct InfoCommand {
    input: String
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Info(t) => {
            let id = u64::from_str(&t.input)?;
            let dets = steamws::workshop::published_file_details(id)
                .await?;
            println!("{:#?}", dets.unwrap());
        },
        SubCommand::Get(t) => {
            let num_id = u64::from_str(&t.input)?;

            let (cl, _scl) = steamworks::Client::init().unwrap();
            let ugc = cl.ugc();

            let id = steamworks::PublishedFileId(num_id);
            let b = ugc.download_item(id, true);
            if !b {
                eprintln!("download_item returned false; is steam running etc?");
            }
            loop {
                let state = ugc.item_state(id);
                if (state & steamworks::ItemState::INSTALLED) == steamworks::ItemState::INSTALLED {
                    break;
                }
                let info = ugc.item_download_info(id);
                eprintln!("download info: {:?}", info);
                let ten_millis = time::Duration::from_millis(100);
                thread::sleep(ten_millis);
            }
            eprintln!("installed!");

            let install_info = ugc.item_install_info(id).unwrap();
            eprintln!("{:?}", install_info);
            let folder = install_info.folder;

            let md = metadata(&folder).unwrap();

            let mut reader: Box<dyn Read>;

            // Legacy workshop items can be direct files
            if md.is_file() {
                let file = File::open(folder)?;
                // Legacy gmod binaries are LZMA compressed
                // TODO verify if file is LZMA before doing this
                reader = Box::new(LzmaReader::new_decompressor(file).unwrap());
            } else {
                let files = fs::read_dir(folder).unwrap().collect::<Vec<_>>();
                if files.len() != 1 {
                    eprintln!("Downloaded item contains more than one file! Specify the file you want with --file");
                }
    
                let path = files[0].as_ref().unwrap().path();
                eprintln!("path: {:?}", path.display());
                let file = File::open(path)?;
                reader = Box::new(file);
            }

            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            io::copy(&mut reader, &mut stdout)?;
        }
    }
    Ok(())
}
