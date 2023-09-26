use crate::network::CanonicalIndicesNetwork;
use discv5::TalkRequest;
use portalnet::types::messages::Message;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{error, warn, Instrument};

pub struct CanonicalIndicesEvents {
    pub network: Arc<CanonicalIndicesNetwork>,
    pub event_rx: UnboundedReceiver<TalkRequest>,
}

impl CanonicalIndicesEvents {
    pub async fn start(mut self) {
        loop {
            tokio::select! {
                Some(talk_request) = self.event_rx.recv() => {
                    self.handle_canonical_indices_talk_request(talk_request);
                } else => {
                    error!("CanonicalIndices event channel closed, shutting down");
                    break;
                }
            }
        }
    }

    /// Handle CanonicalIndices network TalkRequest event
    fn handle_canonical_indices_talk_request(&self, talk_request: TalkRequest) {
        let network = Arc::clone(&self.network);
        let talk_request_id = talk_request.id().clone();
        tokio::spawn(async move {
            let reply = match network
                .overlay
                .process_one_request(&talk_request)
                .instrument(tracing::info_span!("canonical_indices_network", req = %talk_request_id))
                .await
            {
                Ok(response) => Message::from(response).into(),
                Err(error) => {
                    error!(
                        error = %error,
                        request.discv5.id = %talk_request_id,
                        "Error processing portal canonical indices request, responding with empty TALKRESP"
                    );
                    // Return an empty TALKRESP if there was an error executing the request
                    "".into()
                }
            };
            if let Err(error) = talk_request.respond(reply) {
                warn!(error = %error, request.discv5.id = %talk_request_id, "Error responding to TALKREQ");
            }
        });
    }
}
