mod cli;
mod client;
mod github;
mod error;
mod types;
mod web;

use clap::Parser;
use log::warn;
use chrono::Local;
use std::io::Write;
use std::net::Ipv4Addr;
use tokio::runtime::Runtime;
use shared::dirs::eka_dirs;
use crate::error::Result;

const LOG_TARGET: &str = "eka-ci::server::main";

#[tokio::main]
async fn main() -> Result<()> {
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

    let socket = args.socket.unwrap_or_else(||
        eka_dirs().get_runtime_file("ekaci.socket")
          .expect("failed to find xdg_runtime_dir after socket not set")
          .to_str()
          .expect("failed to make socket path into string")
          .to_string()
    );

    let rt = Runtime::new()?;

    rt.spawn(async { client::listen_for_client(socket) });

    if let Err(e) = github::register_app().await {
        warn!(target: &LOG_TARGET, "Failed to register as github app: {:?}", e);
    }

    let addr = args.addr.parse::<Ipv4Addr>().expect("Invalid addr");
    web::serve_web(addr, args.port).await;

    Ok(())
}
