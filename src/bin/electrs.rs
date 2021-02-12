#![recursion_limit = "256"]

use anyhow::{Context, Result};
use electrs::{server, Config, Rpc, Tracker};

fn main() -> Result<()> {
    let config = Config::from_args();
    let mut tracker = Tracker::new(&config)?;
    tracker.sync().context("initial sync failed")?;
    server::run(config, Rpc::new(tracker)).context("server failed")
}
