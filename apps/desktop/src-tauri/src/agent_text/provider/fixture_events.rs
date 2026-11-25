//! Fictional SSE payloads shared only by the library and binary test targets.
use serde_json::{json, Value};

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
