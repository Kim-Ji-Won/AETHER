//! A single nonblocking HTTP/1.1 transfer. curl owns TLS, HTTP framing and the socket.
use super::*;
use curl::easy::{Easy2, Handler, HttpVersion, List, NetRc, SslVersion, WriteError};
use curl::multi::{Easy2Handle, Multi};

pub(super) enum Destination {
    Responses,
    #[cfg(test)]
    Loopback(std::net::SocketAddr, Option<std::path::PathBuf>),
}

#[derive(Default)]
struct Capture {
    status: Option<u16>,
    content_type: Option<bool>,
    length: Option<usize>,
    headers: usize,
    ready: bool,
    sent_head: bool,
    chunk: Option<Vec<u8>>,
    paused: bool,
    failure: Option<ProviderError>,
}

impl Capture {
    fn parse(&mut self, line: &[u8]) -> Result<(), ProviderError> {
        self.headers += line.len();
        if self.headers > CHUNK { return Err(ProviderError::BufferLimit); }
        if self.ready { return Err(ProviderError::MalformedSse); }
        let line = line.strip_suffix(b"\r\n").ok_or(ProviderError::MalformedSse)?;
        if self.status.is_none() {
            let mut fields = line.split(|b| *b == b' ');
            if !matches!(fields.next(), Some(b"HTTP/1.0" | b"HTTP/1.1")) {
                return Err(ProviderError::MalformedSse);
            }
            self.status = fields.next().and_then(|v| std::str::from_utf8(v).ok()?.parse().ok());
            if !matches!(self.status, Some(200..=599)) { return Err(ProviderError::MalformedSse); }
        } else if line.is_empty() {
            self.ready = true;
        } else {
            let sep = line.iter().position(|b| *b == b':').ok_or(ProviderError::MalformedSse)?;
            let (name, rest) = line.split_at(sep);
            let value = rest[1..].trim_ascii();
            if name.eq_ignore_ascii_case(b"content-type") {
                let mime = value.split(|b| *b == b';').next().unwrap_or_default().trim_ascii();
                if self.content_type.replace(mime.eq_ignore_ascii_case(b"text/event-stream")).is_some() {
                    return Err(ProviderError::MalformedSse);
                }
            } else if name.eq_ignore_ascii_case(b"content-length") {
                let n = std::str::from_utf8(value).ok().and_then(|v| v.parse().ok()).ok_or(ProviderError::MalformedSse)?;
                if self.length.replace(n).is_some() { return Err(ProviderError::MalformedSse); }
            } else if name.eq_ignore_ascii_case(b"content-encoding") && !value.eq_ignore_ascii_case(b"identity") {
                return Err(ProviderError::UnsupportedContent);
            }
        }
        Ok(())
    }

    fn take(&mut self) -> Option<Result<Step, ProviderError>> {
        if let Some(error) = self.failure.take() { return Some(Err(error)); }
        if self.ready && !self.sent_head {
            self.sent_head = true;
            return Some(Ok(Step::Head(Head {status:self.status?, sse:self.content_type.unwrap_or(false), length:self.length})));
        }
        self.chunk.take().map(|bytes| Ok(Step::Bytes(bytes)))
    }
}

impl Handler for Capture {
    fn header(&mut self, line: &[u8]) -> bool {
        if let Err(error) = self.parse(line) { self.failure = Some(error); return false; }
        true
    }
    fn write(&mut self, bytes: &[u8]) -> Result<usize, WriteError> {
        if bytes.len() > CHUNK || !self.ready {
            self.failure = Some(ProviderError::BufferLimit);
            return Ok(0);
        }
        if self.chunk.is_some() {
            self.paused = true;
            return Err(WriteError::Pause);
        }
        self.chunk = Some(bytes.to_vec());
        Ok(bytes.len())
    }
}

pub(super) struct Http {
    easy: Easy2Handle<Capture>,
    multi: Multi,
    finished: Option<Result<(), ProviderError>>,
}

fn classify(error: curl::Error) -> ProviderError {
    if error.is_couldnt_connect() || error.is_peer_failed_verification() || error.is_ssl_connect_error() || error.is_couldnt_resolve_host() {
        ProviderError::Offline
    } else { ProviderError::Disconnected }
}

