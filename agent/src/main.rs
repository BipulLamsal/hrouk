//! Agent needs to run by devices that needs to be discovered.
//! When mdns connection is received via console we send it
//! the email addresses.
//! mDNS only carries the endpoint id and addresses, deliberately not the
//! Email. That keeps the broadcast harmless if someone is sniffing the LAN.
//! the identity is only revealed over an encrypted, mutually authenticated
//! iroh connection.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use iroh::{
    endpoint::{presets, Connection},
    protocol::{AcceptError, ProtocolHandler, Router},
    Endpoint,
};
use iroh_mdns_address_lookup::MdnsAddressLookup;
use proto::{Request, UserInfo, ALPN};
use tracing::info;

#[derive(Debug, Parser)]
#[command(
    name = "hrouk-agent",
    about = "Announce an user machine on the local network and identify it to the console over iroh."
)]
struct Args {
    /// Email of the user running the agent.
    #[arg(long)]
    email: String,

    /// File holding this endpoint's secret key.
    #[arg(long, default_value = "hrouk-agent.key")]
    identity: PathBuf,

    /// Don't announce on the local network.
    #[arg(long)]
    quiet: bool,
}

#[derive(Debug)]
struct Identity {
    info: UserInfo,
}

impl ProtocolHandler for Identity {
    /// console request comes here
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let (mut send, mut recv) = connection.accept_bi().await?;

        let _request: Request = proto::recv(&mut recv).await?;
        proto::send(&mut send, &self.info).await?;

        info!(peer = %connection.remote_id(), "served identity");
        connection.closed().await;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();

    let info = UserInfo {
        email: args.email.clone(),
    };

    let secret_key = proto::identity::load_or_create(&args.identity)?;

    let endpoint = Endpoint::builder(presets::Minimal)
        .secret_key(secret_key)
        .bind()
        .await
        .context("bind iroh endpoint")?;

    let endpoint_id = endpoint.id();

    // Register with mDNS before handing the endpoint to the router
    // This is what makes us visible to the console on the LAN.
    let mdns = if args.quiet {
        None
    } else {
        let mdns = MdnsAddressLookup::builder()
            .build(endpoint_id)
            .context("build mDNS address lookup")?;
        endpoint
            .address_lookup()
            .context("endpoint has no address lookup service")?
            .add(mdns.clone());
        Some(mdns)
    };

    let router = Router::builder(endpoint)
        .accept(ALPN, Identity { info: info.clone() })
        .spawn();

    println!("endpoint:{}", endpoint_id);
    println!("email:{}", info.email);

    match mdns {
        Some(_) => println!("announcing on the local network"),
        None => println!("mDNS announcement disabled (--quiet)"),
    }
    println!("identity   {}", args.identity.display());
    eprintln!("\npress ctrl-c to stop\n");

    tokio::signal::ctrl_c().await.context("wait for ctrl-c")?;
    info!("shutting down");
    router.shutdown().await.context("shut down router")?;

    Ok(())
}
