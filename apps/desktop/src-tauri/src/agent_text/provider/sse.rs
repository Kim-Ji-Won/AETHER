//! Incremental bounded SSE framing. Byte fragments (including split UTF-8) are
//! retained only until a line boundary. No reconnect, id replay, or retry behavior.
use super::error::ProviderError as E;

pub(super) struct Message { pub event: Option<String>, pub data: String }
pub(super) struct Decoder {
    line: Vec<u8>, data: String, event: Option<String>,
    frame_bytes: usize, total_bytes: usize, frame_cap: usize, total_cap: usize,
    after_cr: bool, first_line: bool, poisoned: bool,
}
impl Decoder {
    pub fn new(frame_cap: usize, total_cap: usize) -> Self {
        Self { line:Vec::new(),data:String::new(),event:None,frame_bytes:0,total_bytes:0,
            frame_cap,total_cap,after_cr:false,first_line:true,poisoned:false }
    }
    pub fn push(&mut self, byte: u8) -> Result<Option<Message>, E> {
        if self.poisoned { return Err(E::MalformedSse); }
        let result=self.byte(byte);
        if result.is_err() { self.poisoned=true; self.clear(); }
        result
    }
    fn byte(&mut self, byte: u8) -> Result<Option<Message>, E> {
        if self.total_bytes>=self.total_cap { return Err(E::BufferLimit); }
        self.total_bytes+=1;
        if self.after_cr { self.after_cr=false; if byte==b'\n' { return Ok(None); } }
        if self.frame_bytes>=self.frame_cap { return Err(E::BufferLimit); }
        self.frame_bytes+=1;
        if byte!=b'\r' && byte!=b'\n' { self.line.push(byte); return Ok(None); }
        self.after_cr=byte==b'\r';
        let bytes=std::mem::take(&mut self.line);
        let mut line=std::str::from_utf8(&bytes).map_err(|_| E::MalformedSse)?;
        if self.first_line { line=line.strip_prefix('\u{feff}').unwrap_or(line); self.first_line=false; }
        if line.is_empty() {
            self.frame_bytes=0;
            let event=self.event.take();
            if self.data.is_empty() { return Ok(None); }
            self.data.pop(); // SSE removes the final data newline.
            return Ok(Some(Message { event, data:std::mem::take(&mut self.data) }));
        }
        if line.starts_with(':') { return Ok(None); }
        let (field,value)=line.split_once(':').unwrap_or((line,""));
        let value=value.strip_prefix(' ').unwrap_or(value);
        match field {
            "data" => { self.data.push_str(value); self.data.push('\n'); }
            "event" => {
                if self.event.is_some() { return Err(E::MalformedSse); }
                self.event=Some(value.into());
            }
            // SSE transport metadata has no authority; never used for replay/retry.
            "id" | "retry" => {}
            _ => {} // SSE spec ignores unknown framing fields, within byte limits.
        }
        Ok(None)
    }
    pub fn finish(&self) -> Result<(), E> {
        if self.poisoned || !self.line.is_empty() || !self.data.is_empty() || self.event.is_some() {
            Err(E::MalformedSse)
        } else { Ok(()) }
    }
    pub fn clear(&mut self) { self.line.clear(); self.data.clear(); self.event=None; }
}
