mod watch;

use clap::Args;
use clap::Parser;
use clap::Subcommand;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Watch(Watch),
}

#[derive(Args, Debug)]
struct Watch {
    #[arg(default_value = ".")]
    path: String,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Watch(args) => {
            if let Err(e) = watch::watch(args) {
                eprintln!("Error: {:?}", e);
            }
        }
    }
}
