use log::{debug, info, warn};
use std::os::unix::net::UnixListener;
use std::path::Path;
use tokio::runtime::Runtime;
use crate::error::Result;
use crate::client::ClientHandler;
use shared::types::EvalRequest;
use tokio::sync::mpsc;

/// Ensure parent directories
/// Remove potential lingering socket file from previous runs
fn prepare_path(socket: &str) {
    let socket_path = Path::new(&socket);
    let parent = socket_path.parent().expect("Socket can not be under root");

    if !parent.exists() {
        info!("Creating directory: {:?}", &parent);
        let _ = std::fs::create_dir_all(parent);
    }

    // Not deleting the previous socket file results in a:
    // "Already in use" error
    if socket_path.exists() {
        debug!("Previous socket file {:?} found, attempting to remove", socket_path);
        std::fs::remove_file(socket_path)
            .expect("Failed to remove previous socket");
    }
}

pub struct ClientListener {
    socket_path: String,
    eval_sender: mpsc::UnboundedSender<EvalRequest>,
}

impl ClientListener {
    pub fn new(
        socket_path: String,
        eval_sender: mpsc::UnboundedSender<EvalRequest>
        ) -> Self {
        ClientListener { socket_path, eval_sender }
    }

    /// This endlessly loops for incoming connections
    pub fn listen_for_client(&self) -> Result <()> {
        prepare_path(&self.socket_path);

        info!("Attempting to listen on socket: {:?}", &self.socket_path);
        let listener = UnixListener::bind(self.socket_path.clone())?;
        // TODO: Share a single tokio runtime, might not be a concern with #[tokio::main]
        let rt = Runtime::new()?;

        for maybe_socket in listener.incoming() {
            match maybe_socket {
                Ok(s) =>  {
                    let new_sender = self.eval_sender.clone();
                    rt.spawn(async {
                        let mut client_handler = ClientHandler::new(s, new_sender);
                        client_handler.handle_client()
                    });
                },
                Err(e) => warn!("Failed to create socket connection: {:?}", e),
            }
        }
        Ok(())
    }
}


