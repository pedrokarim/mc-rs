//! RCON (Remote Console) server implementation.
//!
//! Implements the Source RCON protocol over TCP for remote server management.

use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// RCON packet types.
const PACKET_TYPE_COMMAND: i32 = 2;
const PACKET_TYPE_LOGIN: i32 = 3;
const PACKET_TYPE_RESPONSE: i32 = 0;
const PACKET_TYPE_LOGIN_SUCCESS: i32 = 2;

/// A command received via RCON, with a channel to send the response back.
pub struct RconCommand {
    pub command: String,
    pub response_tx: tokio::sync::oneshot::Sender<String>,
}

/// Start the RCON server on the given port.
pub fn start(port: u16, password: String, cmd_tx: mpsc::Sender<RconCommand>) {
    tokio::spawn(async move {
        let addr: SocketAddr = format!("0.0.0.0:{port}").parse().unwrap();
        let listener = match TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                warn!("Failed to bind RCON server on port {port}: {e}");
                return;
            }
        };
        info!("RCON server listening on port {port}");

        loop {
            match listener.accept().await {
                Ok((stream, peer)) => {
                    debug!("RCON connection from {peer}");
                    let pw = password.clone();
                    let tx = cmd_tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_rcon_connection(stream, &pw, &tx).await {
                            debug!("RCON connection {peer} closed: {e}");
                        }
                    });
                }
                Err(e) => {
                    warn!("RCON accept error: {e}");
                }
            }
        }
    });
}

async fn handle_rcon_connection(
    mut stream: tokio::net::TcpStream,
    password: &str,
    cmd_tx: &mpsc::Sender<RconCommand>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut authenticated = false;

    loop {
        // Read packet: i32_le(length) + i32_le(request_id) + i32_le(type) + payload + \0\0
        let length = match read_i32_le(&mut stream).await {
            Ok(l) => l,
            Err(_) => return Ok(()), // Connection closed
        };
        if !(10..=4096).contains(&length) {
            return Err("Invalid RCON packet length".into());
        }

        let request_id = read_i32_le(&mut stream).await?;
        let packet_type = read_i32_le(&mut stream).await?;

        // Read payload (length - 10 bytes for request_id + type + 2 null terminators)
        let payload_len = (length - 10) as usize;
        let mut payload = vec![0u8; payload_len];
        if payload_len > 0 {
            stream.read_exact(&mut payload).await?;
        }
        // Read 2 null terminators
        let mut term = [0u8; 2];
        stream.read_exact(&mut term).await?;

        let payload_str = String::from_utf8_lossy(&payload).to_string();

        match packet_type {
            PACKET_TYPE_LOGIN => {
                if payload_str == password {
                    authenticated = true;
                    write_rcon_packet(&mut stream, request_id, PACKET_TYPE_LOGIN_SUCCESS, "")
                        .await?;
                    debug!("RCON client authenticated");
                } else {
                    write_rcon_packet(&mut stream, -1, PACKET_TYPE_LOGIN_SUCCESS, "").await?;
                    return Ok(());
                }
            }
            PACKET_TYPE_COMMAND => {
                if !authenticated {
                    write_rcon_packet(
                        &mut stream,
                        request_id,
                        PACKET_TYPE_RESPONSE,
                        "Not authenticated",
                    )
                    .await?;
                    continue;
                }

                let (response_tx, response_rx) = tokio::sync::oneshot::channel();
                let cmd = RconCommand {
                    command: payload_str,
                    response_tx,
                };
                if cmd_tx.send(cmd).await.is_err() {
                    return Err("Server shutting down".into());
                }
                let response = response_rx.await.unwrap_or_else(|_| "Error".into());
                write_rcon_packet(&mut stream, request_id, PACKET_TYPE_RESPONSE, &response).await?;
            }
            _ => {
                write_rcon_packet(
                    &mut stream,
                    request_id,
                    PACKET_TYPE_RESPONSE,
                    "Unknown packet type",
                )
                .await?;
            }
        }
    }
}

async fn read_i32_le(
    stream: &mut tokio::net::TcpStream,
) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).await?;
    Ok(i32::from_le_bytes(buf))
}

async fn write_rcon_packet(
    stream: &mut tokio::net::TcpStream,
    request_id: i32,
    packet_type: i32,
    body: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let body_bytes = body.as_bytes();
    let length = 10 + body_bytes.len() as i32;
    stream.write_all(&length.to_le_bytes()).await?;
    stream.write_all(&request_id.to_le_bytes()).await?;
    stream.write_all(&packet_type.to_le_bytes()).await?;
    stream.write_all(body_bytes).await?;
    stream.write_all(&[0, 0]).await?;
    stream.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn rcon_packet_type_constants() {
        assert_eq!(super::PACKET_TYPE_COMMAND, 2);
        assert_eq!(super::PACKET_TYPE_LOGIN, 3);
        assert_eq!(super::PACKET_TYPE_RESPONSE, 0);
        assert_eq!(super::PACKET_TYPE_LOGIN_SUCCESS, 2);
    }
}
