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

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    match cli.command {
        Commands::Watch(args) => {
            if let Err(e) = runtime.block_on(watch::watch(args)) {
                eprintln!("Error: {:?}", e);
            }
        }
    }
}
