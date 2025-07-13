use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path;
use std::io;
use std::path::{Path, PathBuf};
use std::error::Error;

use glob::glob;
use serde::Deserialize;
use serde::Serialize;
use chrono::prelude::*;

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Snap {
    path: String,
    items: Vec<String>
}

#[derive(Deserialize, Debug)]
struct SnapConfig {
    modules: HashMap<String, ModuleConfig>
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct ModuleConfig {
    include: Vec<String>,
    description: Option<String>,
    hooks: Option<Hooks>
}

#[derive(Serialize, Deserialize, Debug)]
struct Hooks {
    pre_load: Option<String>,
    post_load: Option<String>
}

#[derive(Serialize, Deserialize, Debug)]
struct SnapMetaData {
    timestamp: DateTime<Local>,
    size: u32,
    items: HashMap<String, PathBuf>,
    hooks: Option<Hooks>
}

impl SnapConfig {
    fn from(path: PathBuf) -> Option<Self> {
        match fs::read_to_string(path) {
            Ok(txt) => {
                match toml::from_str(&txt) {
                    Ok(config) => Some(config),
                    Err(_) => None
                }
            }

            Err(_) => None
        }
    }
}

impl ModuleConfig {
    fn get_item_paths(&self) -> Vec<PathBuf> {
        let mut items = Vec::new();

        for item in &self.include {
            for entry in glob(&item).expect("Never should happen") {
                match entry {
                    Ok(path) => {
                        let meta = fs::metadata(&path).unwrap();
                        if meta.is_file() {
                            items.push(path);
                        }
                    },
                    Err(_) => {
                        // NOTE TO SELF, add an error display here
                        //println!("");
                        continue;
                    }
                }
            }
        }

        items
    }
}


impl SnapMetaData {
    fn new(items: HashMap<String, PathBuf>) -> Self {
        return Self {
            timestamp: chrono::Local::now(),
            size: 0,
            items,
            hooks: None
        } 
    }

    #[allow(dead_code)]
    fn from(path: &PathBuf) -> Result<Self, Box<dyn Error>> {
        let data = fs::read_to_string(path)?;
        let data = serde_json::from_str(&data)?;
        Ok(data)
    }

