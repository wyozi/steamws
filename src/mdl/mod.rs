use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

mod binary;

#[derive(Debug)]
pub enum MDLDependency {
    Direct(PathBuf),
    Material(PathBuf),
    Texture(PathBuf),
}
impl MDLDependency {
    pub fn is_direct(&self) -> bool {
        match self {
            MDLDependency::Direct(_) => true,
            _ => false
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            MDLDependency::Direct(p) => p,
            MDLDependency::Material(p) => p,
            MDLDependency::Texture(p) => p,
        }
    }
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
        let parent_traverse_count = self.partial.name.matches("/").count() + 1;
        let mut assets_path = self.path.as_path();
        for _ in 0..=parent_traverse_count {
            assets_path = assets_path.parent().unwrap();
        }
        assets_path
    }

    pub fn dependencies(&self) -> Result<Vec<MDLDependency>, Box<dyn std::error::Error>> {
        let assets_path = self.assets_path();
        let materials_path = assets_path.join("materials");

        let mut deps = vec![];

        deps.push(MDLDependency::Direct(self.path.to_path_buf()));
        let mdl_containing_folder = self.path.parent().unwrap();
        let mdl_stem = self.path.file_stem().unwrap().to_str().unwrap();
        for entry in fs::read_dir(mdl_containing_folder)? {
            let path = entry?.path();
            if path.is_file()
                && path.file_name().unwrap().to_str().unwrap().starts_with(mdl_stem)
                && path != self.path
            {
                deps.push(MDLDependency::Direct(path.to_path_buf()));
            }
        }

        for mat in &self.partial.materials {
            let cleaned_up = mat.replace("\\", "/");
            let mat_path = materials_path.join(format!("{}.vmt", cleaned_up));
            if mat_path.exists() {
                deps.push(MDLDependency::Material(mat_path.to_path_buf()));
                let vmt = crate::vmt::read(&mat_path)?;
                for tex in vmt.textures {
                    let cleaned_up = tex.replace("\\", "/");
                    let tex_path = materials_path.join(format!("{}.vtf", cleaned_up));
                    if tex_path.exists() {
                        deps.push(MDLDependency::Texture(tex_path.to_path_buf()));
                    }
                }
            }
        }

        Ok(deps)
    }
}
