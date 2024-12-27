use tokio::sync::mpsc::UnboundedReceiver;
use shared::types::EvalRequest;
use log::debug;

pub struct Evaluator {
    recv: UnboundedReceiver<EvalRequest>,
}

impl Evaluator {
    pub fn new(recv: UnboundedReceiver<EvalRequest>) -> Self {
        Evaluator { recv }
    }

    /// As evaluation is quite expensive in most cases,
    /// it will be limited to just one evaluation at a time.
    // TODO: Allow for remote machines to run evals?
    pub async fn process_eval_requests(&mut self) {
        while let Some(req) = self.recv.recv().await {
            debug!("Received eval request: {:?}", req);
        }
    }
}
