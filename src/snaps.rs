use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path;
use std::io;
use std::path::{Path, PathBuf};
use std::error::Error;
use std::process::Command;
use std::process::Stdio;

use glob::glob;
use serde::Deserialize;
use serde::Serialize;
use chrono::prelude::*;

enum HookStatus {
    Success,
    Error,
    Nothing
}

enum HookType {
    Pre,
    Post
}

#[derive(Deserialize, Debug)]
struct SnapConfig {
    modules: HashMap<String, ModuleConfig>,
    hooks: Option<Hooks>
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct ModuleConfig {
    include: Vec<String>,
    description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Hooks {
    pre_load: Option<String>,
    post_load: Option<String>
}

#[derive(Serialize, Deserialize, Debug)]
struct SnapMetaData {
    timestamp: DateTime<Local>,
    size: u64,
    items: HashMap<PathBuf, PathBuf>,
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
    fn new(items: HashMap<PathBuf, PathBuf>, hooks: Option<Hooks>, size: u64) -> Self {
        return Self {
            timestamp: chrono::Local::now(),
            size,
            items,
            hooks
        } 
    }

    fn from(path: &PathBuf) -> Option<Self> {
        let data = fs::read_to_string(path).ok()?;
        let data = serde_json::from_str(&data).ok()?;
        Some(data)
    }

    fn save(&self, path: &PathBuf) -> Result<(), Box<dyn Error>>{
        let json_data = serde_json::to_string(self)?;
        fs::write(path, json_data)?;
        Ok(())
    }

    fn run_hook(&self, hook_type: HookType) -> HookStatus {
        if let Some(ref hooks) = self.hooks {

            let selected_hook = match hook_type {
                HookType::Pre => &hooks.pre_load,
                HookType::Post => &hooks.post_load
            };

            if let Some(post_hook) = selected_hook {
                let mut command_splitted = post_hook.split_whitespace(); 
                let program = command_splitted.next();
                let _args: Vec<&str> = command_splitted.collect();

                if let Some(_program_txt) = program {
                    let mut child = Command::new("sh")
                        .arg("-c")
                        .arg(post_hook)
                        .stdout(Stdio::null())
                        .spawn();

                    if let Ok(ref mut child) = child {

                        let status = child.wait();
                        
                        if let Ok(status) = status {

                            if status.success() {
                                return HookStatus::Success
                            }
                            else {
                                return HookStatus::Error
                            }
                        }
                    }
                    else {
                        return HookStatus::Error
                    }
                }
            }
        }
        HookStatus::Nothing
    }

    fn hook_exist(&self, hook_type: HookType) -> bool {
        match self.hooks {
            Some(ref hooks) => {
                match hook_type {
                    HookType::Pre => {
                        if let Some(_) = hooks.pre_load {
                            true
                        }
                        else {
                            false
                        }
                    },
                    HookType::Post => {
                        if let Some(_) = hooks.post_load {
                            true
                        }
                        else {
                            false
                        }
                    }
                }
            },
            None => false
        }
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

    let mut items_src_to_dst : HashMap<PathBuf, PathBuf> = HashMap::new();
    let mut size_of_snap = 0;

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

                    if let Ok(size) = fs::copy(&item, &saved_item_path) {
                        println!("[\x1b[1;92m+\x1b[0m] Saved {} ({module_name})", item.display());
                        items_src_to_dst.insert(item, saved_item_path);
                        size_of_snap += size;
                    }
                    else {
                        println!("[\x1b[1;91m-\x1b[0m] Failed to save {}, skipping ({module_name})", item.display());
                    }
                } 
            }
        }
    }

    let snap_meta_data = SnapMetaData::new(items_src_to_dst, snap.hooks, size_of_snap);
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

    let snap = SnapMetaData::from(&snap_config_path);

    let mut failed = 0;
    let mut total = 0;
    
    match snap {
        Some(ref snap_meta) => {

            if snap_meta.hook_exist(HookType::Pre) {
                println!("[\x1b[1;92m+\x1b[0m] Executing pre hook");
                let status = snap_meta.run_hook(HookType::Pre);
                match status {
                    HookStatus::Success => println!("[\x1b[1;92m+\x1b[0m] Pre hook executed successfully"),
                    HookStatus::Error => println!("[\x1b[1;91m-\x1b[0m] Pre hook failed to execute"),
                    HookStatus::Nothing => println!("[\x1b[1;91m-\x1b[0m] Pre hook is empty")
                }
            }

            for (src_item, dst_item) in &snap_meta.items {
                total += 1;
                if let Err(_) = fs::copy(&dst_item, &src_item) {
                    println!("[\x1b[1;91m-\x1b[0m] Failed to transfer item {}", dst_item.display());
                    failed += 1;
                    continue;
                }
                println!("[\x1b[1;92m+\x1b[0m] Transferred {}", dst_item.display());
            } 

            if snap_meta.hook_exist(HookType::Post) {
                println!("[\x1b[1;92m+\x1b[0m] Executing post hook");
                let status = snap_meta.run_hook(HookType::Post);

                match status {
                    HookStatus::Success => println!("[\x1b[1;92m+\x1b[0m] Post hook executed successfully"),
                    HookStatus::Error => println!("[\x1b[1;91m-\x1b[0m] Post hook failed to execute"),
                    HookStatus::Nothing => println!("[\x1b[1;91m-\x1b[0m] Post hook is empty")
                }
            }
        },

        None => {
            eprintln!("[\x1b[1;91m-\x1b[0m] Failed to read {snap_name}'s metadata");
            return
        }
    };


    println!("[\x1b[1;92m+\x1b[0m] Transfer complete");
    println!("Transferred {}/{total}", total-failed);
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
        let snap_dir = Path::new(&snaps_dir).join(&snap).join("snap.json");
        if let Some(snap_meta) = SnapMetaData::from(&snap_dir) {
            println!("{snap}: {}kb", snap_meta.size/100);
        }
    }
}
