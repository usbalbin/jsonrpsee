use crate::v2::error::JsonRpcErrorAlloc;
use std::fmt;

/// Convenience type for displaying errors.
#[derive(Clone, Debug, PartialEq)]
pub struct Mismatch<T> {
	/// Expected value.
	pub expected: T,
	/// Actual value.
	pub got: T,
}

impl<T: fmt::Display> fmt::Display for Mismatch<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_fmt(format_args!("Expected: {}, Got: {}", self.expected, self.got))
	}
}

/// Invalid params.
#[derive(Debug)]
pub struct InvalidParams;

/// Error that occurs when a call failed.
#[derive(Debug, thiserror::Error)]
pub enum CallError {
	#[error("Invalid params in the RPC call")]
	/// Invalid params in the call.
	InvalidParams,
	#[error("RPC Call failed: {0}")]
	/// The call failed.
	Failed(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl From<InvalidParams> for CallError {
	fn from(_params: InvalidParams) -> Self {
		Self::InvalidParams
	}
}

/// Error type.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// Error that occurs when a call failed.
	#[error("Server call failed: {0}")]
	Call(CallError),
	/// Networking error or error on the low-level protocol layer.
	#[error("Networking or low-level protocol error: {0}")]
	Transport(#[source] Box<dyn std::error::Error + Send + Sync>),
	/// JSON-RPC request error.
	#[error("JSON-RPC request error: {0:?}")]
	Request(#[source] JsonRpcErrorAlloc),
	/// Frontend/backend channel error.
	#[error("Frontend/backend channel error: {0}")]
	Internal(#[source] futures_channel::mpsc::SendError),
	/// Invalid response,
	#[error("Invalid response: {0}")]
	InvalidResponse(Mismatch<String>),
	/// The background task has been terminated.
	#[error("The background task been terminated because: {0}; restart required")]
	RestartNeeded(String),
	/// Failed to parse the data.
	#[error("Parse error: {0}")]
	ParseError(#[source] serde_json::Error),
	/// Invalid subscription ID.
	#[error("Invalid subscription ID")]
	InvalidSubscriptionId,
	/// Invalid request ID.
	#[error("Invalid request ID")]
	InvalidRequestId,
	/// A request with the same request ID has already been registered.
	#[error("A request with the same request ID has already been registered")]
	DuplicateRequestId,
	/// Method was already registered.
	#[error("Method: {0} was already registered")]
	MethodAlreadyRegistered(String),
	/// Subscribe and unsubscribe method names are the same.
	#[error("Cannot use the same method name for subscribe and unsubscribe, used: {0}")]
	SubscriptionNameConflict(String),
	/// Request timeout
	#[error("Request timeout")]
	RequestTimeout,
	/// Configured max number of request slots exceeded.
	#[error("Configured max number of request slots exceeded")]
	MaxSlotsExceeded,
	/// Custom error.
	#[error("Custom error: {0}")]
	Custom(String),
}

/// Generic transport error.
#[derive(Debug, thiserror::Error)]
pub enum GenericTransportError<T: std::error::Error + Send + Sync> {
	/// Request was too large.
	#[error("The request was too big")]
	TooLarge,
	/// Concrete transport error.
	#[error("Transport error: {0}")]
	Inner(T),
}