    fn save(&self, path: &PathBuf) -> Result<(), Box<dyn Error>>{
        let json_data = serde_json::to_string(self)?;
        fs::write(path, json_data)?;
        Ok(())
    }
}

fn get_snap_config_dir() -> String {
    let user_home_dir = std::env::var("HOME").expect("Failed to read HOME env variable");
    let snaps_config_dir = user_home_dir + "/.config/snapsr";
    snaps_config_dir
}

fn get_snaps_dir() -> String {
    let user_home_dir = std::env::var("HOME").expect("Failed to read HOME env variable");
    let snaps_dir = user_home_dir + "/.config/snapsr/snaps";
    snaps_dir
}

fn get_all_snaps() -> Vec<String> {
    let snaps_dir = get_snaps_dir();
    let snaps_dir = path::Path::new(&snaps_dir);
    let mut snaps : Vec<String> = Vec::new();

    for entry in snaps_dir.read_dir().expect("Could not find snaps dir") {
        if let Ok(name) = entry {
            snaps.push( name.file_name().into_string().unwrap() );
        }
    }

    snaps
}

fn get_snap_size(path: &PathBuf) -> u64 {

    let entries = path.read_dir().expect("Error reading Snap directory");
    let mut size : u64 = 0;

    for entry in entries {
        if let Ok(entry_data) = entry {
            let path = entry_data.path();
            if path.is_dir() {
                size += get_snap_size(&path);
                continue;
            }

            let file = fs::metadata(&path).expect("???");        
            size += file.len();
            //println!("{:?} -> {}", path, file.len());
        }        
    }
    size
}

pub fn take_snap(snap_name: String, snap_config_path: Option<PathBuf>) {
    let saved_snaps = get_all_snaps();

    if saved_snaps.contains(&snap_name) {
        let mut input = String::new();
        print!("Snap {snap_name} already exist. Do you wish to overwrite (y/N)? ");

        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        let input = input.to_lowercase();

        if input != "y" && input != "yes" {
            return;
        }
    }

    let snap = match snap_config_path {
        Some(path) => {
            let path = path::PathBuf::from(&path);
            SnapConfig::from(path)
        },

        None => {
            let path = get_snap_config_dir();
            let path = path::Path::new(&path).join("config.toml");
            SnapConfig::from(path)
        }
    };

    let snap = match snap {
        Some(config) => config,
        None => {
            eprintln!("[\x1b[1;91m-\x1b[0m] Failed to create snap config");
            return
        }
    };
    

    let snap_dir = get_snaps_dir();
    let snap_dir = path::Path::new(&snap_dir).join(&snap_name);

    if let Err(_) = fs::create_dir_all(&snap_dir) {
        eprintln!("[\x1b[1;91m-\x1b[0m] Failed to create snap directory");
        return
    }

    let mut items_src_to_dst : HashMap<String, PathBuf> = HashMap::new();

    for (module_name, module) in &snap.modules {
        let module_dir = snap_dir.join(&module_name);

        if let Err(_) = fs::create_dir_all(&module_dir) {
            println!("[\x1b[1;91m-\x1b[0m] Failed to create module directory for {module_name}");
            continue;
        }

        let items = module.get_item_paths();

        for item in items {

            if let (Some(parent), Some(file_child)) = (item.parent(), item.file_name()) {
                if let Some(grandparent) = parent.file_name() {
                    let grandparent_key = grandparent.to_string_lossy();
                    let file_child_key = file_child.to_string_lossy();

                    let file_key = grandparent_key.to_string() + "_" + &file_child_key;
                    let saved_item_path = module_dir.join(file_key);

                    if let Ok(_) = fs::copy(&item, &saved_item_path) {
                        items_src_to_dst.insert(item.to_string_lossy().to_string(), saved_item_path);
                        println!("[\x1b[1;92m+\x1b[0m] Saved {} ({module_name})", item.display());
                    }
                    else {
                        println!("[\x1b[1;91m-\x1b[0m] Failed to save {}, skipping ({module_name})", item.display());
                    }
                } 
            }
        }
    }

    let snap_meta_data = SnapMetaData::new(items_src_to_dst);
    if let Ok(()) = snap_meta_data.save(&snap_dir.join("snap.json")) {
        println!("[\x1b[1;92m+\x1b[0m] Sucessfully saved Snap");
    }
    else {
        println!("[\x1b[1;91m-\x1b[0m] Failed to save, this snap will be unusable");
    }
}

pub fn transfer_snap(snap_name: String) {
    let snaps = get_all_snaps();

    if !snaps.contains(&snap_name) {
        println!("[\x1b[1;91m-\x1b[0m] Snap {snap_name} does not exist");
        return;
    }

    let snap_dir = get_snaps_dir();
    let snap_dir = path::Path::new(&snap_dir).join(&snap_name);

    let snap_config_path = path::Path::new(&snap_dir).join("snap.json");
    
    let snap_config_path = match snap_config_path.to_str() {
        Some(txt) => txt,
        None => {
            eprintln!("[\x1b[1;91m-\x1b[0m] Failed to read snap {snap_name} config");
            return;
        }
    };

    /*
    let _snap = match read_snap_config(snap_config_path.to_string()) {
        Some(data) => data,
        None => {
            println!("[\x1b[1;91-\x1b[0m] Failed to read snap {snap_name} config");
            return;
        }
    };
    */
 

    
    /*
    let mut total_items = 0;
    let mut items_transferred = 0;
    for (module_name, module) in snap {
        
        for item in module.items {
            total_items += 1;
            let dst_path = path::Path::new(&module.path).join(&item);
            let src_path = path::Path::new(&snap_dir).join(&module_name).join(&item);
            
            if let Err(_) = fs::copy(src_path, dst_path) {
                println!("[\x1b[1;91m-\x1b[0m] Failed to transfer item {item} from module {module_name}");
                continue;
            }

            println!("[\x1b[1;92m+\x1b[0m] Transferred {item} ({module_name})");
            items_transferred += 1;
        }
    }

    println!("[\x1b[1;92m+\x1b[0m] Transfer complete");
    println!("Transferred {items_transferred}/{total_items}");
    */
}

pub fn delete_snap(snap: String) {
    let snaps = get_all_snaps();

    if !snaps.contains(&snap) {
        println!("[\x1b[1;91m-\x1b[0m] Snap {snap} does not exist");
        return;
    }

    let snap_dir = get_snaps_dir();
    let snap_dir = path::Path::new(&snap_dir).join(&snap);

    if let Err(_) = fs::remove_dir_all(snap_dir) {
        eprintln!("[\x1b[1;91m-\x1b[0m] Failed to delete snap");
        return;
    }

    println!("[\x1b[1;92m+\x1b[0m] Deleted {snap} snap");
}

pub fn list_snaps() {
    let snaps = get_all_snaps();
    let snaps_dir = get_snaps_dir();

    for snap in snaps {
        let snap_dir = Path::new(&snaps_dir).join(&snap);
        let snap_size = get_snap_size(&snap_dir);
        println!("{snap}: {}kb", snap_size/100);
    }
}
