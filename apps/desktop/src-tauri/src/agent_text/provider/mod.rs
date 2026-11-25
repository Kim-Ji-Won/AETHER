//! Native Responses foundation. Production network dispatch and credentials are disabled.
pub mod credential;
pub mod consent;
pub mod error;
pub mod policy;
pub mod adapter;
mod transport;
mod sse;
mod mapper;

use super::*;
use error::ProviderError;
use mapper::{Mapped, Mapper};
use policy::{Accounting, Ledger, OfflinePolicy, RequestPreview};
use sse::Decoder;

/// No permission flag can enable production dispatch in this phase.
pub fn live_dispatch() -> Result<(), ErrorCode> { Err(ErrorCode::ProviderDisabled) }

struct Stream { decoder: Decoder, mapper: Mapper, canonical_seq: u32, events: usize }

/// Native provider driver for offline integration, outside the app's Tauri state.
/// The existing Host serializes admission and canonical events. An attached HTTP
/// reader exclusively owns ingress until detach. The WKWebView keeps its own
/// existing synthetic session and cannot construct or dispatch this driver.
pub struct OfflineProvider<R = credential::DisabledCredentials> {
    credentials: R,
    consent: consent::ConsentState,
    host: Host,
    ledger: Ledger,
    identity: Option<Identity>,
    stream: Option<Stream>,
    transport: Option<transport::Transfer>,
    consented_attempt: Option<Identity>,
    request: Option<RequestPreview>,
    abort_requested: bool,
    local_closed: bool,
    last_error: Option<ProviderError>,
}
impl OfflineProvider {
    pub fn new(instance: &str, host_policy: Policy, provider_policy: OfflinePolicy) -> Result<Self, ErrorCode> {
        Self::with_resolver(instance, host_policy, provider_policy, credential::DisabledCredentials)
    }
    /// Legacy virtual SSE fixture entry, available only with disabled credentials.
    pub fn start(&mut self, start:Start, host_selected_model:&str, now:u64)->Result<Identity,ErrorCode>{
        self.consent.withdraw();
        self.revocation().check_admission().map_err(|_| ErrorCode::ProviderDisabled)?;
        self.advance_to(now)?;
        let request=self.ledger.prepare(now,host_selected_model,&start.text)?;
        self.commit_start(start, request, now)
    }
}
impl<R: credential::CredentialResolver> OfflineProvider<R> {
    fn with_resolver(instance: &str, host_policy: Policy, provider_policy: OfflinePolicy, credentials: R) -> Result<Self, ErrorCode> {
        Ok(Self { credentials, consent: consent::ConsentState::new(),
            host:Host::new(instance,host_policy)?, ledger:Ledger::new(provider_policy)?,
            identity:None,stream:None,transport:None,consented_attempt:None,request:None,
            abort_requested:false,local_closed:true,last_error:None })
    }
    /// Same trusted bootstrap precondition as Host::attach, NOT an IPC endpoint.
    pub fn attach(&mut self, instance:&str, legacy_epoch:u32, live:bool)->Result<UiSession,ErrorCode>{
        let previous = self.host.legacy_epoch;
        let session = self.host.attach(instance,legacy_epoch,live)?;
        if previous != Some(legacy_epoch) {
            self.consent.withdraw();
            if self.host.current().is_some_and(|run| !run.state.terminal()
                && run.identity.session_generation == self.host.generation) {
                self.runtime_lost(self.host.now_ms())?;
            }
            self.consent.awaiting_new_context = false;
        }
        Ok(session)
    }
    pub fn snapshot(&self, session:&UiSession)->Result<Snapshot,ErrorCode>{self.host.snapshot(session)}
    pub fn inspect(&self)->Snapshot{self.host.inspect()}
    pub fn events(&self)->&[Event]{self.host.events()}
    pub fn accounting(&self)->Option<Accounting>{self.ledger.accounting}
    pub fn last_error(&self)->Option<ProviderError>{self.last_error}
    pub fn abort_requested(&self)->bool{self.abort_requested}
    pub fn local_transport_closed(&self)->bool{self.local_closed}
    pub fn preview(&self)->Option<&RequestPreview>{self.request.as_ref()}
    fn commit_start(&mut self, start:Start, request:RequestPreview, now:u64)->Result<Identity,ErrorCode>{
        let id=adapter::admit(&mut self.host, &mut self.ledger, start, &request, now)?;
        self.consented_attempt=None;
        self.identity=Some(id.clone());self.stream=None;self.request=Some(request);
        self.abort_requested=false;self.local_closed=true;self.last_error=None;
        Ok(id)
    }
    fn bound(&self,id:&Identity)->Result<(),ErrorCode>{
        if self.identity.as_ref()!=Some(id){return Err(ErrorCode::InvalidIdentity);}
        self.host.check_identity(id)
    }
    pub fn dispatch(&mut self,id:&Identity,now:u64)->Result<(),ErrorCode>{
        self.bound(id)?;self.advance_to(now)?;
        self.stream = Some(Stream::dispatch(&mut self.host, id, &self.ledger, self.request.as_ref())?);
        self.local_closed=false;Ok(())
    }
    pub fn advance_to(&mut self,now:u64)->Result<(),ErrorCode>{
        self.host.advance_to(now)?;
        if self.host.current().is_some_and(|r|matches!(r.state,State::CancelRequested|State::Failed)) && !self.local_closed {
            self.abort_requested=true;
        }
        Ok(())
    }
    fn ingest_ready(&mut self,id:&Identity,now:u64)->Result<(),ErrorCode>{
        self.bound(id)?;self.advance_to(now)?;
        if self.host.current().is_none_or(|r|r.state!=State::Running)||self.stream.is_none(){return Err(ErrorCode::AlreadyTerminal);}
        Ok(())
    }
    pub fn feed(&mut self,id:&Identity,bytes:&[u8],now:u64)->Result<(),ErrorCode>{
        if self.transport.is_some() { return Err(ErrorCode::InvalidIdentity); }
        self.feed_owned(id,bytes,now)
    }
    fn feed_owned(&mut self,id:&Identity,bytes:&[u8],now:u64)->Result<(),ErrorCode>{
        self.ingest_ready(id,now)?;
        match self.stream.as_mut().unwrap().feed(&mut self.host, id, &mut self.ledger, bytes) {
            Ok(false) => Ok(()),
            Ok(true) => {
                self.stream = None;
                self.request = None;
                if self.transport.is_none() {
                    self.local_closed = true;
                    self.host.release(id, CleanupPart::Adapter, true)?;
                }
                Ok(())
            }
            Err(error) => {
                self.abort_requested = true;
                if let adapter::FeedError::Provider(e) = &error { self.last_error = Some(*e); }
                Err(error.apply(&mut self.host))
            }
        }
    }
    fn reject(&mut self,e:ProviderError)->Result<(),ErrorCode>{
        self.last_error=Some(e);self.abort_requested=true;
        Err(adapter::reject(&mut self.host, e))
    }
    pub fn end_of_stream(&mut self,id:&Identity,now:u64)->Result<(),ErrorCode>{
        if self.transport.is_some() { return Err(ErrorCode::InvalidIdentity); }
        self.end_owned_stream(id,now)
    }
    fn end_owned_stream(&mut self,id:&Identity,now:u64)->Result<(),ErrorCode>{
        self.bound(id)?;self.advance_to(now)?;
        if self.local_closed {return Ok(());}
        if self.host.current().is_some_and(|r|r.state!=State::Running){return Err(ErrorCode::AlreadyTerminal);}
        let e=self.stream.as_ref().ok_or(ErrorCode::InvalidIdentity)?.eof_error();
        self.reject(e)
    }
    pub fn transport_error(&mut self,id:&Identity,e:ProviderError,now:u64)->Result<(),ErrorCode>{
        if self.transport.is_some() { return Err(ErrorCode::InvalidIdentity); }
        self.ingest_ready(id,now)?;self.reject(e)
    }
    pub fn cancel(&mut self,session:&UiSession,id:&Identity,now:u64)->Result<(),ErrorCode>{
        self.bound(id)?;self.advance_to(now)?;self.host.cancel(session,id)?;
        self.consent.withdraw();
        if !self.local_closed {self.abort_requested=true;}else{self.request=None;}
        Ok(())
    }
    /// Explicit abort barrier for virtual ingress. An attached native reader
    /// derives its ACK exclusively from successful handle removal and destruction.
    pub fn confirm_local_abort(&mut self,id:&Identity,confirmed:bool)->Result<(),ErrorCode>{
        self.bound(id)?;
        if self.transport.is_some() { return Err(ErrorCode::Busy); }
        if !self.abort_requested {return Err(ErrorCode::InvalidInput);}
        if confirmed {
            self.stream=None;self.request=None;self.local_closed=true;
            self.host.release(id,CleanupPart::Adapter,true)?;
        }
        Ok(())
    }
    pub fn confirm_runtime_cleanup(&mut self,id:&Identity,confirmed:bool)->Result<(),ErrorCode>{
        self.bound(id)?;
        if !self.local_closed {return Err(ErrorCode::InvalidInput);}
        self.host.release(id,CleanupPart::Runtime,confirmed)?;
        self.finish_pending_revocation();
        Ok(())
    }
    pub fn shutdown(&mut self,now:u64)->Result<(),ErrorCode>{
        self.consent.withdraw();
        if self.revocation() != consent::Revocation::AwaitingCleanup {
            self.credentials.shutdown();
        }
        self.advance_to(now)?;self.host.shutdown()?;
        if !self.local_closed{self.abort_requested=true;}else{self.request=None;}
        Ok(())
    }
    pub fn runtime_lost(&mut self,now:u64)->Result<(),ErrorCode>{
        self.consent.withdraw();
        self.consent.awaiting_new_context = true;
        self.advance_to(now)?;self.host.runtime_lost()?;
        if !self.local_closed{self.abort_requested=true;}else{self.request=None;}
        Ok(())
    }
    /// Called by the owner after losing the old runtime context. Detach any native
    /// reader before handing its release and the owner's context confirmation to Host.
    pub fn confirm_lost_owned_cleanup(&mut self)->Result<(),ErrorCode>{
        let id=self.identity.as_ref().ok_or(ErrorCode::InvalidIdentity)?;
        if self.host.inspect().session_generation==id.session_generation{return Err(ErrorCode::InvalidInput);}
        let run_id=id.run_id;
        self.close_transport()?;
        self.stream=None;self.request=None;self.local_closed=true;
        self.host.confirm_owned_cleanup(run_id)?;
        self.finish_pending_revocation();
        Ok(())
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod fixtures;
