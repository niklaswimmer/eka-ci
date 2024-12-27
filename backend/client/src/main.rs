mod cli;
mod server_client;
mod error;

use chrono::Local;
use clap::Parser;
use cli::Commands;
use shared::types::ClientRequest;
use std::io::Write;
use crate::error::Result;

fn main() -> Result <()> {
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            writeln!(buf,
                "[{}] {:5} - {} - {}",
                Local::now().to_rfc3339(),
                record.level(),
                record.target(),
                record.args()
            )
        }).init();

    let args = cli::Args::parse();
    let client = server_client::Client::new(args.socket);

    match &args.command {
        Some(Commands::Info) => {
            client.send_request(ClientRequest::Info)?;
        },
        Some(Commands::Eval(ref eval_request)) => {
            client.send_request(ClientRequest::Eval(eval_request.clone()))?;
        },
        Some(Commands::Status) => {
        },
        None => {
        },
    }

    Ok(())
}
