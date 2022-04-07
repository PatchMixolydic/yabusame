#![feature(try_blocks)]

mod tera_helpers;

use anyhow::anyhow;
use argh::FromArgs;
use axum::{
    extract::Extension,
    http::StatusCode,
    response::Html,
    routing::{get, get_service},
    Router, Server,
};
use axum_macros::debug_handler;
use serde::Serialize;
use std::{net::SocketAddr, sync::Arc};
use tera::Tera;
use tera_helpers::axum_render;
use tokio::{sync::RwLock, task};
use tower_http::services::ServeDir;
use url::Url;
use yabusame::{
    connection::{default_server, url_from_str, ClientConnection},
    Message, Task,
};

use crate::tera_helpers::{date_time, tera_watcher};

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

#[derive(Serialize)]
struct IndexTemplate {
    tasks: Vec<Task>,
}

#[debug_handler]
async fn index(tera: Extension<Arc<RwLock<Tera>>>) -> Result<Html<String>, StatusCode> {
    // TODO: hack? need to manually intervene to swap
    // `anyhow::Error` for `StatusCode::INTERNAL_SERVER_ERROR`
    let result: anyhow::Result<Html<String>> = try {
        // TODO: should be a connection pool instead
        let mut connection = ClientConnection::new(&"yabu://127.0.0.1:11180".parse()?)
            .await
            .unwrap();

        let tasks = match connection.send(Message::List).await? {
            // `yabusame::Response` is qualified to avoid confusion with `http::Response`
            yabusame::Response::Tasks(tasks) => tasks,
            yabusame::Response::Error(err) => Err(err)?,
            yabusame::Response::Nothing => Err(anyhow!("got `Response::Nothing` from the server"))?,
        };

        axum_render(&tera, "index.html", IndexTemplate { tasks }).await?
    };

    result.map_err(|err| {
        eprintln!("error while rendering index.html: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
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
    pub server: Url,
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    let static_files = get_service(ServeDir::new(STATIC_DIR)).handle_error(|err| async move {
        eprintln!("error while serving a static file: {err}");
        StatusCode::NOT_FOUND
    });

    let tera = Arc::new(RwLock::new(Tera::new(TEMPLATE_GLOB).unwrap()));
    tera.write().await.register_filter("date_time", date_time);

    let app = Router::new()
        .route("/", get(index))
        .nest("/static", static_files)
        .layer(Extension(Arc::clone(&tera)));

    if cfg!(debug_assertions) {
        task::spawn(tera_watcher(tera, TEMPLATE_DIR));
    }

    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
