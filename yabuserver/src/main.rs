#![allow(unused)]
#![warn(unused_imports, unused_must_use)]

mod db;

use argh::FromArgs;
use db::{Database, DEFAULT_DATABASE_URL};
use std::{net::IpAddr, io};
use tokio::net::{TcpListener, TcpStream};
use yabusame::{Message, Response, DEFAULT_SERVER_PORT, YabuError};

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
    loop {
        socket.readable().await?;

        let message = match Message::read_from_socket(&mut socket).await {
            Ok(res) => res,

            Err(YabuError::IoError(err)) => {
                match err.kind() {
                    // `UnexpectedEof` just means the connection closed.
                    // *Probably* not worth reporting.
                    io::ErrorKind::UnexpectedEof => return Ok(()),
                    _ => return Err(err.into()),
                }
            },

            Err(err) => return Err(err.into()),
        };

        // TODO: use a database pool instead
        let database = Database::connect(DEFAULT_DATABASE_URL)?;

        let response = match message {
            Message::Add(task) => {
                database.add_task(task)?;
                Response::Nothing
            }

            Message::List => Response::Tasks(database.all_tasks()?),
            Message::Update(id, new_task) => database.update_task(id, new_task)?,

            Message::Remove(id) => {
                database.remove_task(id)?;
                Response::Nothing
            }
        };

        response.write_to_socket(&mut socket).await?;
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = argh::from_env::<Args>();
    let listener = TcpListener::bind((args.listen_address, args.port)).await?;

    loop {
        let (socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            if let Err(err) = handle_connection(socket).await {
                eprintln!("error while processing connection:");

                for err in err.chain() {
                    eprintln!("    {err}")
                }
            }
        });
    }
}
