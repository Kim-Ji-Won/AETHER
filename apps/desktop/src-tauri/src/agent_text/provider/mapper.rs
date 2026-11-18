//! Text-only projection of the official Responses SSE schema (2026-11-18).
//! Unknown content/events fail closed. Provider metadata is NOT canonical IPC.
use serde_json::Value;
use super::{error::ProviderError as E, policy::Usage, sse::Message};

pub(super) enum Mapped { Metadata, Delta(String), Completed(Usage), Failed(E, Option<Usage>) }
pub(super) struct Mapper {
    expected_model: String,
    response_id: Option<String>, item_id: Option<String>, text: String,
    last_sequence: Option<u64>, part_added: bool, text_done: bool, part_done: bool,
    item_done: bool, terminal: bool, max_output: usize, max_delta: usize,
}
fn s<'a>(v: &'a Value, key: &str) -> Result<&'a str,E> { v.get(key).and_then(Value::as_str).ok_or(E::MalformedSse) }
fn n(v:&Value,key:&str)->Result<u64,E>{ v.get(key).and_then(Value::as_u64).ok_or(E::MalformedSse) }
fn array<'a>(v:&'a Value,key:&str)->Result<&'a Vec<Value>,E>{ v.get(key).and_then(Value::as_array).ok_or(E::MalformedSse) }
fn null(v:&Value,key:&str)->Result<(),E>{ if v.get(key)==Some(&Value::Null){Ok(())}else{Err(E::MalformedSse)} }
fn id(v:&Value,key:&str)->Result<String,E>{
    let value=s(v,key)?;
    if value.is_empty() || value.len()>128 || !value.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_'||b==b'-') {
        return Err(E::Identity);
    }
    Ok(value.into())
}
fn plain_part(v:&Value)->Result<&str,E>{
    if s(v,"type")? != "output_text" { return Err(E::UnsupportedContent); }
    // No annotations/links/tools/logprobs requested. Reject unexpected content rather than execute it.
    if !array(v,"annotations")?.is_empty() || !array(v,"logprobs")?.is_empty() { return Err(E::UnsupportedContent); }
    s(v,"text")
}
fn plain_item(v:&Value, status:&str)->Result<(),E>{
    if s(v,"type")? != "message" || s(v,"role")? != "assistant" { return Err(E::UnsupportedContent); }
    if s(v,"status")? != status { return Err(E::MalformedSse); }
    id(v,"id")?; array(v,"content")?;
    Ok(())
}
fn usage(u:&Value)->Result<Usage,E>{
    let result=Usage {input_tokens:n(u,"input_tokens")?,output_tokens:n(u,"output_tokens")?,total_tokens:n(u,"total_tokens")?};
    if result.input_tokens.checked_add(result.output_tokens)!=Some(result.total_tokens){return Err(E::UsageInvalid);}
    if n(&u["input_tokens_details"],"cached_tokens")?>result.input_tokens
        || n(&u["output_tokens_details"],"reasoning_tokens")?>result.output_tokens{return Err(E::UsageInvalid);}
    if u["input_tokens_details"].get("cache_write_tokens").is_some_and(|x|x.as_u64()!=Some(0)){
        return Err(E::UsageInvalid); // unpriced cache writes cannot silently pass
    }
    Ok(result)
}
fn optional_usage(r:&Value)->Result<Option<Usage>,E>{
    match r.get("usage") {Some(Value::Null)|None=>Ok(None),Some(u)=>usage(u).map(Some)}
}
impl Mapper {
    pub fn new(max_output:usize,max_delta:usize,expected_model:&str)->Self{
        Self {expected_model:expected_model.into(),response_id:None,item_id:None,text:String::new(),last_sequence:None,part_added:false,
            text_done:false,part_done:false,item_done:false,terminal:false,max_output,max_delta}
    }
    pub fn accept(&mut self, message:Message)->Result<Mapped,E>{
        if self.terminal {return Err(E::LateEvent);}
        let v:Value=serde_json::from_str(&message.data).map_err(|_|E::MalformedSse)?;
        let kind=s(&v,"type")?;
        if message.event.as_deref().is_some_and(|event| event!=kind) {return Err(E::MalformedSse);}
        let seq=n(&v,"sequence_number")?;
        if let Some(prev)=self.last_sequence {
            if seq==prev {return Err(E::Duplicate);}
            if prev.checked_add(1)!=Some(seq) {return Err(E::Sequence);}
        } else if !matches!(kind,"response.created"|"error") || seq>1 {return Err(E::Sequence);}
        // Provider sequence includes metadata. Canonical Host frames get their own sequence.
        self.last_sequence=Some(seq);
        if kind=="error" {
            s(&v,"message")?;
            for key in ["code","param"] {
                if !v.get(key).is_some_and(|x|x.is_null()||x.is_string()){return Err(E::MalformedSse);}
            }
            self.terminal=true;
            return Ok(Mapped::Failed(E::stream(v["code"].as_str()),None));
        }
        if kind=="response.created" {
            if self.response_id.is_some(){return Err(E::Sequence);}
            let r=&v["response"];
            if s(r,"object")? != "response" || s(r,"status")?!="in_progress" || !array(r,"output")?.is_empty(){return Err(E::MalformedSse);}
            null(r,"error")?; null(r,"incomplete_details")?;
            if s(r,"model")? != self.expected_model { return Err(E::Identity); }
            self.response_id=Some(id(r,"id")?);
            return Ok(Mapped::Metadata);
        }
        if self.response_id.is_none(){return Err(E::Identity);}
        match kind {
            "response.in_progress"=>{
                let r=&v["response"];self.check_response(r,"in_progress")?;
                if !array(r,"output")?.is_empty(){return Err(E::UnsupportedContent);}
                null(r,"error")?; null(r,"incomplete_details")?;Ok(Mapped::Metadata)
            }
            "response.output_item.added"=>{
                if self.item_id.is_some()||n(&v,"output_index")?!=0{return Err(E::UnsupportedContent);}
                let item=&v["item"];plain_item(item,"in_progress")?;
                if !array(item,"content")?.is_empty(){return Err(E::MalformedSse);}
                self.item_id=Some(id(item,"id")?);Ok(Mapped::Metadata)
            }
            "response.content_part.added"=>{
                self.location(&v)?;
                if self.part_added || !plain_part(&v["part"])?.is_empty(){return Err(E::MalformedSse);}
                self.part_added=true;Ok(Mapped::Metadata)
            }
            "response.output_text.delta"=>{
                self.location(&v)?;
                if !self.part_added||self.text_done{return Err(E::Sequence);}
                if !array(&v,"logprobs")?.is_empty(){return Err(E::UnsupportedContent);}
                let delta=s(&v,"delta")?;
                if delta.len()>self.max_delta||delta.len()>self.max_output-self.text.len(){return Err(E::OutputLimit);}
                self.text.push_str(delta);
                // Empty deltas are valid metadata; never violate canonical nonempty delta rule.
                if delta.is_empty(){Ok(Mapped::Metadata)}else{Ok(Mapped::Delta(delta.into()))}
            }
            "response.output_text.done"=>{
                self.location(&v)?;
                if !self.part_added||self.text_done||s(&v,"text")?!=self.text{return Err(E::Sequence);}
                if !array(&v,"logprobs")?.is_empty(){return Err(E::UnsupportedContent);}
                self.text_done=true;Ok(Mapped::Metadata)
            }
            "response.content_part.done"=>{
                self.location(&v)?;
                if !self.text_done||self.part_done||plain_part(&v["part"])?!=self.text{return Err(E::Sequence);}
                self.part_done=true;Ok(Mapped::Metadata)
            }
            "response.output_item.done"=>{
                if !self.part_done||self.item_done||n(&v,"output_index")?!=0{return Err(E::Sequence);}
                self.final_item(&v["item"])?;self.item_done=true;Ok(Mapped::Metadata)
            }
            "response.completed"=>{
                let r=&v["response"];self.check_response(r,"completed")?;
                null(r,"error")?;null(r,"incomplete_details")?;
                let out=array(r,"output")?;
                if self.item_id.is_none() {
                    if !out.is_empty()||!self.text.is_empty(){return Err(E::UnsupportedContent);}
                }else{
                    if !self.item_done||out.len()!=1{return Err(E::Sequence);}
                    self.final_item(&out[0])?;
                }
                // Future pricing must handle cache/reasoning details; do not double bill reasoning.
                let usage=usage(&r["usage"])?;
                self.terminal=true;Ok(Mapped::Completed(usage))
            }
            "response.failed"=>{
                let r=&v["response"];self.check_response(r,"failed")?;
                let err=&r["error"];s(err,"message")?;
                let e=E::stream(Some(s(err,"code")?));let usage=optional_usage(r)?;
                self.terminal=true;Ok(Mapped::Failed(e,usage))
            }
            "response.incomplete"=>{
                let r=&v["response"];self.check_response(r,"incomplete")?;
                s(&r["incomplete_details"],"reason")?;let usage=optional_usage(r)?;
                self.terminal=true;Ok(Mapped::Failed(E::Incomplete,usage))
            }
            _=>Err(E::UnsupportedContent),
        }
    }
    fn check_response(&self,r:&Value,status:&str)->Result<(),E>{
        if s(r,"model")? != self.expected_model { return Err(E::Identity); }
        if id(r,"id")?.as_str()!=self.response_id.as_deref().ok_or(E::Identity)?{return Err(E::Identity);}
        if s(r,"object")?!="response"||s(r,"status")?!=status{return Err(E::MalformedSse);}
        Ok(())
    }
    fn location(&self,v:&Value)->Result<(),E>{
        if id(v,"item_id")?.as_str()!=self.item_id.as_deref().ok_or(E::Identity)?{return Err(E::Identity);}
        if n(v,"output_index")?!=0||n(v,"content_index")?!=0{return Err(E::UnsupportedContent);}
        Ok(())
    }
    fn final_item(&self,v:&Value)->Result<(),E>{
        plain_item(v,"completed")?;
        if id(v,"id")?.as_str()!=self.item_id.as_deref().ok_or(E::Identity)?{return Err(E::Identity);}
        let parts=array(v,"content")?;
        if parts.len()!=1{return Err(E::UnsupportedContent);}
        if plain_part(&parts[0])?!=self.text{return Err(E::Sequence);}
        Ok(())
    }
}
