//! Listens for mDNS announcements from agents and logs

mod registry;
mod state;

use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    path::PathBuf,
    time::Duration,
};

use anyhow::{Context, Result};
use clap::Parser;
use iroh::{endpoint::presets, Endpoint};
use iroh_mdns_address_lookup::{DiscoveryEvent, MdnsAddressLookup};
use n0_future::StreamExt;
use tracing::{debug, warn};

use crate::state::{fetch_identity, stamp};

const DEFAULT_STALE_AFTER_SECS: u64 = 60;

#[derive(Debug, Parser)]
#[command(
    name = "hrouk-console",
    about = "Log agent logins and logouts on the local network."
)]
struct Args {
    /// File holding this console's secret key.
    #[arg(long, default_value = "hrouk-console.key")]
    identity: PathBuf,

    /// Seconds without an announcement before someone counts as logged out.
    #[arg(long, default_value_t = DEFAULT_STALE_AFTER_SECS)]
    stale_after: u64,

    /// MySQL connection URL for the HR registry.
    #[arg(long, default_value = "mysql://hrouk:hrouk@127.0.0.1:3306/hrouk")]
    database_url: String,

    /// Address for the HR registry web page.
    #[arg(long, default_value = "0.0.0.0:8080")]
    listen: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();

    // connection pool
    let pool = sqlx::MySqlPool::connect(&args.database_url).await?;
    let registry_pool = pool.clone();
    let listen = args.listen;

    // seperate thread for serving web content
    tokio::spawn(async move {
        if let Err(e) = registry::serve(registry_pool, listen).await {
            tracing::warn!("registry web server stopped: {e:#}");
        }
    });

    let endpoint = Endpoint::builder(presets::Minimal)
        .secret_key(proto::identity::load_or_create(&args.identity)?)
        .bind()
        .await
        .context("bind iroh endpoint")?;

    let mdns = MdnsAddressLookup::builder()
        .build(endpoint.id())
        .context("build mDNS address lookup")?;
    endpoint
        .address_lookup()
        .context("endpoint has no address lookup service")?
        .add(mdns.clone());

    let mut emails: HashMap<String, Option<String>> = HashMap::new();
    let mut last_seen: HashMap<String, std::time::Instant> = HashMap::new();

    // for first request till we get the email response
    let mut inflight: HashSet<String> = HashSet::new();
    let (login_tx, mut login_rx) =
        tokio::sync::mpsc::unbounded_channel::<(String, Option<String>)>();

    let mut sweep = tokio::time::interval(Duration::from_secs(5));
    let mut events = mdns.subscribe().await;

    loop {
        tokio::select! {
            _ = sweep.tick() => {
                // checks for timout
                let stale = Duration::from_secs(args.stale_after);
                let now = std::time::Instant::now();

                let timed_out: Vec<String> = last_seen.iter()
                    .filter(|(_, t)| now.duration_since(**t) > stale)
                    .map(|(k, _)| k.clone())
                    .collect();

                for key in timed_out {
                    last_seen.remove(&key);
                    inflight.remove(&key);
                    let email = emails.remove(&key).flatten();
                    println!("[{}] LOGOUT {} ({})", stamp(), email.as_deref().unwrap_or("?"), key);

                    if let Some(email) = email {
                        let authorized = registry::is_valid(&pool, &email, &key).await;
                        registry::log_presence(&pool, &email, &key, "logout", authorized).await;
                    }
                }
            }
            Some((key, result)) = login_rx.recv() => {
                // already logged out then ignore
                if !emails.contains_key(&key) {
                    continue;
                }
                inflight.remove(&key);
                match result {
                    Some(email) => {
                        let slot = emails.get_mut(&key).expect("checked above");
                        if slot.is_none() {
                            *slot = Some(email.clone());
                            let authorized = registry::is_valid(&pool, &email, &key).await;
                            let tag = if authorized { "AUTHORIZED" } else { "UNKNOWN" };
                            println!("[{}] LOGIN  {} ({}) [{tag}]", stamp(), email, key);
                            registry::log_presence(&pool, &email, &key, "login", authorized).await;
                        }
                    }
                    None => debug!(%key, "identity fetch failed; will retry on next announcement"),
                }
            }
            event = events.next() => {
                let Some(event) = event else {
                    warn!("mDNS event stream ended");
                    break;
                };
                match event {
                    DiscoveryEvent::Discovered { endpoint_info, .. } => {
                        let key = endpoint_info.endpoint_id.to_string();
                        debug!(%key, "announced");
                        last_seen.insert(key.clone(), std::time::Instant::now());
                        if !emails.contains_key(&key) {
                            emails.insert(key.clone(), None);
                        }
                        let needs_fetch = emails.get(&key).map(|e| e.is_none()).unwrap_or(true);
                        if needs_fetch && inflight.insert(key.clone()) {
                            let addr = endpoint_info.to_endpoint_addr();
                            let ep: Endpoint = endpoint.clone();
                            let tx = login_tx.clone();
                            let k = key.clone();
                            tokio::spawn(async move {
                                let email = fetch_identity(&ep, &addr).await.ok().map(|i| i.email);
                                let _ = tx.send((k, email));
                            });
                        }
                    }
                    DiscoveryEvent::Expired { endpoint_id } => {
                        let key = endpoint_id.to_string();
                        last_seen.remove(&key);
                        inflight.remove(&key);
                        if let Some(email) = emails.remove(&key) {
                            println!("[{}] LOGOUT {} ({})", stamp(), email.as_deref().unwrap_or("?"), key);
                            if let Some(email) = email {
                                let authorized = registry::is_valid(&pool, &email, &key).await;
                                registry::log_presence(&pool, &email, &key, "logout", authorized).await;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
