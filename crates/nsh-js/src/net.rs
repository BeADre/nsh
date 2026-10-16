//! HTTP requests made by `fetch`, as handed to the platform's network stack.

pub use nsh_core::platform::{HttpRequest, HttpResponse};

/// Identifies a request between JS and the network stack.
pub type RequestId = u32;

/// Requests and cancellations made by JS since the last
/// [`crate::Host::take_network`].
#[derive(Debug, Default)]
pub struct NetworkQueue {
    /// New requests.
    pub requests: Vec<(RequestId, HttpRequest)>,
    /// Requests aborted by JS (`AbortController`).
    pub cancels: Vec<RequestId>,
}

impl NetworkQueue {
    /// True if there is nothing to do.
    pub fn is_empty(&self) -> bool {
        self.requests.is_empty() && self.cancels.is_empty()
    }
}
