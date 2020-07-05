use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use crate::dependency::DependencyGraph;

mod binary;

#[derive(Debug, Clone)]
pub enum MDLDependency {
    /// The .mdl itself and auxiliary files
    Model(PathBuf),
    Material(PathBuf),
    Texture(PathBuf),
}
impl MDLDependency {
    pub fn path(&self) -> &Path {
        match self {
            MDLDependency::Model(p) => p,
            MDLDependency::Material(p) => p,
            MDLDependency::Texture(p) => p,
        }
    }
}

#[derive(Debug, Clone)]
pub enum MDLDependencyType {
    Direct,
    Indirect
}

pub struct MDLFile {
    path: PathBuf,
    partial: binary::PartialMDL,
}
impl MDLFile {
    pub fn open(p: &Path) -> Result<MDLFile, Box<dyn std::error::Error>> {
        let mut file = File::open(p)?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        let partial = binary::read(&mut buf)?;

        Ok(MDLFile {
            path: p.to_path_buf(),
            partial: partial,
        })
    }

    pub fn assets_path(&self) -> &Path {
        let cleaned_name = self.partial.name.replace("\\", "/");

        let parent_traverse_count = cleaned_name.matches("/").count() + 1;
        let mut assets_path = self.path.as_path();
        for _ in 0..=parent_traverse_count {
            assets_path = assets_path.parent().unwrap();
        }
        assets_path
    }

    fn discover_texture_path(&self, base_path: &Path, tex_name: &str) -> Option<PathBuf> {
        for dir in &self.partial.texture_dirs {
            let s = &format!("{}{}.vmt", dir, tex_name).replace("\\", "/").to_lowercase();
            let path = base_path.join(s);
            if path.exists() {
                return Some(path.to_path_buf());
            }
        }
        None
    }

    pub fn dependencies(&self) -> Result<DependencyGraph<MDLDependency, MDLDependencyType>, Box<dyn std::error::Error>> {
        let assets_path = self.assets_path();
        let materials_path = assets_path.join("materials");

        let mut deps = DependencyGraph::new(MDLDependency::Model(self.path.to_path_buf()));

        let mdl_containing_folder = self.path.parent().unwrap();
        let mdl_stem = self.path.file_stem().unwrap().to_str().unwrap();
        for entry in fs::read_dir(mdl_containing_folder)? {
            let path = entry?.path();
            if path.is_file()
                && path.file_name().unwrap().to_str().unwrap().starts_with(mdl_stem)
                && path != self.path
            {
                deps.insert(MDLDependency::Model(path.to_path_buf()), MDLDependencyType::Direct);
            }
        }

        let mut discovered_textures = HashSet::new();
        for mat_name in &self.partial.texture_names {
            let discovered = self.discover_texture_path(&materials_path, mat_name);

            if let Some(mat_path) = discovered {
                let mat_dep = deps.insert(MDLDependency::Material(mat_path.to_path_buf()), MDLDependencyType::Indirect);
                let vmt = crate::vmt::read(&mat_path)?;
                for tex in vmt.textures {
                    let cleaned_up = tex.replace("\\", "/").to_lowercase();
                    let tex_path = materials_path.join(format!("{}.vtf", cleaned_up));
                    if tex_path.exists() && !discovered_textures.contains(&tex_path) {
                        deps.insert_sub(mat_dep, MDLDependency::Texture(tex_path.to_path_buf()), MDLDependencyType::Indirect);
                        discovered_textures.insert(tex_path);
                    }
                }
            }
        }

        Ok(deps)
    }

    /// Return each skin as a vector of simple material names
    pub fn skins_with_material_names(&self) -> Vec<Vec<String>> {
        self.partial.skins
            .iter()
            .map(|s| {
                s.0.iter()
                    .map(|slot| self.partial.texture_names[*slot as usize].clone())
                    .collect()
            })
            .collect()
    }

    /// Return each skin as a vector of absolute paths to the materials
    pub fn skins_with_material_paths(&self) -> Vec<Vec<Option<PathBuf>>> {
        let assets_path = self.assets_path();
        let materials_path = assets_path.join("materials");

        self.partial.skins
            .iter()
            .map(|s| {
                s.0.iter()
                    .map(|slot| {
                        let tex_name = &self.partial.texture_names[*slot as usize];
                        self.discover_texture_path(&materials_path, &tex_name)
                    })
                    .collect()
            })
            .collect()
    }
}
