use argh::FromArgs;
use std::fmt::Debug;
use url::Url;
use yabusame::{DEFAULT_SERVER_PORT, URL_SCHEME};

use crate::datetime::DateTime;

fn default_server() -> Url {
    Url::parse(&format!("{URL_SCHEME}://127.0.0.1:{DEFAULT_SERVER_PORT}"))
        .expect("default server URL failed to parse")
}

fn url_from_str(s: &str) -> Result<Url, String> {
    let mut maybe_url = Url::parse(s).map_err(|e| e.to_string());

    if let Ok(url) = &maybe_url && url.scheme() == URL_SCHEME {
        return maybe_url;
    }

    // Chain from `maybe_url` to preserve the original error
    maybe_url = maybe_url
        // Try to guess what the user meant. Try adding the url scheme and a port.
        .or_else(|first_err| {
            Url::parse(&format!("{URL_SCHEME}://{s}:{DEFAULT_SERVER_PORT}")).map_err(|_| first_err)
        })
        // The url might already have a scheme; try adding just a port.
        .or_else(|first_err| {
            Url::parse(&format!("{s}:{DEFAULT_SERVER_PORT}")).map_err(|_| first_err)
        });

    // If the url does not have a scheme but did have a port,
    // the base will be parsed as a scheme. Try to detect this.
    if maybe_url
        .as_ref()
        .map(|url| url.scheme().contains('.'))
        .unwrap_or(true)
    {
        maybe_url = Url::parse(&format!("{URL_SCHEME}://{s}")).map_err(|err| err.to_string());
    }

    // We're finished with recovery; raise any errors now
    let url = maybe_url?;

    if url.scheme() == URL_SCHEME {
        Ok(url)
    } else {
        Err(format!(
            "server URL has an incorrect scheme (expected {URL_SCHEME}, got {})",
            url.scheme()
        ))
    }
}

/// Foo;
#[derive(Debug, FromArgs)]
pub struct Args {
    #[argh(
        option,
        short = 's',
        description = "server where ",
        default = "default_server()",
        from_str_fn(url_from_str)
    )]
    pub server: Url,

    #[argh(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(Debug, FromArgs)]
#[argh(subcommand)]
pub enum Subcommand {
    New(New),
}

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "new", description = "")]
pub struct New {
    #[argh(option, short = 'p', description = "priority for this task")]
    pub priority: Option<u8>,

    #[argh(
        option,
        short = 'd',
        description = "date by which this task should be completed"
    )]
    pub due_date: Option<DateTime>,

    #[argh(positional)]
    pub description: String,
}
