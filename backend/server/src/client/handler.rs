use log::debug;
use std::os::unix::net::UnixStream;
use std::net::Shutdown;
use shared::types::{ClientRequest, ClientResponse};
use crate::error::Result;
use shared::types::EvalRequest;
use tokio::sync::mpsc;

pub struct ClientHandler {
    stream: UnixStream,
    eval_sender: mpsc::UnboundedSender<EvalRequest>,
}

impl ClientHandler {
    pub fn new(stream: UnixStream, eval_sender: mpsc::UnboundedSender<EvalRequest>) -> Self {
        ClientHandler { stream, eval_sender }
    }

    pub fn handle_client(&mut self) -> Result<()> {
        use shared::types as t;
        use std::io::{Write, Read};

        debug!("Got unix socket client: {:?}", &self.stream);

        let mut request_message: String = String::new();
        self.stream.read_to_string(&mut request_message)?;
        debug!("Got raw message from client: \"{:?}\"", &request_message);
        let message: t::ClientRequest = serde_json::from_str(&request_message)?;
        debug!("Got deserialized message from client: {:?}", &message);

        let response = self.handle_request(message);
        let response_message = serde_json::to_string(&response)?;

        self.stream.write_all(response_message.as_bytes())?;
        self.stream.flush()?;

        debug!("Shutting down socket for client: {:?}", &self.stream);
        self.stream.shutdown(Shutdown::Both)?;
        Ok(())
    }

    fn handle_request(&self, request: ClientRequest) -> ClientResponse {
        use shared::types::ClientRequest as req;
        use shared::types::ClientResponse as resp;
        use shared::types as t;

        match request {
            req::Info => resp::Info (t::InfoResponse {
                status: t::ServerStatus::Active,
                version : "0.1.0".to_string(),
            }),
            req::Eval(eval_args) => {
                // Register eval request, return eval ID
                resp::Eval(1)
            },
        }
    }
}
