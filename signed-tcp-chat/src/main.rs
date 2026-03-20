mod crypto;
mod protocol;
mod server;
mod client;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "chat")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Server {
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        addr: String,
    },
    Client {
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        addr: String,
        #[arg(short, long)]
        username: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Server { addr } => server::start(&addr),
        Command::Client { addr, username } => client::connect(&addr, &username),
    }
}
