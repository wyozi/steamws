use std::fs;
use std::path::{Path, PathBuf};
use std::vec::Vec;
use std::collections::HashSet;

use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(author, about, version)]
struct Opts {
    #[command(subcommand)]
    subcmd: SubCommand,
}

#[derive(Subcommand)]
enum SubCommand {
    /// Lists all dependencies of this .mdl
    ///
    /// Namely, this will return a list of paths that
    /// are required for the .mdl to function, including the
    /// .mdl file itself.
    ///
    /// The paths are returned relative to the working directory
    #[command(alias = "deps")]
    Dependencies(DependenciesCommand),

    /// List skins in this .mdl
    #[command()]
    Skins(SkinsCommand),

    /// Copies given .mdl with dependencies to target path
    ///
    /// Maintains the folder structure, including materials.
    /// For that reason, the target directory should be one
    /// with "models" and "materials" folders
    #[command(alias = "cp")]
    Copy(CopyCommand),

    /// Removes given .mdl (and optionally its dependencies)
    ///
    /// By default this command removes just direct dependencies
    /// (the files references the .mdl in the same models folder).
    /// This behavior can be changed with the deps flag
    #[command(alias = "rm")]
    Remove(RemoveCommand),
}

#[derive(Args)]
struct DependenciesCommand {
    /// Source mdl
    input: String,

    /// Show dependencies only for given skin index
    #[arg(long)]
    skin: Option<u16>,

    /// Print dependencies in graphviz format
    /// 
    /// Example (OS X):
    /// `mdl --deps --dot mymodel.mdl | dot -Tpng | open -a Preview.app -f`
    #[arg(long)]
    dot: bool,
}

#[derive(Args)]
struct SkinsCommand {
    /// Source mdl
    input: String,
}

#[derive(Args)]
struct CopyCommand {
    /// Source mdl
    input: String,

    /// Only copy texture dependencies required by given skin index
    #[arg(long)]
    skin: Option<u16>,

    /// Only copy direct dependencies (excludes e.g. materials)
    #[arg(long)]
    direct_deps: bool,

    /// Where mdl and dependencies will be placed.
    /// Should be the folder containing "models" and "materials"
    output_folder: String,

    /// Prints what the command would copy if executed without
    /// this flag
    #[arg(long, short = 'n')]
    dry_run: bool,
}

#[derive(Args)]
struct RemoveCommand {
    /// Mdl file to remove
    input: String,

    /// Also remove indirect dependencies.
    ///
    /// Indirect assets are the ones that other models may rely on,
    /// such as materials and textures.
    #[arg(long, short)]
    all_deps: bool,

    /// Prints what the command would remove if executed without
    /// this flag
    #[arg(long, short = 'n')]
    dry_run: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Dependencies(t) => {
            let path = Path::new(&t.input);

            let mdl = steamws::mdl::MDLFile::open(path)?;
            let mut deps = mdl.dependencies()?;

            if let Some(skin) = t.skin {
                let skin_mats = &mdl.skins_with_material_paths()[skin as usize];

                let found_skin_mats: HashSet<&PathBuf> =
                    skin_mats.iter()
                    .filter_map(|s| s.as_ref())
                    .collect();
                deps.filter_root_dependencies(|d| {
                    matches!(d, steamws::mdl::MDLDependency::Material(p) if found_skin_mats.contains(p))
                })
            }

            if t.dot {
                println!("{}", deps.dot());
            } else {
                for dep in deps.flatten() {
                    println!("{}", dep.path().to_str().unwrap());
                }
            }

            Ok(())
        }
        SubCommand::Skins(t) => {
            let path = Path::new(&t.input);

            let mdl = steamws::mdl::MDLFile::open(path)?;
            for (i, skin) in mdl.skins_with_material_paths().iter().enumerate() {
                println!("Skin #{}", i);
                for tex in skin {
                    println!("{:?}", tex);
                }
                println!();
            }

            Ok(())
        }
        SubCommand::Copy(t) => {
            let path = Path::new(&t.input);
            let out_path = Path::new(&t.output_folder);

            let mut copy_map: Vec<(PathBuf, PathBuf)> = vec![];

            let mdl = steamws::mdl::MDLFile::open(path)?;
            let mut deps = mdl.dependencies()?;

            if t.direct_deps {
                deps.filter_root_edges(|e| {
                    matches!(e, steamws::mdl::MDLDependencyType::Direct)
                });
            }

            if let Some(skin) = t.skin {
                let skin_mats = &mdl.skins_with_material_paths()[skin as usize];

                let found_skin_mats: HashSet<&PathBuf> =
                    skin_mats.iter()
                    .filter_map(|s| s.as_ref())
                    .collect();
                deps.filter_root_dependencies(|d| {
                    matches!(d, steamws::mdl::MDLDependency::Material(p) if found_skin_mats.contains(p))
                })
            }

            let assets_path = mdl.assets_path();
            for dep in &deps.flatten() {
                let bare_dep = dep.path().strip_prefix(assets_path)?.to_path_buf();
                copy_map.push((dep.path().to_path_buf(), out_path.join(bare_dep)));
            }

            if t.dry_run {
                println!("Would do following copy operations (dry run): ");
                println!();
            }

            let mut size = 0;

            for (from, to) in copy_map {
                let this_size = fs::metadata(&from).ok().map(|m| m.len());
                println!(
                    "{} -> {} ({})",
                    from.display(),
                    to.display(),
                    this_size
                        .map(|s| steamws::human_readable_size(s))
                        .unwrap_or_else(|| "Unknown".to_string())
                );
                if !t.dry_run {
                    fs::create_dir_all(&to.parent().unwrap())?;
                    fs::copy(&from, &to)?;
                } else {
                    size += this_size.unwrap_or(0);
                }
            }

            if t.dry_run {
                println!();
                println!("Totaling {} in size", steamws::human_readable_size(size));
            }

            Ok(())
        }
        SubCommand::Remove(t) => {
            let path = Path::new(&t.input);
            let mdl = steamws::mdl::MDLFile::open(path)?;

            if t.dry_run {
                println!("Would remove the following files (dry run): ");
                println!();
            }

            let mut size = 0;

            let mut deps = mdl.dependencies()?;

            if !t.all_deps {
                deps.filter_root_edges(|e| {
                    matches!(e, steamws::mdl::MDLDependencyType::Direct)
                });
            }

            for dep in &deps.flatten() {
                println!("{}", dep.path().display());

                if !t.dry_run {
                    fs::remove_file(dep.path())?;
                } else {
                    size += fs::metadata(dep.path()).ok().map(|m| m.len()).unwrap_or(0);
                }
            }

            if t.dry_run {
                println!();
                println!("Totaling {} in size", steamws::human_readable_size(size));
            }

            Ok(())
        }
    }
}
