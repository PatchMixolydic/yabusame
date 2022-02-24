#![allow(unused)]
#![feature(let_chains, try_blocks)]
#![warn(unused_imports, unused_must_use)]

mod args;
mod datetime;

use anyhow::anyhow;
use tokio::{
    io::AsyncWriteExt,
    net::{lookup_host, TcpSocket},
};
use yabusame::{Message, Task, DEFAULT_SERVER_PORT};

use crate::args::Args;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = argh::from_env::<Args>();
    let host_str = args
        .server
        .host_str()
        .ok_or_else(|| anyhow!("server url ({}) does not have a host", args.server))?;

    let addr = lookup_host((host_str, args.server.port().unwrap_or(DEFAULT_SERVER_PORT)))
        .await?
        .next()
        .ok_or_else(|| anyhow!("dns lookup for {} returned no addresses", args.server))?;

    let mut socket = TcpSocket::new_v4()?.connect(addr).await?;
    let message = serde_json::to_vec(&Message::New(Task::new(None, false, "foo", 1, None)))?;
    socket.write_all(&message).await?;
    Ok(())
}
