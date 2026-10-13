use std::time::Duration;

use anyhow::{Context, Result};
use iroh::{Endpoint, EndpointAddr};
use proto::{Request, UserInfo};

/// Cap on a single identity fetch.
const FETCH_TIMEOUT: Duration = Duration::from_secs(5);

/// Local timestamp for log lines.
pub fn stamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Opens a connection asks who they are and drops the connection.
pub async fn fetch_identity(endpoint: &Endpoint, addr: &EndpointAddr) -> Result<UserInfo> {
    let connection =
        tokio::time::timeout(FETCH_TIMEOUT, endpoint.connect(addr.clone(), proto::ALPN))
            .await
            .context("timed out connecting")?
            .context("connect to agent")?;

    let (mut send, mut recv) = connection.open_bi().await.context("open stream")?;
    proto::send(&mut send, &Request::WhoAmI)
        .await
        .context("send request")?;
    let info: UserInfo = proto::recv(&mut recv).await.context("read reply")?;

    drop(connection);
    Ok(info)
}
