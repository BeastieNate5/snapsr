use std::{fs::{self, File}, io::Write, path::PathBuf, process};

use clap::{Args, Parser};

#[derive(Parser)]
#[command(name="Snapsr")]
#[command(version="1.0")]
#[command(about="Snaps", long_about = None)]
struct Cli {
    #[command(flatten)]
    args: Arg,

    #[arg(short, long, value_name="SNAP_FILE", help="Sets what Snap config file to use")]
    file: Option<PathBuf>,

    #[arg(long, value_name="PRE_HOOK", help="Pre hook when snapping")]
    pre: Option<String>,

    #[arg(long, value_name="POST_HOOK", help="Post hook when snapping")]
    post: Option<String>
}

#[derive(Args)]
#[group(required = true, multiple=false)]
struct Arg {
    #[arg(short, long, value_name="SNAP_NAME", help="Saves a Snap")]
    snap: Option<String>,

    #[arg(short, long, value_name="SNAP_NAME", help="???")]
    transfer: Option<String>,

    #[arg(short, long, value_name="SNAP_NAME", help="Deletes a Snap")]
    delete: Option<String>,

    #[arg(short, long, help="Displays all saved Snaps")]
    list: bool,

    #[arg(short, long, help="Cleans out unuseable snaps")]
    clean: bool,

    #[arg(short, long, help="Rename a snap, ex. 'old_name:new_name'", value_parser = parse_rename_args)]
    rename: Option<(String, String)>
}

fn parse_rename_args(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();

    if parts.len() != 2 {
        return Err("Expected format 'old_name:new_name'".into());
    }

    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn setup_env() {
    let base_snaps_dir = PathBuf::from(std::env::var("HOME")
        .expect("Failed to read $HOME env variable. Please set $HOME env variable"))
        .join(".config/snapsr");

    let snaps_dir = base_snaps_dir.join("snaps");
    let templates_dir = base_snaps_dir.join("templates");

    for dir in [&base_snaps_dir, &snaps_dir, &templates_dir]  {
        fs::create_dir_all(dir).unwrap_or_else(|err| {
            println!("[\x1b[1;91m-\x1b[0m] Failed to create config directory {} ({err})", dir.display());
            process::exit(1);
        });
    }


    let snap_config_path = base_snaps_dir.join("config.toml");
    if !snap_config_path.exists() {
        File::create(snap_config_path).unwrap_or_else(|err| {
            println!("[\x1b[1;91m-\x1b[0m] Failed to create snap config file ({err})");
            process::exit(1);
        });
    }
    
    let snap_log_path = base_snaps_dir.join("snaplog.json");
    if !snap_log_path.exists() {
        let mut log_file = File::create(snap_log_path).unwrap_or_else(|err| {
            println!("[\x1b[1;91m-\x1b[0m] Failed to create snap log file ({err})");
            process::exit(1);
        });

        log_file.write(b"{}").unwrap_or_else(|err| {
            println!("[\x1b[1;91m-\x1b[0m] Failed to initialize snap log file ({err})");
            process::exit(1);
        });
    }
}

mod snaps;
mod logger;
mod ui;

fn main() {
    let cli = Cli::parse();
    
    if let Some(snap) = cli.args.snap {
        setup_env();
        snaps::cmd_snap(snap, cli.file, cli.pre, cli.post);
    }
    else if let Some(snap) = cli.args.transfer {
        setup_env();
        snaps::cmd_transfer_snap(snap);
    }
    else if let Some(snap) = cli.args.delete {
        setup_env();
        snaps::cmd_delete_snap(snap);
    }
    else if let Some((old_name, new_name)) = cli.args.rename {
        setup_env();
        snaps::cmd_rename_snap(old_name.as_str(), new_name.as_str());
    }
    else if cli.args.list {
        setup_env();
        snaps::cmd_list_snaps();
    }
    else if cli.args.clean {
        setup_env();
        snaps::cmd_clean_snaps();
    }
}
