pub mod identity;

use std::io;

use iroh::endpoint::{RecvStream, SendStream};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub const ALPN: &[u8] = b"hrouk";

/// What an agent says about the person at its machine. Just the e-mail —
/// it's already unique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInfo {
    pub email: String,
}

/// The one thing the console can ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    WhoAmI,
}

/// Length-prefixed JSON, then close the send side.
///
/// The length prefix is load-bearing: a QUIC stream is just bytes with no
/// message boundaries, so without it the reader has no way to know where one
/// message ends and the next begins.
pub async fn send<T: Serialize>(send: &mut SendStream, value: &T) -> io::Result<()> {
    let payload = serde_json::to_vec(value)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    let len = u32::try_from(payload.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "frame too large"))?;

    send.write_all(&len.to_be_bytes())
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err))?;
    send.write_all(&payload)
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err))?;
    send.finish()
        .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err))?;
    Ok(())
}

pub async fn recv<T: DeserializeOwned>(recv: &mut RecvStream) -> io::Result<T> {
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf)
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::UnexpectedEof, err))?;

    let mut payload = vec![0u8; u32::from_be_bytes(len_buf) as usize];
    recv.read_exact(&mut payload)
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::UnexpectedEof, err))?;

    serde_json::from_slice(&payload)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}
