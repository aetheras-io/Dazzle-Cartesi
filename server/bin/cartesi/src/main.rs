mod mono;
use clap::Parser;

#[derive(Debug, Parser)]
enum Commands {
    /// Start the Monolithic API
    #[clap(name = "mono")]
    Mono,

    #[clap(name = "version")]
    /// Prints binary build information
    Version,
}

#[derive(Debug, Parser)]
#[clap(name = "Dinder", about = "Dinder Cartesi Dapp")]
struct Dinder {
    #[clap(subcommand)]
    command: Commands,
}

fn main() {
    let opts = Dinder::parse();

    use Commands::*;
    match opts.command {
        Mono => mono::run(),
        Version => {
            print_version();
        }
    }
}

fn print_version() {
    println!(
        "Package: {:?}\nVersion: {:?}\nAuthor: {:?}\nDescription: {:?}\nSupport: {:?}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS"),
        env!("CARGO_PKG_DESCRIPTION"),
        env!("CARGO_PKG_REPOSITORY")
    );
}
