use std::path::PathBuf;

use clap::{Parser, Args};

#[derive(Parser)]
#[command(name="Snapsr")]
#[command(version="1.0")]
struct Cli {
    #[command(flatten)]
    args: Arg,

    #[arg(short, long, value_name="SNAP_FILE", help="Sets what Snap config file to use")]
    file: Option<PathBuf>
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

mod snaps;

fn main() {
    let cli = Cli::parse();
    
    if let Some(snap) = cli.args.snap {
        snaps::take_snap(snap, cli.file);
    }
    else if let Some(snap) = cli.args.transfer {
        snaps::transfer_snap(snap);
    }
    else if let Some(snap) = cli.args.delete {
        snaps::delete_snap(snap);
    }
    else if let Some((old_name, new_name)) = cli.args.rename {
        snaps::rename_snap(old_name.as_str(), new_name.as_str());
    }
    else if cli.args.list {
        snaps::list_snaps();
    }
    else if cli.args.clean {
        snaps::clean_snaps();
    }
}
