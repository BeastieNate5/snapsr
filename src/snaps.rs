use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Snap {
    path: String,
    items: Vec<String>
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

fn read_snap_config(path: String) -> Option<HashMap<String, Snap>> {
    let data = match fs::read_to_string(path) {
        Ok(txt) => txt,
        Err(_) => {
            return None;
        }
    };

    let snaps : HashMap<String, Snap> = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(_) => {
            return None
        }
    };

    Some(snaps)
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

pub fn take_snap(snap_name: String, snap_config_path: Option<String>) {
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
            let snap = path::PathBuf::from(&path);
            snap
        },

        None => {
            let snap = get_snap_config_dir();
            let snap = path::Path::new(&snap).join("snaps.jsonc");
            snap
        }
    };
    
    let snap = match snap.to_str() {
        Some(txt) => txt,
        None => {
            println!("[\x1b[1;91m-\x1b[0m] Failed to read Snaps config directory");
            return
        }
    };

    let snap = match read_snap_config(snap.to_string()) {
        Some(data) => data,
        None => {
            println!("[\x1b[1;91m-\x1b[0m] Failed to read snap config");
            return;
        }
    };

    let snap_dir = get_snaps_dir();
    let snap_dir = path::Path::new(&snap_dir).join(&snap_name);

    if let Err(_) = fs::create_dir_all(&snap_dir) {
        eprintln!("[\x1b[1;91m-\x1b[0m] Failed to create snap directory");
        return
    }

    if let Err(_) = fs::copy(get_snap_config_dir() + "/snaps.jsonc", snap_dir.join("snap.jsonc")) {
        eprintln!("[\x1b[1;91m-\x1b[0m] Failed to copy snap config to snap {snap_name}");
        return
    }

    let mut total_size: u64 = 0;

    match fs::copy(get_snap_config_dir() + "/snaps.jsonc", snap_dir.join("snap.jsonc")) {
        Ok(size) => {
            total_size += size;
        }
        Err(_) => {
            eprintln!("[\x1b[1;91m-\x1b[0m] Failed to copy snap config to snap {snap_name}");
            return;
        }
    }

    let mut total_items = 0;
    let mut items_copied = 0;

    for (module, config) in snap {
        let module_dir = snap_dir.join(&module);

        if let Err(_) = fs::create_dir_all(&module_dir) {
            println!("[\x1b[1;91m-\x1b[0m] Failed to create module directory for {module}");
            continue;
        }

        for item in config.items {
            total_items += 1;
            let full_path = path::Path::new(&config.path).join(&item);

            if let Err(_) = fs::copy(&full_path, module_dir.join(&item)) {
                println!("[\x1b[1;91m-\x1b[0m] Failed to save {item}, skipping");
                continue;
            }
            
            let file_meta = fs::metadata(full_path);

            if let Ok(meta_data) = file_meta {
                total_size += meta_data.len(); 
            }

            println!("[\x1b[1;92m+\x1b[0m] Saved {item} ({module})");
            items_copied += 1;
        }
    }

    println!("[\x1b[1;92m+\x1b[0m] Snape {snap_name} successfully saved");
    println!("Saved {items_copied}/{total_items} items ({}kb)", total_size/100);
}

pub fn transfer_snap(snap_name: String) {
    let snaps = get_all_snaps();

    if !snaps.contains(&snap_name) {
        println!("[\x1b[1;91m-\x1b[0m] Snap {snap_name} does not exist");
        return;
    }

    let snap_dir = get_snaps_dir();
    let snap_dir = path::Path::new(&snap_dir).join(&snap_name);

    let snap_config_path = path::Path::new(&snap_dir).join("snap.jsonc");
    
    let snap_config_path = match snap_config_path.to_str() {
        Some(txt) => txt,
        None => {
            eprintln!("[\x1b[1;91m-\x1b[0m] Failed to read snap {snap_name} config");
            return;
        }
    };

    let snap = match read_snap_config(snap_config_path.to_string()) {
        Some(data) => data,
        None => {
            println!("[\x1b[1;91-\x1b[0m] Failed to read snap {snap_name} config");
            return;
        }
    };
    
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
