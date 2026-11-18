//! Deliberately fictional, memory-only fixtures. No captured request/response/user data.
use serde_json::{json,Value};
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
pub fn part(text:&str)->Value{json!({"type":"output_text","text":text,"annotations":[],"logprobs":[]})}
pub fn item(text:&str)->Value{json!({"id":"msg_fixture","type":"message","role":"assistant","status":"completed","content":[part(text)]})}
pub fn response(status:&str)->Value{
    json!({"id":"resp_fixture","object":"response","created_at":0,"model":"fixture-model",
        "status":status,"error":null,"incomplete_details":null,"output":[],"usage":null})
}
pub fn usage()->Value{json!({"input_tokens":2,"output_tokens":1,"total_tokens":3,
    "input_tokens_details":{"cached_tokens":0,"cache_write_tokens":0},"output_tokens_details":{"reasoning_tokens":0}})}
pub fn events(text:&str)->Vec<Value>{
    let mut complete=response("completed");complete["output"]=json!([item(text)]);complete["usage"]=usage();
    let mut events=vec![
        json!({"type":"response.created","response":response("in_progress")}),
        json!({"type":"response.in_progress","response":response("in_progress")}),
        json!({"type":"response.output_item.added","output_index":0,"item":{"id":"msg_fixture","type":"message","role":"assistant","status":"in_progress","content":[]}}),
        json!({"type":"response.content_part.added","item_id":"msg_fixture","output_index":0,"content_index":0,"part":part("")}),
        json!({"type":"response.output_text.delta","item_id":"msg_fixture","output_index":0,"content_index":0,"delta":text,"logprobs":[]}),
        json!({"type":"response.output_text.done","item_id":"msg_fixture","output_index":0,"content_index":0,"text":text,"logprobs":[]}),
        json!({"type":"response.content_part.done","item_id":"msg_fixture","output_index":0,"content_index":0,"part":part(text)}),
        json!({"type":"response.output_item.done","output_index":0,"item":item(text)}),
        json!({"type":"response.completed","response":complete}),
    ];
    for(i,e)in events.iter_mut().enumerate(){e["sequence_number"]=json!(i);}
    events
}
pub fn bytes(event:&Value)->Vec<u8>{format!("event: {}\ndata: {}\n\n",event["type"].as_str().unwrap(),event).into_bytes()}
pub fn all(events:&[Value])->Vec<u8>{events.iter().flat_map(bytes).collect()}
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
