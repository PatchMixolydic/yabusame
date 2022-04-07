#![feature(try_blocks)]

mod routes;
mod tera_helpers;

use argh::FromArgs;
use axum::{
    extract::Extension,
    http::StatusCode,
    routing::{get, get_service},
    Router, Server,
};
use deadpool::unmanaged;
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    thread::available_parallelism,
};
use tera::Tera;
use tokio::{sync::RwLock, task};
use tower_http::services::ServeDir;
use url::Url;
use yabusame::connection::{default_server, url_from_str, ClientConnection};

use crate::tera_helpers::{date_time, tera_watcher};

const DEFAULT_YABUSITE_PORT: u16 = 8000;

// The working directory is the workspace root in debug mode and
// the executable directory in release mode
const STATIC_DIR: &str = if cfg!(debug_assertions) {
    "yabusite/static"
} else {
    "static"
};

// TODO: hack; tera wants a glob but notify wants a directory
macro_rules! synced_template_consts {
    ($(
        $(#[$m:meta])*
        const {$dir:ident, $glob:ident}: &str = $s:literal;
    )*) => {
        $(
            $(#[$m])*
            const $dir: &str = $s;
            $(#[$m])*
            const $glob: &str = concat!($s, "/**/*");
        )*
    };
}

synced_template_consts! {
    #[cfg(debug_assertions)]
    const {TEMPLATE_DIR, TEMPLATE_GLOB}: &str = "yabusite/templates";
    #[cfg(not(debug_assertions))]
    const {TEMPLATE_DIR, TEMPLATE_GLOB}: &str = "templates";
}

/// Web client for the Yabusame todo list.
#[derive(Debug, FromArgs)]
pub struct Args {
    #[argh(
        option,
        short = 's',
        description = "URL that points to an instance of `yabuserver`",
        default = "default_server()",
        from_str_fn(url_from_str)
    )]
    pub server_url: Url,

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
        default = "DEFAULT_YABUSITE_PORT"
    )]
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = argh::from_env::<Args>();

    let parallelism = available_parallelism().unwrap().get();
    let mut yabuserver_connections = Vec::with_capacity(parallelism);

    for _ in 0..parallelism {
        yabuserver_connections.push(ClientConnection::new(&args.server_url).await.unwrap());
    }

    let connection_pool = unmanaged::Pool::from(yabuserver_connections);

    let static_files = get_service(ServeDir::new(STATIC_DIR)).handle_error(|err| async move {
        eprintln!("error while serving a static file: {err}");
        StatusCode::NOT_FOUND
    });

    let tera = Arc::new(RwLock::new(Tera::new(TEMPLATE_GLOB).unwrap()));
    tera.write().await.register_filter("date_time", date_time);

    let app = Router::new()
        .route("/", get(routes::index))
        .nest("/static", static_files)
        .layer(Extension(Arc::clone(&tera)))
        .layer(Extension(connection_pool));

    if cfg!(debug_assertions) {
        task::spawn(tera_watcher(tera, TEMPLATE_DIR));
    }

    Server::bind(&SocketAddr::from((args.listen_address, args.port)))
        .serve(app.into_make_service())
        .await
        .unwrap();
}
