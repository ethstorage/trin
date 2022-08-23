use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::rpc_params;
use rand::{thread_rng, Rng};
use std::time::Duration;
use trin_core::utils::bytes::hex_encode;

const SERVER_ADDR: &str = "193.167.100.100:9041";
const CLIENT_ADDR: &str = "193.167.0.100:9042";

/// Test suite for testing uTP protocol with network simulator
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    send_1000_bytes().await?;

    Ok(())
}

/// Send 1k bytes payload from client to server
async fn send_1000_bytes() -> anyhow::Result<()> {
    println!("Sending 1000 bytes uTP payload from client to server...");
    let client_url = format!("http://{}", CLIENT_ADDR);
    let client_rpc = HttpClientBuilder::default().build(client_url)?;
    let client_enr: String = client_rpc.request("local_enr", None).await.unwrap();

    let server_url = format!("http://{}", SERVER_ADDR);
    let server_rpc = HttpClientBuilder::default().build(server_url)?;
    let server_enr: String = server_rpc.request("local_enr", None).await.unwrap();

    let connection_id: u16 = thread_rng().gen();

    // Add client enr to allowed server uTP connections
    let params = rpc_params!(client_enr, connection_id);
    let response: String = server_rpc.request("prepare_to_recv", params).await.unwrap();
    assert_eq!(response, "true");

    // Send uTP payload from client to server
    let payload: Vec<u8> = vec![thread_rng().gen(); 100_000];

    let params = rpc_params!(server_enr, connection_id, payload.clone());
    let response: String = client_rpc
        .request("send_utp_payload", params)
        .await
        .unwrap();

    assert_eq!(response, "true");

    // Sleep to allow time for uTP transmission
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify received uTP payload
    let utp_payload: String = server_rpc.request("get_utp_payload", None).await.unwrap();
    let expected_payload = hex_encode(payload);

    assert_eq!(expected_payload, utp_payload);

    println!("Sent 100k bytes uTP payload from client to server: OK");

    Ok(())
}
