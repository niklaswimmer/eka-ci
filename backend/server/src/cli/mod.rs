use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Args {

    #[arg(short,long)]
    pub port: u16,

    #[clap(default_value = "127.0.0.1")]
    #[arg(short,long)]
    pub addr: String,
}
