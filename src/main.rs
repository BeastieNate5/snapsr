use clap::{Parser, Args};

#[derive(Parser)]
#[command(name="Snapsr")]
#[command(version="1.0")]
struct Cli {
    #[command(flatten)]
    args: Arg,

    #[arg(short, long, value_name="SNAP_FILE", help="Sets what Snap config file to use")]
    file: Option<String>
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
    list: bool
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
    else if cli.args.list {
        snaps::list_snaps();
    }
}
