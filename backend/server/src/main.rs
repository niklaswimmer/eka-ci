mod cli;
use clap::Parser;
use log::info;
use warp::Filter;
use chrono::Local;
use std::io::Write;

const LOG_TARGET: &str = "eka-ci::server::main";

#[tokio::main]
async fn main() {
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

    let about = warp::path("about")
    .map(|| format!("About Page"));
    let root = warp::path::end()
    .map(|| format!("Welcome to Eka-CI"));

    let routes = warp::get().and(about.or(root));

    info!(target: LOG_TARGET, "Serving Eka-CI on port: {}", args.port);

    warp::serve(routes)
    .run(([127, 0, 0, 1], args.port))
    .await
}
