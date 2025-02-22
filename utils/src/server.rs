//! Shared helpers for JSON-RPC Servers.

use futures_channel::mpsc;
use futures_util::stream::StreamExt;
use jsonrpsee_types::v2::error::{JsonRpcError, JsonRpcErrorCode, JsonRpcErrorObject};
use jsonrpsee_types::v2::params::{RpcParams, TwoPointZero};
use jsonrpsee_types::v2::response::JsonRpcResponse;
use rustc_hash::FxHashMap;
use serde::Serialize;
use serde_json::value::RawValue;

/// Connection ID.
pub type ConnectionId = usize;
/// Sender.
pub type RpcSender<'a> = &'a mpsc::UnboundedSender<String>;
/// RPC ID.
pub type RpcId<'a> = Option<&'a RawValue>;
/// Method registered in the server.
pub type Method = Box<dyn Send + Sync + Fn(RpcId, RpcParams, RpcSender, ConnectionId) -> anyhow::Result<()>>;
/// Methods registered in the Server.
pub type Methods = FxHashMap<&'static str, Method>;

/// Helper for sending JSON-RPC responses to the client
pub fn send_response(id: RpcId, tx: RpcSender, result: impl Serialize) {
	let json = match serde_json::to_string(&JsonRpcResponse { jsonrpc: TwoPointZero, id, result }) {
		Ok(json) => json,
		Err(err) => {
			log::error!("Error serializing response: {:?}", err);

			return send_error(id, tx, JsonRpcErrorCode::InternalError.into());
		}
	};

	if let Err(err) = tx.unbounded_send(json) {
		log::error!("Error sending response to the client: {:?}", err)
	}
}

/// Helper for sending JSON-RPC errors to the client
pub fn send_error(id: RpcId, tx: RpcSender, error: JsonRpcErrorObject) {
	let json = match serde_json::to_string(&JsonRpcError { jsonrpc: TwoPointZero, error, id }) {
		Ok(json) => json,
		Err(err) => {
			log::error!("Error serializing error message: {:?}", err);

			return;
		}
	};

	if let Err(err) = tx.unbounded_send(json) {
		log::error!("Error sending response to the client: {:?}", err)
	}
}

/// Read all the results of all method calls in a batch request from the ['Stream']. Format the result into a single
/// `String` appropriately wrapped in `[`/`]`.
pub async fn collect_batch_response(rx: mpsc::UnboundedReceiver<String>) -> String {
	let mut buf = String::with_capacity(2048);
	buf.push('[');
	let mut buf = rx
		.fold(buf, |mut acc, response| async {
			acc = [acc, response].concat();
			acc.push(',');
			acc
		})
		.await;
	// Remove trailing comma
	buf.pop();
	buf.push(']');
	buf
}
