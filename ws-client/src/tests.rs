#![cfg(test)]

use crate::v2::{
	error::{JsonRpcErrorCode, JsonRpcErrorObjectAlloc},
	params::JsonRpcParams,
};
use crate::{
	traits::{Client, SubscriptionClient},
	Error, Subscription, WsClientBuilder,
};
use jsonrpsee_test_utils::helpers::*;
use jsonrpsee_test_utils::types::{Id, WebSocketTestServer};
use serde_json::Value as JsonValue;

#[tokio::test]
async fn method_call_works() {
	let result = run_request_with_response(ok_response("hello".into(), Id::Num(0))).await.unwrap();
	assert_eq!(JsonValue::String("hello".into()), result);
}

#[tokio::test]
async fn notif_works() {
	// this empty string shouldn't be read because the server shouldn't respond to notifications.
	let server = WebSocketTestServer::with_hardcoded_response("127.0.0.1:0".parse().unwrap(), String::new()).await;
	let uri = to_ws_uri_string(server.local_addr());
	let client = WsClientBuilder::default().build(&uri).await.unwrap();
	assert!(client.notification("notif", JsonRpcParams::NoParams).await.is_ok());
}

#[tokio::test]
async fn response_with_wrong_id() {
	let err = run_request_with_response(ok_response("hello".into(), Id::Num(99))).await.unwrap_err();
	assert!(matches!(err, Error::RestartNeeded(_)));
}

#[tokio::test]
async fn response_method_not_found() {
	let err = run_request_with_response(method_not_found(Id::Num(0))).await.unwrap_err();
	assert_error_response(err, JsonRpcErrorCode::MethodNotFound.into());
}

#[tokio::test]
async fn parse_error_works() {
	let err = run_request_with_response(parse_error(Id::Num(0))).await.unwrap_err();
	assert_error_response(err, JsonRpcErrorCode::ParseError.into());
}

#[tokio::test]
async fn invalid_request_works() {
	let err = run_request_with_response(invalid_request(Id::Num(0_u64))).await.unwrap_err();
	assert_error_response(err, JsonRpcErrorCode::InvalidRequest.into());
}

#[tokio::test]
async fn invalid_params_works() {
	let err = run_request_with_response(invalid_params(Id::Num(0_u64))).await.unwrap_err();
	assert_error_response(err, JsonRpcErrorCode::InvalidParams.into());
}

#[tokio::test]
async fn internal_error_works() {
	let err = run_request_with_response(internal_error(Id::Num(0_u64))).await.unwrap_err();
	assert_error_response(err, JsonRpcErrorCode::InternalError.into());
}

#[tokio::test]
async fn subscription_works() {
	let server = WebSocketTestServer::with_hardcoded_subscription(
		"127.0.0.1:0".parse().unwrap(),
		server_subscription_id_response(Id::Num(0)),
		server_subscription_response(JsonValue::String("hello my friend".to_owned())),
	)
	.await;
	let uri = to_ws_uri_string(server.local_addr());
	let client = WsClientBuilder::default().build(&uri).await.unwrap();
	{
		let mut sub: Subscription<String> =
			client.subscribe("subscribe_hello", JsonRpcParams::NoParams, "unsubscribe_hello").await.unwrap();
		let response: String = sub.next().await.unwrap();
		assert_eq!("hello my friend".to_owned(), response);
	}
}

#[tokio::test]
async fn batch_request_works() {
	let batch_request = vec![
		("say_hello", JsonRpcParams::NoParams),
		("say_goodbye", JsonRpcParams::Array(vec![0_u64.into(), 1.into(), 2.into()])),
		("get_swag", JsonRpcParams::NoParams),
	];
	let server_response = r#"[{"jsonrpc":"2.0","result":"hello","id":0}, {"jsonrpc":"2.0","result":"goodbye","id":1}, {"jsonrpc":"2.0","result":"here's your swag","id":2}]"#.to_string();
	let response = run_batch_request_with_response(batch_request, server_response).await.unwrap();
	assert_eq!(response, vec!["hello".to_string(), "goodbye".to_string(), "here's your swag".to_string()]);
}

#[tokio::test]
async fn batch_request_out_of_order_response() {
	let batch_request = vec![
		("say_hello", JsonRpcParams::NoParams),
		("say_goodbye", JsonRpcParams::Array(vec![0_u64.into(), 1.into(), 2.into()])),
		("get_swag", JsonRpcParams::NoParams),
	];
	let server_response = r#"[{"jsonrpc":"2.0","result":"here's your swag","id":2}, {"jsonrpc":"2.0","result":"hello","id":0}, {"jsonrpc":"2.0","result":"goodbye","id":1}]"#.to_string();
	let response = run_batch_request_with_response(batch_request, server_response).await.unwrap();
	assert_eq!(response, vec!["hello".to_string(), "goodbye".to_string(), "here's your swag".to_string()]);
}

#[tokio::test]
async fn is_connected_works() {
	let server = WebSocketTestServer::with_hardcoded_response(
		"127.0.0.1:0".parse().unwrap(),
		ok_response(JsonValue::String("foo".into()), Id::Num(99_u64)),
	)
	.await;
	let uri = to_ws_uri_string(server.local_addr());
	let client = WsClientBuilder::default().build(&uri).await.unwrap();
	assert!(client.is_connected());
	client.request::<String>("say_hello", JsonRpcParams::NoParams).await.unwrap_err();
	// give the background thread some time to terminate.
	std::thread::sleep(std::time::Duration::from_millis(100));
	assert!(!client.is_connected())
}

async fn run_batch_request_with_response<'a>(
	batch: Vec<(&'a str, JsonRpcParams<'a>)>,
	response: String,
) -> Result<Vec<String>, Error> {
	let server = WebSocketTestServer::with_hardcoded_response("127.0.0.1:0".parse().unwrap(), response).await;
	let uri = to_ws_uri_string(server.local_addr());
	let client = WsClientBuilder::default().build(&uri).await.unwrap();
	client.batch_request(batch).await
}

async fn run_request_with_response(response: String) -> Result<JsonValue, Error> {
	let server = WebSocketTestServer::with_hardcoded_response("127.0.0.1:0".parse().unwrap(), response).await;
	let uri = format!("ws://{}", server.local_addr());
	let client = WsClientBuilder::default().build(&uri).await.unwrap();
	client.request("say_hello", JsonRpcParams::NoParams).await
}

fn assert_error_response(error: Error, code: JsonRpcErrorObjectAlloc) {
	match &error {
		Error::Request(e) => assert_eq!(e.error, code),
		e => panic!("Expected error: \"{}\", got: {:?}", error, e),
	};
}
