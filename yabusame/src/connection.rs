use tokio::net::{lookup_host, TcpSocket, TcpStream};
use url::Url;

use crate::{Message, Response, YabuError, DEFAULT_SERVER_PORT, URL_SCHEME};

pub struct ClientConnection {
    stream: TcpStream,
}

impl ClientConnection {
    pub async fn new(server_url: &Url) -> Result<Self, YabuError> {
        let host_str = server_url
            .host_str()
            .ok_or_else(|| YabuError::UrlHasNoHost(server_url.clone()))?;

        let addr = lookup_host((host_str, server_url.port().unwrap_or(DEFAULT_SERVER_PORT)))
            .await?
            .next()
            .ok_or_else(|| YabuError::DnsLookupFailed(server_url.clone()))?;

        Ok(Self {
            stream: TcpSocket::new_v4()?.connect(addr).await?,
        })
    }

    pub async fn send(&mut self, message: Message) -> Result<Response, YabuError> {
        message.write_to_socket(&mut self.stream).await?;
        Response::read_from_socket(&mut self.stream).await
    }
}

pub fn default_server() -> Url {
    Url::parse(&format!("{URL_SCHEME}://127.0.0.1:{DEFAULT_SERVER_PORT}"))
        .expect("default server URL failed to parse")
}

pub fn url_from_str(s: &str) -> Result<Url, String> {
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
