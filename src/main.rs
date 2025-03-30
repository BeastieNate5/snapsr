use clap::{Parser, Args};

#[derive(Parser)]
#[command(name="Snapsr")]
#[command(version="1.0")]
struct Cli {
    #[command(flatten)]
    args: Arg
}

#[derive(Args)]
#[group(required = true, multiple=false)]
struct Arg {
    #[arg(short, long, value_name="SNAP_NAME")]
    snap: Option<String>,

    #[arg(short, long, value_name="SNAP_NAME")]
    transfer: Option<String>,

    #[arg(short, long, value_name="SNAP_NAME")]
    delete: Option<String>,

    #[arg(short, long)]
    list: bool
}

mod snaps;

fn main() {
    let cli = Cli::parse();
    
    if let Some(snap) = cli.args.snap {
        snaps::take_snap(snap);
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
