//! Offline Responses foundation. No HTTP client, credentials, IPC, or live dispatch.
pub mod credential;
pub mod error;
pub mod policy;
mod sse;
mod mapper;

use super::*;
use error::ProviderError;
use mapper::{Mapped, Mapper};
use policy::{Accounting, Ledger, OfflinePolicy, RequestPreview};
use sse::Decoder;

/// No permission flag can enable production dispatch in this phase.
pub fn live_dispatch() -> Result<(), ErrorCode> { Err(ErrorCode::ProviderDisabled) }

struct Stream { decoder: Decoder, mapper: Mapper, canonical_seq: u32 }

/// Native-owned offline driver, not registered with Tauri or used by the UI.
/// Exclusive &mut is the same linearization boundary as Host. Inputs are virtual
/// transport bytes, never Renderer Frames/JSON credentials. A future live transport
/// must own its socket/task and bind it to this identity after Lifecycle validation.
pub struct OfflineProvider {
    host: Host,
    ledger: Ledger,
    identity: Option<Identity>,
    stream: Option<Stream>,
    request: Option<RequestPreview>,
    abort_requested: bool,
    local_closed: bool,
    last_error: Option<ProviderError>,
}
impl OfflineProvider {
    pub fn new(instance: &str, host_policy: Policy, provider_policy: OfflinePolicy) -> Result<Self, ErrorCode> {
        Ok(Self { host:Host::new(instance,host_policy)?, ledger:Ledger::new(provider_policy)?,
            identity:None,stream:None,request:None,abort_requested:false,local_closed:true,last_error:None })
    }
    /// Same trusted bootstrap precondition as Host::attach, NOT an IPC endpoint.
    pub fn attach(&mut self, instance:&str, legacy_epoch:u32, live:bool)->Result<UiSession,ErrorCode>{
        self.host.attach(instance,legacy_epoch,live)
    }
    pub fn snapshot(&self, session:&UiSession)->Result<Snapshot,ErrorCode>{self.host.snapshot(session)}
    pub fn inspect(&self)->Snapshot{self.host.inspect()}
    pub fn events(&self)->&[Event]{self.host.events()}
    pub fn accounting(&self)->Option<Accounting>{self.ledger.accounting}
    pub fn last_error(&self)->Option<ProviderError>{self.last_error}
    pub fn abort_requested(&self)->bool{self.abort_requested}
    pub fn local_transport_closed(&self)->bool{self.local_closed}
    pub fn preview(&self)->Option<&RequestPreview>{self.request.as_ref()}
    pub fn start(&mut self, start:Start, host_selected_model:&str, now:u64)->Result<Identity,ErrorCode>{
        self.advance_to(now)?;
        let request=self.ledger.prepare(now,host_selected_model,&start.text)?;
        let id=self.host.start_source(start,"provider")?;
        self.ledger.reserve(now,&request);
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
        self.host.dispatch(id)?;
        self.stream=Some(Stream {decoder:Decoder::new(self.ledger.policy.max_sse_frame_bytes,self.ledger.policy.max_stream_bytes),
            mapper:Mapper::new(self.host.policy.max_output_bytes,self.host.policy.max_delta_bytes,
                self.request.as_ref().ok_or(ErrorCode::PolicyUnconfigured)?.model_id()),canonical_seq:0});
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
        self.ingest_ready(id,now)?;
        // At most one framed message/JSON is allocated at a time, irrespective of chunk size.
        for byte in bytes {
            let message=match self.stream.as_mut().unwrap().decoder.push(*byte){
                Ok(m)=>m,Err(e)=>return self.reject(e),
            };
            if let Some(message)=message {
                let mapped=match self.stream.as_mut().unwrap().mapper.accept(message){
                    Ok(m)=>m,Err(e)=>return self.reject(e),
                };
                match mapped {
                    Mapped::Metadata=>{},
                    Mapped::Delta(text)=>self.canonical(id,AdapterPayload::TextDelta(text))?,
                    Mapped::Completed(usage)=>{
                        if let Err(e)=self.ledger.record(usage){return self.reject(e);}
                        self.canonical(id,AdapterPayload::Completed)?;
                        // All resources here are memory-only. Completion is still gated on
                        // Runtime release. A live reader/task close is P2.2b work.
                        self.stream=None;self.request=None;self.local_closed=true;
                        self.host.release(id,CleanupPart::Adapter,true)?;
                        // Any bytes following terminal belong to a closed stream. Ignore them;
                        // they cannot create canonical events or mutate a terminal.
                        return Ok(());
                    }
                    Mapped::Failed(e,usage)=>{
                        if let Some(usage)=usage {
                            if let Err(accounting_error)=self.ledger.record(usage){return self.reject(accounting_error);}
                        }
                        return self.reject(e);
                    }
                }
            }
        }
        Ok(())
    }
    fn canonical(&mut self,id:&Identity,payload:AdapterPayload)->Result<(),ErrorCode>{
        let stream=self.stream.as_mut().unwrap();
        stream.canonical_seq=stream.canonical_seq.checked_add(1).ok_or(ErrorCode::CounterExhausted)?;
        let result=self.host.ingest(Frame{identity:id.clone(),source_seq:stream.canonical_seq,payload});
        if result.is_err(){self.abort_requested=true;}
        result
    }
    fn reject(&mut self,e:ProviderError)->Result<(),ErrorCode>{
        self.last_error=Some(e);self.abort_requested=true;
        let ingress = e.host_code();
        // DuplicateEvent identifies the rejected frame. Repeated SSE sequence numbers
        // violate the bound stream's protocol and terminate the run.
        let terminal = match e {
            ProviderError::Duplicate => ErrorCode::ProtocolError,
            _ => ingress,
        };
        self.host.fail(terminal, Phase::Stream);
        Err(ingress)
    }
    pub fn end_of_stream(&mut self,id:&Identity,now:u64)->Result<(),ErrorCode>{
        self.bound(id)?;self.advance_to(now)?;
        if self.local_closed {return Ok(());}
        if self.host.current().is_some_and(|r|r.state!=State::Running){return Err(ErrorCode::AlreadyTerminal);}
        let e=match self.stream.as_ref().ok_or(ErrorCode::InvalidIdentity)?.decoder.finish(){
            Ok(())=>ProviderError::Incomplete,Err(e)=>e,
        };
        self.reject(e)
    }
    pub fn transport_error(&mut self,id:&Identity,e:ProviderError,now:u64)->Result<(),ErrorCode>{
        self.ingest_ready(id,now)?;self.reject(e)
    }
    pub fn cancel(&mut self,session:&UiSession,id:&Identity,now:u64)->Result<(),ErrorCode>{
        self.bound(id)?;self.advance_to(now)?;self.host.cancel(session,id)?;
        if !self.local_closed {self.abort_requested=true;}else{self.request=None;}
        Ok(())
    }
    /// Explicit barrier for the offline transport's abort confirmation. `false` cannot
    /// release resources. Future native transport must derive confirmation from ownership.
    pub fn confirm_local_abort(&mut self,id:&Identity,confirmed:bool)->Result<(),ErrorCode>{
        self.bound(id)?;
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
        self.host.release(id,CleanupPart::Runtime,confirmed)
    }
    pub fn shutdown(&mut self,now:u64)->Result<(),ErrorCode>{
        self.advance_to(now)?;self.host.shutdown()?;
        if !self.local_closed{self.abort_requested=true;}else{self.request=None;}
        Ok(())
    }
    pub fn runtime_lost(&mut self,now:u64)->Result<(),ErrorCode>{
        self.advance_to(now)?;self.host.runtime_lost()?;
        if !self.local_closed{self.abort_requested=true;}else{self.request=None;}
        Ok(())
    }
    /// Owner-only loss recovery; old Runtime ACKs stay invalid. Offline buffers can
    /// actually be dropped here; live task/socket release needs separate implementation.
    pub fn confirm_lost_owned_cleanup(&mut self)->Result<(),ErrorCode>{
        let id=self.identity.as_ref().ok_or(ErrorCode::InvalidIdentity)?;
        if self.host.inspect().session_generation==id.session_generation{return Err(ErrorCode::InvalidInput);}
        self.stream=None;self.request=None;self.local_closed=true;
        self.host.confirm_owned_cleanup(id.run_id)
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod fixtures;
