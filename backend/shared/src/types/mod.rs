use serde::{Deserialize, Serialize};
use serde;
use clap::Parser;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ClientRequest {
  Info,
  Eval(EvalRequest),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerStatus {
    Active,
    Degraded,
    Dead,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InfoResponse {
    pub status: ServerStatus,
    pub version: String,
}


#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ClientResponse {
  Info(InfoResponse),
  Eval(u32),
}

#[derive(Parser, Serialize, Deserialize, Debug, Clone)]
pub struct EvalRequest {
    pub repo: String,
    #[arg(short, long)]
    pub branch: Option<String>,
    #[arg(long)]
    pub rev: Option<String>,
}

