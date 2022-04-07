use anyhow::anyhow;
use axum::{http::StatusCode, response::Html, Extension};
use axum_macros::debug_handler;
use deadpool::unmanaged;
use serde::Serialize;
use std::sync::Arc;
use tera::Tera;
use tokio::sync::RwLock;
use yabusame::{connection::ClientConnection, Message, Task};

use crate::tera_helpers::axum_render;

#[derive(Serialize)]
struct IndexContext {
    tasks: Vec<Task>,
}

#[debug_handler]
pub(crate) async fn index(
    tera: Extension<Arc<RwLock<Tera>>>,
    connection_pool: Extension<unmanaged::Pool<ClientConnection>>,
) -> Result<Html<String>, StatusCode> {
    // TODO: hack? need to manually intervene to swap
    // `anyhow::Error` for `StatusCode::INTERNAL_SERVER_ERROR`
    let result: anyhow::Result<Html<String>> = try {
        // TODO: should be a connection pool instead
        let mut connection = connection_pool.get().await?;

        let tasks = match connection.send(Message::List).await? {
            // `yabusame::Response` is qualified to avoid confusion with `http::Response`
            yabusame::Response::Tasks(tasks) => tasks,
            yabusame::Response::Error(err) => Err(err)?,
            yabusame::Response::Nothing => Err(anyhow!("got `Response::Nothing` from the server"))?,
        };

        axum_render(&tera, "index.html", IndexContext { tasks }).await?
    };

    result.map_err(|err| {
        eprintln!("error while rendering index.html:");
        for err in err.chain() {
            eprintln!("    {err}");
        }

        StatusCode::INTERNAL_SERVER_ERROR
    })
}
