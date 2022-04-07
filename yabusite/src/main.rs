#![feature(try_blocks)]

use anyhow::anyhow;
use argh::FromArgs;
use askama_axum::Template;
use axum::{
    http::{header::CONTENT_TYPE, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::get,
    Router, Server,
};
use std::net::SocketAddr;
use url::Url;
use yabusame::{
    connection::{default_server, url_from_str, ClientConnection},
    Message, Task,
};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    tasks: Vec<Task>,
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

        IndexTemplate { tasks }
    };

    result.map_err(|err| {
        eprintln!("error while rendering index.html: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

async fn style_css() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/css"));
    (headers, include_str!("../static/style.css"))
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

    let app = Router::new()
        .route("/", get(index))
        .route("/style.css", get(style_css));

    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
