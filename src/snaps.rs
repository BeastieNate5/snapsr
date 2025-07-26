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

use crate::logger;
use crate::logger::log;
use crate::logger::LogLevel;

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
    #[serde(default)]
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
                let txt = Self::parse_for_template(txt);
                match toml::from_str(&txt) {
                    Ok(config) => Some(config),
                    Err(_) => None
                }
            }

            Err(_) => None
        }
    }

    fn parse_for_template(txt: String) -> String {
        txt.lines()
           .map(|line| {
                let line = line.trim();
                
                if line.starts_with("template ") {
                    let mut line_splitted = line.split_whitespace();
                    let template_file_opt = line_splitted.nth(1);
                    
                    if let Some(template_file) = template_file_opt {
                        let template_file_path = PathBuf::from(get_snap_config_dir()).join("templates").join(template_file);

                        match fs::read_to_string(&template_file_path) {
                            Ok(template_txt) => {
                                template_txt
                            },
                            Err(err) => {
                                log(logger::LogLevel::Error, format!("Failed to read template {} ({err})", template_file_path.display()).as_str());
                                "".to_string()
                            }
                        }
                    }
                    else {
                        "".to_string()
                    }
                }
                else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
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

impl Hooks {
    fn new(pre_hook: Option<String>, post_hook: Option<String>) -> Self {
        return Self {
            pre_load: pre_hook,
            post_load: post_hook
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

pub fn cmd_snap(snap_name: String, snap_config_path: Option<PathBuf>, pre_hook: Option<String>, post_hook: Option<String>) {
    match SnapLog::fetch() {
        Some(snaplog) => {
            if snaplog.exist(snap_name.as_str()) {
                let mut input = String::new();
                log(logger::LogLevel::Info, format!("Snap {snap_name} already exist. Do you wish to overwrite (y/N)? ").as_str());
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut input).unwrap();
                let input = input.trim();
                let input = input.to_lowercase();

                if input != "y" && input != "yes" {
                    log(logger::LogLevel::Info, "aborting");
                    return;
                }
            }
        },
        None => {
            log(logger::LogLevel::Error, "Failed to read snap log");
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
            log(logger::LogLevel::Error, "Failed to read snap config");
            return
        }
    };
    

    let snap_dir = get_snaps_dir();
    let snap_dir = path::Path::new(&snap_dir).join(&snap_name);

    if let Err(_) = fs::create_dir_all(&snap_dir) {
        log(logger::LogLevel::Error, "Failed to create snap directory");
        return
    }

    let mut items_src_to_dst : HashMap<PathBuf, PathBuf> = HashMap::new();
    let mut size_of_snap = 0;

    for (module_name, module) in &snap.modules {
        let module_dir = snap_dir.join(&module_name);

        if let Err(_) = fs::create_dir_all(&module_dir) {
            log(logger::LogLevel::Error, format!("Failed to create module directory for {module_name}").as_str());
            continue;
        }

        let items = module.get_item_paths();

        log(logger::LogLevel::Info, format!("{module_name}: {} items", items.len()).as_str());

        for item in items {

            if let (Some(parent), Some(file_child)) = (item.parent(), item.file_name()) {
                if let Some(grandparent) = parent.file_name() {
                    let grandparent_key = grandparent.to_string_lossy();
                    let file_child_key = file_child.to_string_lossy();

                    let file_key = grandparent_key.to_string() + "_" + &file_child_key;
                    let saved_item_path = module_dir.join(file_key);

                    if let Ok(size) = fs::copy(&item, &saved_item_path) {
                        log(logger::LogLevel::Success, format!("Snapped {} ({module_name})", item.display()).as_str());
                        items_src_to_dst.insert(item, saved_item_path);
                        size_of_snap += size;
                    }
                    else {
                        log(logger::LogLevel::Error, format!("Failed to snap {}, skipping ({module_name})", item.display()).as_str());
                    }
                } 
            }
        }
    }


    let hooks : Option<Hooks>;
    if pre_hook.is_some() || post_hook.is_some() {
        hooks = Some(Hooks::new(pre_hook, post_hook))
    }
    else {
        hooks = snap.hooks;
    }


    let snap_meta_data = SnapMetaData::new(items_src_to_dst, hooks, size_of_snap);
    if let Ok(_) = snap_meta_data.save(&snap_dir.join("snap.json")) {
        if let Some(mut snaplog) = SnapLog::fetch() {
            snaplog.snaps.insert(snap_name, snap_dir);
            if let Ok(_) = snaplog.save() {
                println!("[\x1b[1;92m+\x1b[0m] Sucessfully saved Snap");
                log(logger::LogLevel::Success, "Sucessfully saved Snap");
            }
            else {
                log(logger::LogLevel::Error, "Failed to save snap log, this snap will be unusable");
            }
        }
        else {
            log(logger::LogLevel::Error, "Failed to save snap log, this snap will be unusable");
        }
    }
    else {
        log(logger::LogLevel::Error, "Failed to save snap log, this snap will be unusable");
    }
}

pub fn cmd_transfer_snap(snap_name: String) {
    match SnapLog::fetch() {
        Some(snaplog) => {
            if !snaplog.exist(snap_name.as_str()) {
                log(logger::LogLevel::Error, format!("Snap {snap_name} does not exist").as_str());
                return;
            }
        },
        None => {
            log(logger::LogLevel::Error, "Failed to read snap log");
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
                log(logger::LogLevel::Info, "Executing pre-hook");
                let status = snap_meta.run_hook(HookType::Pre);
                match status {
                    HookStatus::Success => log(logger::LogLevel::Success, "Pre-hook executed successfully"),
                    HookStatus::Error => log(logger::LogLevel::Error, "Pre-hook failed to execute"),
                    HookStatus::Nothing => log(logger::LogLevel::Warn, "Pre-hook is empty")
                }
            }

            for (src_item, dst_item) in &snap_meta.items {
                total += 1;
                if let Err(_) = fs::copy(&dst_item, &src_item) {
                    log(logger::LogLevel::Error, format!("Failed to transfer item {}", dst_item.display()).as_str());
                    failed += 1;
                    continue;
                }
                log(LogLevel::Success, format!("Transferred {}", dst_item.display()).as_str());
            } 

            if snap_meta.hook_exist(HookType::Post) {
                log(logger::LogLevel::Success, "Executing post-hook");
                let status = snap_meta.run_hook(HookType::Post);

                match status {
                    HookStatus::Success => log(logger::LogLevel::Success, "Post-hook executed successfully"),
                    HookStatus::Error => log(logger::LogLevel::Error, "Post-hook failed to execute"),
                    HookStatus::Nothing => log(logger::LogLevel::Warn, "Post-hook is empty")

                }
            }
        },

        None => {
            log(logger::LogLevel::Error, format!("Failed to read {snap_name}'s metadata").as_str());
            return
        }
    };

    log(logger::LogLevel::Success, "Transfer complete");
    log(logger::LogLevel::Info, format!("Transferred {}/{total}", total-failed).as_str());
}

pub fn cmd_delete_snap(snap: String) {
    let mut snaplog = SnapLog::fetch().unwrap_or_else(|| {
        log(logger::LogLevel::Error, "Failed to read snap log");
        process::exit(1)
    });

    if !snaplog.exist(&snap) {
        log(logger::LogLevel::Error, format!("Snap {snap} does not exist").as_str());
        process::exit(1);
    }

    let snap_dir = snaplog.snaps.remove(&snap).unwrap_or_else(|| {
        log(logger::LogLevel::Error, format!("Snap {snap} does not exist").as_str());
        process::exit(1);
    });

    fs::remove_dir_all(snap_dir).unwrap_or_else(|err| {
        log(logger::LogLevel::Error, format!("Failed to remove snap directory ({err})").as_str());
        process::exit(1);
    });

    snaplog.save().unwrap_or_else(|_| {
        log(logger::LogLevel::Error, "Failed to update snap log, please try again");
        process::exit(1);
    });

    log(logger::LogLevel::Success, format!("Deleted {snap}").as_str())
}

pub fn cmd_rename_snap(old_name: &str, new_name: &str) {
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

pub fn cmd_list_snaps() {
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

pub fn cmd_clean_snaps() {
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
