use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::process;
use std::path;
use std::io;
use std::path::Component;
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

#[derive(Serialize, Deserialize, Debug)]
struct SnapLog {
    #[serde(default)]
    snaps: HashMap<String, PathBuf>
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

impl SnapLog {
    fn fetch() -> Option<Self> {
        let snap_config_dir = get_snap_config_dir();
        let snap_config_path = PathBuf::from(snap_config_dir).join("snaplog.json");
        if let Ok(file_txt) = fs::read_to_string(snap_config_path) {
            if let Ok(log) = serde_json::from_str(&file_txt) {
                log
            }
            else {
                None
            }
        }
        else {
            None
        }
    }

    fn save(&self) -> Result<(), ()> {
        let snap_config_dir = get_snap_config_dir();
        let snap_config_path = PathBuf::from(snap_config_dir).join("snaplog.json");
        match serde_json::to_string(self) {
            Ok(json_txt) => {
                match fs::write(snap_config_path, json_txt) {
                    Ok(_) => Ok(()),
                    Err(_) => Err(())
                }
            },
            Err(_) => {
                Err(())
            }
        }
    }

    fn exist(&self, snap_name: &str) -> bool {
        self.snaps.contains_key(snap_name)
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

fn replace_component_in_path<P: AsRef<Path>>(path: P, name: &str, level: usize) -> Option<PathBuf> {
    let path = path.as_ref();
    let components: Vec<_> = path.components().collect();

    if components.len() < level{
        return None
    }

    let index_to_replace = components.len() - level;

    let new_compoents = components.iter()
        .enumerate()
        .map(|(i, component)| {
            if i == index_to_replace {
                Component::Normal(OsStr::new(name)) }
            else {
                *component
            }
        });

    let new_path = new_compoents.fold(PathBuf::new(), |mut new_path, cur_comp| {
        new_path.push(cur_comp);
        new_path
    });

    Some(new_path)
}

pub fn take_snap(snap_name: String, snap_config_path: Option<PathBuf>) {
    //let saved_snaps = get_all_snaps();
    match SnapLog::fetch() {
        Some(snaplog) => {
            if snaplog.exist(snap_name.as_str()) {
                let mut input = String::new();
                print!("Snap {snap_name} already exist. Do you wish to overwrite (y/N)? ");
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut input).unwrap();
                let input = input.trim();
                let input = input.to_lowercase();

                if input != "y" && input != "yes" {
                    println!("[\x1b[1;92m+\x1b[0m] aborting");
                    return;
                }
            }
        },
        None => {
            println!("[\x1b[1;91m-\x1b[0m] Failed to read snap log");
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
    if let Ok(_) = snap_meta_data.save(&snap_dir.join("snap.json")) {
        if let Some(mut snaplog) = SnapLog::fetch() {
            snaplog.snaps.insert(snap_name, snap_dir);
            if let Ok(_) = snaplog.save() {
                println!("[\x1b[1;92m+\x1b[0m] Sucessfully saved Snap");
            }
            else {
                println!("[\x1b[1;91m-\x1b[0m] Failed to save snap log, this snap will be unusable");
            }
        }
        else {
            println!("[\x1b[1;91m-\x1b[0m] Failed to read snap log, this snap will be unusable");
        }
    }
    else {
        println!("[\x1b[1;91m-\x1b[0m] Failed to save snap meta data, this snap will be unusable");
    }
}

pub fn transfer_snap(snap_name: String) {
    match SnapLog::fetch() {
        Some(snaplog) => {
            if !snaplog.exist(snap_name.as_str()) {
                println!("[\x1b[1;91m-\x1b[0m] Snap {snap_name} does not exist");
                return;
            }
        },
        None => {
            println!("[\x1b[1;91m-\x1b[0m] Failed to read snap log");
            return
        }
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
    if let Some(ref mut snaplog) = SnapLog::fetch() {
        if !snaplog.exist(snap.as_str()) {
            eprintln!("[\x1b[1;91m-\x1b[0m] Snap {snap} does not exist");
            return
        }

        if let Some(snap_dir) = snaplog.snaps.remove(&snap) {
            if let Ok(_) = fs::remove_dir_all(snap_dir) {
                println!("[\x1b[1;92m+\x1b[0m] Deleted {snap} snap");
            }
            else {
                eprintln!("[\x1b[1;91m-\x1b[0m] Failed to remove snap directory");
            }
        }
        else {
            eprintln!("[\x1b[1;91m-\x1b[0m] Failed to remove snap from snap log");
        }
    }

    else {
        eprintln!("[\x1b[1;91m-\x1b[0m] Failed to read snap log")
    }
}

pub fn rename_snap(old_name: &str, new_name: &str) {
    let mut snaplog = SnapLog::fetch().unwrap_or_else(|| {
        println!("[\x1b[1;91m-\x1b[0m] Failed to read snap log");
        process::exit(1);
    });

    
    if !snaplog.snaps.contains_key(old_name) {
        eprintln!("[\x1b[1;91m-\x1b[0m] Snap {old_name} does not exist");
        process::exit(1);
    }

    let snap_path = snaplog.snaps.get(old_name).unwrap_or_else(|| {
        eprintln!("[\x1b[1;91m-\x1b[0m] Failed to get snap from snap log");
        process::exit(1);
    });

    let new_snap_path = replace_component_in_path(snap_path, new_name, 1).unwrap_or_else(|| {
        eprintln!("[\x1b[1;91m-\x1b[0m] Failed");
        process::exit(1);
    });

    fs::rename(snap_path, &new_snap_path).unwrap_or_else(|_| {
        eprintln!("[\x1b[1;91m-\x1b[0m] Failed to rename snap directory");
        process::exit(1);
    });

    let mut snap_meta = SnapMetaData::from(&new_snap_path.join("snap.json")).unwrap_or_else(|| {
        eprintln!("[\x1b[1;91m-\x1b[0m] Failed to rename snap directory. DO NOT RUN -c, --clean\nRun the following command to restore snap 'mv {} {}'", new_snap_path.display(), snap_path.display());
        process::exit(1);
    });

    snap_meta.items = snap_meta.items
        .into_iter()
        .map(|item| {
            (item.0, replace_component_in_path(item.1, new_name, 3).unwrap())
        })
        .collect();

    snap_meta.save(&new_snap_path.join("snap.json")).unwrap_or_else(|_| {
        eprintln!("[\x1b[1;91m-\x1b[0m] Failed to save snap meta data. DO NOT RUN -c, --clean\nRun the following command to restore snap 'mv {} {}'", new_snap_path.display(), snap_path.display());
        process::exit(1);
    });
    
    snaplog.snaps.remove(old_name);
    snaplog.snaps.insert(new_name.into(), new_snap_path);

    snaplog.save().unwrap_or_else(|_| {
        eprintln!("[\x1b[1;91m-\x1b[0m] Failed to update snaplog no changes made");
        process::exit(1);
    });

    
    println!("[\x1b[1;92m+\x1b[0m] Renamed snap to {new_name}");
}

pub fn list_snaps() {
    match SnapLog::fetch() {
        Some(snaplog) => {
            for (snap, snap_dir) in &snaplog.snaps {
                match SnapMetaData::from(&snap_dir.join("snap.json")) {
                    Some(snap_meta) => {
                        println!("{snap}: {}kb", snap_meta.size/100);
                    },
                    None => {}
                }
            }
        },
        None => println!("[\x1b[1;91m-\x1b[0m] Failed to read snap log")
    }
}

pub fn clean_snaps() {
    let mut amount: u8 = 0;
    let snaps_dir = get_snaps_dir();
    match SnapLog::fetch() {

        Some(snap_log) => {
            match fs::read_dir(snaps_dir) {
                Ok(dir_iter) => {

                    for entry_result in dir_iter {
                        match entry_result {
                            Ok(entry) => {
                                let path = entry.path(); 

                                if path.is_dir() {
                                    let snap_name_op = path.file_name();

                                    if let Some(snap_name) = snap_name_op {
                                        let snap_name = snap_name.to_string_lossy().into_owned();

                                        if !snap_log.snaps.contains_key(&snap_name) {
                                            if let Err(err) = fs::remove_dir_all(&path) {
                                                eprintln!("[\x1b[1;91m-\x1b[0m] Failed to read snap log ({err})");
                                            }
                                            else {
                                                println!("[\x1b[1;92m+\x1b[0m] Cleaned out {}", path.display());
                                                amount += 1;
                                            }
                                        }
                                    }
                                }
                            },
                            Err(err) => {
                                println!("[\x1b[1;92m+\x1b[0m] Failed to read dir entry ({err})");
                            }
                        }
                    }
                },

                Err(err) => {
                    eprintln!("[\x1b[1;91m-\x1b[0m] Failed to read snap log ({err})");
                }
            }
        },

        None => {
            eprintln!("[\x1b[1;91m-\x1b[0m] Failed to read snap log");
        }
    }
    println!("[\x1b[1;92m+\x1b[0m] Cleaned out {amount} snap(s)");
}

#[cfg(test)]
mod test {
    use super::*;


    #[test]
    fn test_repalce_component() {
        assert_eq!(replace_component_in_path(PathBuf::from("/home/bob/.config/hypr"), "waybar", 1), Some(PathBuf::from("/home/bob/.config/waybar")));
        assert_eq!(replace_component_in_path(PathBuf::from("/home/bob/.config/hypr/scripts"), "waybar", 2), Some(PathBuf::from("/home/bob/.config/waybar/scripts")));
    }
}
