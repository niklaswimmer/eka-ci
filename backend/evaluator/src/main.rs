mod cli;
use clap::Parser;

fn main() {
    cli::Args::parse();
}
