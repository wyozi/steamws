use steamws;
use steamworks;
use std::str::FromStr;
use std::{thread, time};
use std::path::Path;
use std::fs;
use std::fs::File;
use std::fs::metadata;
use std::io;
use std::io::Read;
use clap::Clap;
use lzma::LzmaReader;
use std::error::Error;
use std::fmt;

#[derive(Clap)]
#[clap(author, about, version)]
struct Opts {
    /// Creates "steam_appid.txt" in working directory with given app id
    #[clap(short, long)]
    app_id: Option<String>,

    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    /// Downloads workshop item file and prints its contents to stdout
    #[clap()]
    Get(GetCommand),

    /// Prints info about a workshop item
    #[clap()]
    Info(InfoCommand),
}

#[derive(Clap)]
struct GetCommand {
    workshop_id: String
}

#[derive(Clap)]
struct InfoCommand {
    workshop_id: String
}

struct SteamAppidHandle(bool);
impl SteamAppidHandle {
    fn create(id: &str) -> SteamAppidHandle {
        let path = Path::new("steam_appid.txt");
        if !path.exists() {
            fs::write(path, id).unwrap();
            SteamAppidHandle(true)
        } else {
            SteamAppidHandle(false)
        }
    }
}
impl Drop for SteamAppidHandle {
    fn drop(&mut self) {
        if self.0 {
            let path = Path::new("steam_appid.txt");
            if path.exists() {
                fs::remove_file(path).unwrap();
            }
        }
    }
}

#[derive(Debug)]
struct WrappedSteamError(steamworks::SteamError);
impl fmt::Display for WrappedSteamError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}
impl Error for WrappedSteamError {}
impl From<steamworks::SteamError> for WrappedSteamError {
    fn from(error: steamworks::SteamError) -> Self {
        WrappedSteamError(error)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    // Handle that implements Drop for automatic cleanup
    let _app_id_handle = match opts.app_id {
        Some(id) => Some(SteamAppidHandle::create(&id)),
        _ => None
    };

    match opts.subcmd {
        SubCommand::Info(t) => {
            let id = u64::from_str(&t.workshop_id)?;
            let dets = steamws::workshop::published_file_details(id)
                .await?;
            println!("{:#?}", dets);
        },
        SubCommand::Get(t) => {
            let num_id = u64::from_str(&t.workshop_id)?;

            let (cl, _scl) = steamworks::Client::init().map_err(|e| WrappedSteamError(e))?;
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

            let md = metadata(&folder)?;

            let mut reader: Box<dyn Read>;

            // Legacy workshop items can be direct files
            if md.is_file() {
                let file = File::open(folder)?;
                // Legacy gmod binaries are LZMA compressed
                // TODO verify if file is LZMA before doing this
                reader = Box::new(LzmaReader::new_decompressor(file)?);
            } else {
                let mut files = fs::read_dir(folder)?.collect::<Vec<_>>();
                if files.len() != 1 {
                    eprintln!("Downloaded item contains more than one file");
                }
    
                let path = files.remove(0)?.path();
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
