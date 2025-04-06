use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
};

use anyhow::Result;
use clap::{
    builder::{TypedValueParser, ValueParser},
    Args, Parser,
};
use figment::{
    providers::{Env, Format, Serialized, Toml},
    value::Value,
    Figment,
};
use serde::{Deserialize, Serialize};

#[derive(Parser, Deserialize, Serialize, Debug)]
#[command(version, about, long_about = None)]
pub struct ConfigStructure {
    /// Configuration for the web service.
    #[command(flatten)]
    web: ConfigStructureWeb,

    #[command(flatten)]
    unix: ConfigStructureUnix,

    #[serde(skip)]
    config_file: Option<PathBuf>,

    #[arg(skip)]
    file_only: Option<String>,

    #[arg(short, long)]
    #[serde(skip)]
    cli_only: Option<String>,
}

#[derive(Args, Deserialize, Serialize, Debug)]
struct ConfigStructureWeb {
    /// IPv4 address to bind http traffic.
    #[arg(default_value = "127.0.0.1")]
    #[arg(short, long)]
    addr: Option<Ipv4Addr>,

    /// Port for server to host http traffic.
    #[arg(default_value = "3030")]
    #[arg(short, long)]
    port: Option<u16>,

    /// Path for the frontend bundle. Frontend will be disabled if not provided.
    #[arg(short, long)]
    bundle: Option<PathBuf>,
}

#[derive(Args, Deserialize, Serialize, Debug)]
struct ConfigStructureUnix {
    /// Socket for ekaci client. Defaults to $XDG_RUNTIME_DIR/ekaci.
    #[arg(short, long)]
    socket: Option<PathBuf>,
}

#[derive(Deserialize, Debug)]
struct ConfigEnv {
    #[serde(rename = "eka_ci_config_file")]
    config_file: Option<PathBuf>,
    env_only: Option<String>,
}

impl ConfigStructure {
    pub fn from_env() -> Result<Self> {
        let args = ConfigStructure::parse();
        let env = envy::from_env::<ConfigEnv>()?;
        let dirs = xdg::BaseDirectories::with_prefix("ekaci")?;

        let config_path = args.config_file.or(env.config_file)
            .unwrap_or_else(|| dirs.get_config_file("ekaci.toml"));

        let mut config: ConfigStructure = Figment::new()
            .merge(Serialized::defaults(ConfigStructure::parse()))
            .merge(Env::prefixed("EKA_CI_").split("__"))
            .merge(Toml::file(&config_path)).extract()?;

        config.cli_only = args.cli_only;

        dbg!(&config);

        println!("{config:?}");

        Ok(config)
    }
}
