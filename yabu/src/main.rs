#![allow(unused)]
#![feature(let_chains, try_blocks)]
#![warn(unused_imports, unused_must_use)]

mod args;
mod datetime;

use anyhow::anyhow;
use args::Subcommand;
use tokio::net::{lookup_host, TcpSocket};
use yabusame::{Message, Response, Task, DEFAULT_SERVER_PORT};

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

    let message = match args.subcommand {
        Subcommand::New(new_args) => Message::New(Task::new(
            None,
            false,
            new_args.description,
            new_args.priority,
            new_args.due_date,
        )),

        Subcommand::List(_) => Message::List,
    };

    message.write_to_socket(&mut socket).await?;
    let response = Response::read_from_socket(&mut socket).await?;

    match response {
        Response::Nothing => {}
        Response::Tasks(tasks) => println!("{tasks:?}"),
    }

    Ok(())
}
