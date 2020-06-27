use std::fs;
use std::path::Path;
use std::collections::HashSet;

use lazy_static::lazy_static;
use regex::RegexBuilder;
use regex::Regex;


#[derive(Debug)]
pub struct VMT {
    pub textures: Vec<String>
}

pub fn read(path: &Path) -> Result<VMT, Box<dyn std::error::Error>> {
    lazy_static! {
        static ref RE: Regex = RegexBuilder::new(r"\$(?P<key>\w+)\s+(?P<value>.*)$")
            .multi_line(true)
            .build()
            .unwrap();
        static ref KEYS: HashSet<&'static str> = {
            let mut m = HashSet::new();
            m.insert("basetexture");
	        m.insert("iris");
	        m.insert("ambientoccltexture");
	        m.insert("bumpmap");
	        m.insert("phongexponenttexture");
	        m.insert("detail");
	        m.insert("selfillummask");
            m
        };
    }
    let string = fs::read_to_string(path)?;

    let textures: Vec<String> = RE.captures_iter(&string)
        .filter(|c| KEYS.contains(&c["key"]))
        .map(|c| c["value"].trim().to_owned())
        .collect();
    Ok(VMT {
        textures: textures
    })
}
