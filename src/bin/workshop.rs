use steamws;
use steamworks;
use steamworks::{PublishedFileId, AppId, SteamError, ItemState};
use std::str::FromStr;
use std::{thread, time};
use std::path::Path;
use std::fs;
use std::fs::File;
use std::fs::metadata;
use std::io;
use std::io::Read;
use clap::{Parser, Subcommand, Args};
use lzma::LzmaReader;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::sync::Mutex;


#[derive(Parser)]
#[command(author, about, version)]
struct Opts {
    /// Creates "steam_appid.txt" in working directory with given app id
    #[arg(short, long)]
    app_id: Option<String>,

    /// How loud we will be
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    subcmd: SubCommand,
}

#[derive(Subcommand)]
enum SubCommand {
    /// Prints info about a workshop item
    Info(InfoCommand),

    /// Downloads workshop item file and prints its contents to stdout
    Get(GetCommand),

    /// Creates a new workshop item
    /// 
    /// You must populate the item separately with `workshop update`
    Create(CreateCommand),

    /// Updates a new workshop item
    Update(UpdateCommand),
}

#[derive(Args)]
struct InfoCommand {
    workshop_id: String
}

#[derive(Args)]
struct GetCommand {
    workshop_id: String
}

#[derive(Args)]
struct CreateCommand {
}

#[derive(Args)]
struct UpdateCommand {
    workshop_id: String,

    /// Folder containing new item contents or "-" (for stdin). If omitted, only update metadata.
    input: Option<String>,

    /// Filename used for single-file items when piping into the update command.
    /// 
    /// If you use the "-" option for input, we will automatically create
    /// a temporary folder containing a single file (from stdin) and use that
    /// as the folder that will be uploaded to workshop.
    /// 
    /// This parameter determines the name of the file inside that folder.
    #[clap(long, default_value = "temp.gma")]
    content_file_name: String,

    /// Workshop item title
    #[clap(short, long)]
    title: Option<String>,

    /// Changelog message visible on the workshop changes pages
    #[clap(short, long)]
    message: Option<String>,
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

type CreateResult = Result<(PublishedFileId, bool), SteamError>;
fn create<M: steamworks::Manager>(scl: &steamworks::SingleClient<M>, ugc: &steamworks::UGC<M>, app_id: AppId) -> CreateResult {
    let data = Arc::new(Mutex::new(None));

    {
        let c_data = data.clone();
        ugc.create_item(app_id, steamworks::FileType::Community, move |res| {
            let mut data = c_data.lock().unwrap();
            *data = Some(res);
        });
    }

    while data.lock().unwrap().is_none() {
        scl.run_callbacks(); 
        ::std::thread::sleep(::std::time::Duration::from_millis(100)); 
    }

    return data.lock().unwrap().unwrap();
}

type SubmitUpdateResult = Result<(PublishedFileId, bool), SteamError>;
fn submit_update<M: steamworks::Manager>(scl: &steamworks::SingleClient<M>, handle: steamworks::UpdateHandle<M>, message: Option<&str>) -> SubmitUpdateResult {
    let data = Arc::new(Mutex::new(None));
    
    {
        let c_data = data.clone();
        handle.submit(message, move |res| {
            let mut data = c_data.lock().unwrap();
            *data = Some(res);
        });
    }

    while data.lock().unwrap().is_none() {
        scl.run_callbacks(); 
        ::std::thread::sleep(::std::time::Duration::from_millis(100)); 
    }

    return data.lock().unwrap().unwrap();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();
    let is_verbose = opts.verbose > 0; // TODO maybe we'll do multiple levels later

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

            Ok(())
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
                if is_verbose {
                    eprintln!("item state: {:?}", state);
                }
                if (state & ItemState::INSTALLED) == ItemState::INSTALLED {
                    
                    if !state.intersects(
                        ItemState::NEEDS_UPDATE | ItemState::DOWNLOADING | ItemState::DOWNLOAD_PENDING
                    ) {
                        break;
                    }
                    // installed but needs an update
                }
                let info = ugc.item_download_info(id);
                if is_verbose {
                    match info {
                        Some((downloaded, total)) if total > 0 => 
                            eprintln!("downloaded {} / {} ({}%)", downloaded, total, downloaded / total * 100),
                        _ => {}
                    }
                }

                thread::sleep(time::Duration::from_millis(500));
            }
            if is_verbose {
                eprintln!("workshop item set to installed! proceeding..");
            }

            let install_info = ugc.item_install_info(id).unwrap();

            if is_verbose {
                eprintln!("item info {:?}", install_info);
            }

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
                if is_verbose {
                    eprintln!("found likely item path: {:?}", path.display());
                }
                let file = File::open(path)?;
                reader = Box::new(file);
            }

            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            io::copy(&mut reader, &mut stdout)?;

            Ok(())
        },
        SubCommand::Create(_) => {
            let (cl, scl) = steamworks::Client::init().map_err(|e| WrappedSteamError(e))?;
            let ugc = cl.ugc();

            let app_id = cl.utils().app_id();

            let res = create(&scl, &ugc, app_id).map_err(|e| WrappedSteamError(e))?;
            match res {
                (steamworks::PublishedFileId(id), true) => {
                    eprintln!("Accept Steam Workshop legal agreement before publishing");
                    println!("{}", id);
                    Ok(())
                },
                (steamworks::PublishedFileId(id), false) => {
                    println!("{}", id);
                    Ok(())
                }
            }
        },
        SubCommand::Update(t) => {
            let num_id = u64::from_str(&t.workshop_id)?;

            let (cl, scl) = steamworks::Client::init().map_err(|e| WrappedSteamError(e))?;
            let ugc = cl.ugc();

            let app_id = cl.utils().app_id();
            let mut upd = ugc.start_item_update(app_id, PublishedFileId(num_id));
            if let Some(title) = t.title {
                upd = upd.title(&title);
            }
            if let Some(input) = t.input {
                if input == "-" {
                    let tempdir = tempfile::tempdir()?;
                    let tempdir_path = tempdir.into_path(); // TODO cleanup tempdir
                    let file_path = tempdir_path.join(t.content_file_name);
                    
                    {
                        if is_verbose {
                            eprintln!("creating a temporary file as the upload target: {:?}", file_path);
                        }
                        let stdin = io::stdin();
                        let mut handle = stdin.lock();
                        let mut f = File::create(file_path)?;
                        let b = io::copy(&mut handle, &mut f)?;
                        if is_verbose {
                            eprintln!("wrote {} bytes to the temporary file", b);
                        }
                    }

                    upd = upd.content_path(&tempdir_path);
                } else {
                    upd = upd.content_path(Path::new(&input));
                }
            }
            submit_update(&scl, upd, t.message.as_deref()).map_err(|e| WrappedSteamError(e))?;
            
            Ok(())
        }
    }
}
