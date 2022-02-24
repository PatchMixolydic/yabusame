#![allow(unused)]
#![warn(unused_imports, unused_must_use)]

use argh::FromArgs;
use std::net::IpAddr;
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
};
use yabusame::{Message, DEFAULT_SERVER_PORT};

/// Foo;
#[derive(FromArgs)]
struct Args {
    #[argh(
        option,
        short = 'a',
        description = "address to listen on",
        default = "[0, 0, 0, 0].into()"
    )]
    listen_address: IpAddr,

    #[argh(
        option,
        short = 'p',
        description = "port to serve on",
        default = "DEFAULT_SERVER_PORT"
    )]
    port: u16,
}

async fn handle_connection(mut socket: TcpStream) -> anyhow::Result<()> {
    let mut buf = Vec::new();
    socket.read_to_end(&mut buf).await?;

    let message = serde_json::from_slice::<Message>(&buf)?;
    eprintln!("got {:?}", message);
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = argh::from_env::<Args>();

    let listener = TcpListener::bind((args.listen_address, args.port)).await?;

    loop {
        let (socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            if let Err(err) = handle_connection(socket).await {
                eprintln!("error while processing connection: {}", err);
            }
        });
    }
}
