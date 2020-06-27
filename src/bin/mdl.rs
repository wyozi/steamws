use std::fs::File;
use std::path::Path;
use std::vec::Vec;
use std::io::Read;

use clap::Clap;

#[derive(Clap)]
#[clap(author, about, version)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    /// Lists material and texture paths used by this .mdl
    /// 
    /// This includes both .vmt and transitively referenced .vtf
    /// Paths are returned relative to the working directory
    #[clap()]
    Materials(MaterialsCommand)
}

#[derive(Clap)]
struct MaterialsCommand {
    /// Source mdl
    input: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Materials(t) => {
            let path = Path::new(&t.input);
            let mut file = File::open(path)?;

            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            
            let mdl = steamws::mdl::read(&mut buf)?;

            let parent_traverse_count = mdl.name.matches("/").count() + 1;
            let mut assets_path = Path::new(&t.input);
            for _ in 0..=parent_traverse_count {
                assets_path = assets_path.parent().unwrap();
            }
            let materials_path = assets_path.join("materials");

            for mat in mdl.materials {
                let cleaned_up = mat.replace("\\", "/");
                let mat_path = materials_path.join(format!("{}.vmt", cleaned_up));
                if mat_path.exists() {
                    println!("{}", mat_path.to_str().unwrap());
                    let vmt = steamws::vmt::read(&mat_path)?;

                    for tex in vmt.textures {
                        let cleaned_up = tex.replace("\\", "/");
                        let tex_path = materials_path.join(format!("{}.vtf", cleaned_up));
                        if tex_path.exists() {
                            println!("{}", tex_path.to_str().unwrap());
                        }
                    }
                }
            }

            Ok(())
        }
    }
}