impl Http {
    #[cfg(test)]
    pub(super) fn wait_for_io(&self) {
        self.multi.wait(&mut [], Duration::from_millis(5)).unwrap();
    }

    pub(super) fn open(request: &RequestPreview, timeout: Duration, destination: Destination) -> Result<Self, ProviderError> {
        let mut easy = Easy2::new(Capture::default());
        match destination {
            Destination::Responses => easy.url(request.endpoint()).map_err(classify)?,
            #[cfg(test)]
            Destination::Loopback(addr, ca) => {
                use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
                if addr.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) && addr.ip() != IpAddr::V6(Ipv6Addr::LOCALHOST) {
                    return Err(ProviderError::Permission);
                }
                easy.url(&format!("https://{addr}/v1/responses")).map_err(classify)?;
                if let Some(path) = ca { easy.cainfo(path).map_err(classify)?; }
            }
        }
        let bytes = encode_request(&request.body()).map_err(|_| ProviderError::BufferLimit)?;
        let mut headers = List::new();
        for header in ["Content-Type: application/json", "Accept: text/event-stream", "Accept-Encoding: identity", "Expect:"] {
            headers.append(header).map_err(classify)?;
        }
        easy.http_headers(headers).map_err(classify)?;
        easy.proxy("").map_err(classify)?;
        easy.netrc(NetRc::Ignored).map_err(classify)?;
        easy.follow_location(false).map_err(classify)?;
        easy.ssl_verify_peer(true).map_err(classify)?;
        easy.ssl_verify_host(true).map_err(classify)?;
        easy.ssl_version(SslVersion::Tlsv12).map_err(classify)?;
        easy.http_version(HttpVersion::V11).map_err(classify)?;
        easy.http_content_decoding(false).map_err(classify)?;
        easy.fresh_connect(true).map_err(classify)?;
        easy.forbid_reuse(true).map_err(classify)?;
        easy.connect_timeout(timeout).map_err(classify)?;
        easy.timeout(timeout).map_err(classify)?;
        easy.buffer_size(CHUNK).map_err(classify)?;
        easy.post(true).map_err(classify)?;
        easy.post_fields_copy(&bytes).map_err(classify)?;
        let multi = Multi::new();
        let easy = multi.add2(easy).map_err(|_| ProviderError::Offline)?;
        Ok(Self {easy, multi, finished:None})
    }

    pub(super) fn next(&mut self) -> Result<Step, ProviderError> {
        if let Some(step) = self.easy.get_mut().take() { return step; }
        if let Some(result) = self.finished { return result.map(|_| Step::End); }
        if std::mem::take(&mut self.easy.get_mut().paused) {
            self.easy.unpause_write().map_err(classify)?;
        }
        self.multi.perform().map_err(|_| ProviderError::Disconnected)?;
        self.multi.messages(|m| {
            if let Some(result) = m.result_for2(&self.easy) { self.finished = Some(result.map_err(classify)); }
        });
        self.easy.get_mut().take().unwrap_or_else(|| match self.finished {
            Some(result) => result.map(|_| Step::End),
            None => Ok(Step::Pending),
        })
    }

    pub(super) fn close(self) -> Result<(), ErrorCode> {
        drop(self.multi.remove2(self.easy).map_err(|_| ErrorCode::CleanupUnconfirmed)?);
        drop(self.multi);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_slot_pauses_without_overwriting_accepted_bytes() {
        let mut c = Capture {ready:true, sent_head:true, ..Default::default()};
        assert!(matches!(c.write(&vec![7;CHUNK]), Ok(CHUNK)));
        assert!(matches!(c.write(b"x"), Err(WriteError::Pause)));
        assert!(matches!(c.take(),Some(Ok(Step::Bytes(v))) if v == vec![7;CHUNK]));
        assert!(matches!(c.write(b"x"), Ok(1)));
        assert!(matches!(c.write(&vec![0;CHUNK+1]),Ok(0)));
        assert_eq!(c.failure,Some(ProviderError::BufferLimit));
    }
}
