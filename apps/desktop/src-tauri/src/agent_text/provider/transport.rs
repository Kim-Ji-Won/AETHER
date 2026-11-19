//! One polled transfer per admitted attempt. No credential lookup, IPC or retry loop.
mod http;
use super::{credential::CredentialResolver, error::ProviderError, policy::RequestPreview, *};
use std::io::{self, Write};
use std::time::Duration;

const MAX_REQUEST: usize = 32 * 1024;
const CHUNK: usize = 16 * 1024;
const MAX_ERROR: usize = 8192;
pub(super) const MAX_EVENTS: usize = 4096;

struct RequestBytes(Vec<u8>);

impl Write for RequestBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_REQUEST - self.0.len() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

pub(super) fn encode_request(body: &serde_json::Value) -> Result<Vec<u8>, ErrorCode> {
    let mut out = RequestBytes(Vec::new());
    serde_json::to_writer(&mut out, body).map_err(|_| ErrorCode::InputLimit)?;
    Ok(out.0)
}

struct Head { status: u16, sse: bool, length: Option<usize> }
enum Step { Pending, Head(Head), Bytes(Vec<u8>), End }
enum Reader {
    Http(http::Http),
    #[cfg(test)]
    Memory(std::collections::VecDeque<Result<Step, ProviderError>>),
}

impl Reader {
    fn next(&mut self) -> Result<Step, ProviderError> {
        match self {
            Self::Http(http) => http.next(),
            #[cfg(test)]
            Self::Memory(steps) => steps.pop_front().unwrap_or(Ok(Step::End)),
        }
    }
    fn close(self) -> Result<(), ErrorCode> {
        match self {
            Self::Http(http) => http.close(),
            #[cfg(test)]
            Self::Memory(_) => Ok(()),
        }
    }
}

pub(super) struct Transfer {
    reader: Option<Reader>,
    status: Option<u16>,
    error_body: Vec<u8>,
    remaining: usize,
}

enum Input { Wait, Data(Vec<u8>), Eof }

impl Transfer {
    fn next(&mut self) -> Result<Input, ProviderError> {
        let reader = self.reader.as_mut().ok_or(ProviderError::Disconnected)?;
        match reader.next()? {
            Step::Pending => Ok(Input::Wait),
            Step::Head(head) => {
                if self.status.replace(head.status).is_some() { return Err(ProviderError::MalformedSse); }
                match head.status {
                    200 if !head.sse => Err(ProviderError::UnsupportedContent),
                    200 if head.length.is_some_and(|n| n > self.remaining) => Err(ProviderError::BufferLimit),
                    200 => Ok(Input::Wait),
                    400..=599 if head.length.is_none_or(|n| n <= MAX_ERROR) => Ok(Input::Wait),
                    status => Err(ProviderError::http(status, None, None)),
                }
            }
            Step::Bytes(data) => {
                let status = self.status.ok_or(ProviderError::MalformedSse)?;
                if data.len() > CHUNK { return Err(ProviderError::BufferLimit); }
                if status != 200 && data.len() > MAX_ERROR - self.error_body.len() {
                    return Err(ProviderError::http(status, None, None));
                }
                self.remaining = self.remaining.checked_sub(data.len()).ok_or(ProviderError::BufferLimit)?;
                if status == 200 { return Ok(Input::Data(data)); }
                self.error_body.extend_from_slice(&data);
                Ok(Input::Wait)
            }
            Step::End => match self.status {
                None => Err(ProviderError::Disconnected),
                Some(200) => Ok(Input::Eof),
                Some(status) => Err(ProviderError::http_body(status, &self.error_body)),
            },
        }
    }
}

impl<R: CredentialResolver> OfflineProvider<R> {
    /// This is the only production HTTP entry. No address, headers or permission flags
    /// come from the caller. Even an admitted fixture run cannot open the live endpoint.
    pub fn dispatch_http(&mut self, id: &Identity, now: u64) -> Result<(), ErrorCode> {
        live_dispatch()?;
        self.begin_transfer(id, now, |request, timeout| {
            http::Http::open(request, timeout, http::Destination::Responses).map(Reader::Http)
        })
    }

    fn begin_transfer(&mut self, id: &Identity, now: u64,
        open: impl FnOnce(&RequestPreview, Duration) -> Result<Reader, ProviderError>,
    ) -> Result<(), ErrorCode> {
        self.bound(id)?;
        if self.consented_attempt.as_ref() != Some(id) { return Err(ErrorCode::PermissionDenied); }
        self.dispatch(id, now)?;
        self.consented_attempt = None;
        let request = self.request.as_ref().expect("dispatched run owns its preview");
        let reader = match open(request, Duration::from_millis(self.host.policy.run_ms)) {
            Ok(reader) => reader,
            Err(error) => {
                // open is transactional: no handle survives an initialization failure.
                let result = self.reject(error);
                self.confirm_local_abort(id, true)?;
                return result;
            }
        };
        self.transport = Some(Transfer { reader: Some(reader), status: None,
            error_body: Vec::new(), remaining: self.ledger.policy.max_stream_bytes });
        Ok(())
    }

    /// Poll from the native owner. Lack of consumer credit stops socket reads, while
    /// host time and cancellation continue to advance. No background request worker.
    pub fn poll_http(&mut self, id: &Identity, now: u64, consumer_ready: bool) -> Result<(), ErrorCode> {
        self.bound(id)?;
        self.advance_to(now)?;
        if self.transport.is_none() { return Err(ErrorCode::AlreadyTerminal); }
        let result = if self.abort_requested || !consumer_ready { Ok(()) } else {
            match self.transport.as_mut().unwrap().next() {
                Ok(Input::Wait) => Ok(()),
                Ok(Input::Data(bytes)) => self.feed_owned(id, &bytes, now),
                Ok(Input::Eof) => self.end_owned_stream(id, now),
                Err(error) => self.reject(error),
            }
        };
        if self.abort_requested || self.stream.is_none() || result.is_err() {
            self.close_transport()?;
            self.stream = None;
            self.request = None;
            self.local_closed = true;
            self.host.release(id, CleanupPart::Adapter, true)?;
        }
        result
    }

    pub(super) fn close_transport(&mut self) -> Result<(), ErrorCode> {
        if let Some(transfer) = &mut self.transport {
            // A failed detach leaves the ownership gate closed even on repeated ACKs.
            transfer.reader.take().ok_or(ErrorCode::CleanupUnconfirmed)?.close()?;
            self.transport = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
