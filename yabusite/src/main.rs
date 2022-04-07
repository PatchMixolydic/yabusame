#![feature(try_blocks)]

use anyhow::anyhow;
use argh::FromArgs;
use askama_axum::Template;
use axum::{
    http::StatusCode,
    routing::{get, get_service},
    Router, Server,
};
use std::net::SocketAddr;
use time::OffsetDateTime;
use tower_http::services::ServeDir;
use url::Url;
use yabusame::{
    connection::{default_server, url_from_str, ClientConnection},
    format_date_time, Message, Task,
};

const STATIC_DIR: &'static str = if cfg!(debug_assertions) {
    "yabusite/static"
} else {
    "static"
};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    tasks: Vec<Task>,
    // TODO: hack to access this function in the template
    format_date_time: fn(&OffsetDateTime) -> String,
}

async fn index() -> Result<IndexTemplate, StatusCode> {
    // TODO: hack? need to manually intervene to swap
    // `anyhow::Error` for `StatusCode::INTERNAL_SERVER_ERROR`
    let result: anyhow::Result<IndexTemplate> = try {
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

        IndexTemplate {
            tasks,
            format_date_time,
        }
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

    let app = Router::new()
        .route("/", get(index))
        .nest("/static", static_files);

    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
