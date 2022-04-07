use axum::response::Html;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use serde_json::{to_value, Value};
use std::{collections::HashMap, path::Path, sync::Arc};
use tera::{Context, Tera};
use time::OffsetDateTime;
use tokio::sync::{
    mpsc::{channel, Receiver},
    RwLock,
};
use yabusame::DATE_TIME_FORMAT;

pub(crate) async fn axum_render<C: Serialize>(
    tera: &RwLock<Tera>,
    template_name: &str,
    context: C,
) -> anyhow::Result<Html<String>> {
    Ok(Html(tera.read().await.render(
        template_name,
        &Context::from_serialize(context)?,
    )?))
}

fn async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (tx, rx) = channel(1);
    let watcher = RecommendedWatcher::new(move |res| tx.blocking_send(res).unwrap())?;
    Ok((watcher, rx))
}

pub(crate) async fn tera_watcher(
    tera: Arc<RwLock<Tera>>,
    path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let (mut watcher, mut rx) = async_watcher()?;
    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) => {
                if matches!(event.kind, EventKind::Modify(_)) {
                    tera.write().await.full_reload()?;
                }
            }

            Err(e) => eprintln!("watch error: {:?}", e),
        }
    }

    Ok(())
}

pub fn date_time(value: &Value, _args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let date_time: OffsetDateTime = serde_json::from_value(value.clone())
        .map_err(|err| tera::Error::chain("couldn't deserialize OffsetDateTime", err))?;

    to_value(&date_time.format(&DATE_TIME_FORMAT).unwrap()).map_err(tera::Error::json)
}
