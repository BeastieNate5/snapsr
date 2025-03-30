use std::collections::HashMap;
use std::fs;
use std::path;

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

fn read_snap_config(path: String) -> HashMap<String, Snap> {
    let data = fs::read_to_string(path).expect("Unable to read snaps config");
    let snaps : HashMap<String, Snap> = serde_json::from_str(&data).expect("Snap config in invalid format");
    snaps
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

pub fn take_snap(snap_name: String) {
    let snap = get_snap_config_dir();
    let snap = path::Path::new(&snap).join("snaps.jsonc");
    
    let snap = match snap.to_str() {
        Some(txt) => txt,
        None => {
            println!("Failed to read Snaps config dir");
            return
        }
    };

    let snap = read_snap_config(snap.to_string());

    let snap_dir = get_snaps_dir();
    let snap_dir = path::Path::new(&snap_dir).join(&snap_name);

    if let Err(_) = fs::create_dir_all(&snap_dir) {
        eprintln!("Failed to create snap directory");
        return
    }

    if let Err(_) = fs::copy(get_snap_config_dir() + "/snaps.jsonc", snap_dir.join("snap.jsonc")) {
        eprintln!("Failed to copy snap config to snap {snap_name}");
        return
    }

    for (module, config) in snap {
        let module_dir = snap_dir.join(&module);

        if let Err(_) = fs::create_dir_all(&module_dir) {
            println!("Failed to create module directory for {module}");
            continue;
        }

        for item in config.items {
            let full_path = path::Path::new(&config.path).join(&item);

            if let Err(_) = fs::copy(full_path, module_dir.join(&item)) {
                println!("Failed to copy {item}, skipping");
                continue;
            }
            println!("Copied {item}");
        }
    }

    println!("Snaped {snap_name}");
}

pub fn transfer_snap(snap_name: String) {
    let snaps = get_all_snaps();

    if !snaps.contains(&snap_name) {
        println!("Snap {snap_name} does not exist");
        return;
    }

    let snap_dir = get_snaps_dir();
    let snap_dir = path::Path::new(&snap_dir).join(&snap_name);

    let snap_config_path = path::Path::new(&snap_dir).join("snap.jsonc");
    
    let snap_config_path = match snap_config_path.to_str() {
        Some(txt) => txt,
        None => {
            eprintln!("Failed to read snap {snap_name} config");
            return;
        }
    };

    let snap = read_snap_config(snap_config_path.to_string());

    for (module_name, module) in snap {
        
        for item in module.items {
            let dst_path = path::Path::new(&module.path).join(&item);
            let src_path = path::Path::new(&snap_dir).join(&module_name).join(&item);
            
            if let Err(_) = fs::copy(src_path, dst_path) {
                println!("Failed to transfer item {item} from module {module_name}");
                continue;
            }

            println!("Transferred {item} from module {module_name}");
        }
    }

    println!("Transfer complete");
}

pub fn delete_snap(snap: String) {
    let snaps = get_all_snaps();

    if !snaps.contains(&snap) {
        println!("Snap {snap} does not exist");
        return;
    }

    let snap_dir = get_snaps_dir();
    let snap_dir = path::Path::new(&snap_dir).join(&snap);

    if let Err(_) = fs::remove_dir_all(snap_dir) {
        eprintln!("Failed to delete snap");
        return;
    }

    println!("Deleted {snap} snap");
}

pub fn list_snaps() {
    let snaps = get_all_snaps();
    for snap in snaps {
        println!("{snap}");
    } 
}
