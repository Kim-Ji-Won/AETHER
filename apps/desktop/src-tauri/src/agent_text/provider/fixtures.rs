//! Deliberately fictional, memory-only fixtures. No captured request/response/user data.
use serde_json::Value;
#[path = "fixture_events.rs"]
mod fixture_events;
pub use fixture_events::*;
use super::*;

pub fn host_policy()->Policy{
    Policy{max_input_bytes:128,max_delta_bytes:64,max_output_bytes:2048,max_deltas:128,
        max_queue_frames:2,max_queue_bytes:128,run_ms:1000,idle_ms:100,cleanup_ms:50,
        rate_window_ms:1000,max_starts_per_window:100,synthetic_budget:100}
}
pub fn policy()->OfflinePolicy{
    OfflinePolicy{models:vec![policy::ModelPolicy{id:"fixture-model".into(),max_input_units:128,
        max_output_tokens:64,input_unit_price:2,output_unit_price:3}],total_cost_units:100_000,
        max_requests_per_window:10,rate_window_ms:1000,max_sse_frame_bytes:8192,max_stream_bytes:65536}
}
pub fn setup()->(OfflineProvider,UiSession){
    let mut p=OfflineProvider::new("native-fixture",host_policy(),policy()).unwrap();
    let s=p.attach("native-fixture",0,true).unwrap();(p,s)
}
pub fn start(p:&mut OfflineProvider,s:&UiSession,request:u32)->Identity{
    p.start(Start{session:s.clone(),request_id:request,text:"질문".into()},"fixture-model",0).unwrap()
}
pub fn running()->(OfflineProvider,UiSession,Identity){
    let(mut p,s)=setup();let id=start(&mut p,&s,1);p.dispatch(&id,0).unwrap();(p,s,id)
}
pub fn feed(p:&mut OfflineProvider,id:&Identity,events:&[Value])->Result<(),ErrorCode>{p.feed(id,&all(events),0)}
pub fn clean(p:&mut OfflineProvider,id:&Identity){
    if p.abort_requested(){p.confirm_local_abort(id,true).unwrap();}
    p.confirm_runtime_cleanup(id,true).unwrap();
}
pub fn assert_terminal(p:&OfflineProvider,state:State){
    let s=p.inspect();let r=s.run.unwrap();assert_eq!(r.state,state);assert_eq!(r.terminal_count,1);
    assert_eq!(p.events().iter().filter(|e|matches!(e.payload,Payload::RunSucceeded|Payload::RunFailed(_)|Payload::RunCancelled{..})).count(),1);
    assert!(p.events().windows(2).all(|w|w[0].seq+1==w[1].seq));
    assert!(p.events().iter().all(|e|e.source_kind=="provider"));
    assert_eq!(s.source_kind,"provider");
}
/// Each step is a controlled transport barrier. Never sleeps or touches a socket.
pub struct MockTransport{chunks:std::collections::VecDeque<Vec<u8>>}
impl MockTransport{
    pub fn new(bytes:&[u8],size:usize)->Self{Self{chunks:bytes.chunks(size).map(<[u8]>::to_vec).collect()}}
    pub fn step(&mut self,p:&mut OfflineProvider,id:&Identity,now:u64)->Result<bool,ErrorCode>{
        // Completion closes the owned reader. Discard queued trailing CRLF bytes;
        // direct late event injection is separately tested and remains rejected.
        if p.local_transport_closed(){self.chunks.clear();return Ok(false);}
        if let Some(bytes)=self.chunks.pop_front(){p.feed(id,&bytes,now)?;Ok(true)}else{Ok(false)}
    }
}
