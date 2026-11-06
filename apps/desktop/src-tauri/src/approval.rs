use std::sync::{mpsc, Mutex, OnceLock};

type Reply = Result<u32, String>;

#[derive(Default)]
struct ApprovalState {
    count: u32,
    next_id: u64,
    pending: Option<(u64, mpsc::Sender<Reply>)>,
}

impl ApprovalState {
    fn begin(&mut self) -> Result<(u64, mpsc::Receiver<Reply>), String> {
        if self.pending.is_some() { return Err("Approval already pending".into()); }
        self.next_id = self.next_id.checked_add(1).ok_or("Request ID exhausted")?;
        let (sender, receiver) = mpsc::channel();
        self.pending = Some((self.next_id, sender));
        Ok((self.next_id, receiver))
    }

    fn complete(&mut self, id: u64, response: i32) {
        if self.pending.as_ref().map(|p| p.0) != Some(id) { return; }
        // Consume the request before changing the counter: duplicate/stale completions do nothing.
        let (_, sender) = self.pending.take().unwrap();
        let result = match response {
            1 => match self.count.checked_add(1) {
                Some(value) => { self.count = value; Ok(value) }
                None => Err("Counter exhausted".into()),
            },
            0 => Ok(self.count),
            _ => Err("Native approval failed".into()),
        };
        let _ = sender.send(result);
    }
}

static STATE: OnceLock<Mutex<ApprovalState>> = OnceLock::new();
fn state() -> &'static Mutex<ApprovalState> { STATE.get_or_init(Default::default) }

pub fn counter() -> Reply {
    Ok(state().lock().map_err(|_| "Approval state unavailable")?.count)
}

extern "C" fn complete(id: u64, response: i32) {
    if let Ok(mut state) = state().lock() { state.complete(id, response); }
}

unsafe extern "C" {
    fn aether_request_counter_approval(
        window: *mut std::ffi::c_void,
        id: u64,
        callback: extern "C" fn(u64, i32),
    );
}

pub async fn request(window: tauri::WebviewWindow) -> Reply {
    let (id, receiver) = state().lock().map_err(|_| "Approval state unavailable")?.begin()?;
    // No state lock is held while scheduling or waiting for the native sheet.
    let parent = window.clone();
    if window.run_on_main_thread(move || {
        match parent.ns_window() {
            Ok(pointer) => {
                // SAFETY: AppKit runs on the main thread; the live window is retained by Swift.
                // Callback is a static function, and IDs contain no pointers or caller input.
                unsafe { aether_request_counter_approval(pointer, id, complete); }
            }
            Err(_) => complete(id, -1),
        }
    }).is_err() { complete(id, -1); }
    tauri::async_runtime::spawn_blocking(move || receiver.recv()
        .unwrap_or_else(|_| Err("Approval response unavailable".into())))
        .await.map_err(|_| "Approval wait failed")?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_error_and_replay_do_not_increment() {
        let mut state = ApprovalState::default();
        for response in [0, -1, 99] {
            let (id, rx) = state.begin().unwrap();
            state.complete(id, response);
            let _ = rx.recv().unwrap();
            state.complete(id, 1);
            assert_eq!(state.count, 0);
        }
        let (id, rx) = state.begin().unwrap();
        state.complete(id, 1);
        assert_eq!(rx.recv().unwrap(), Ok(1));
        state.complete(id, 1);
        assert_eq!(state.count, 1);
        let (new_id, rx) = state.begin().unwrap();
        state.complete(id, 1); // old approval cannot approve a new pending request
        assert!(rx.try_recv().is_err());
        state.complete(new_id, 0);
        assert_eq!(rx.recv().unwrap(), Ok(1));
    }

    #[test]
    fn concurrent_requests_and_completions_increment_at_most_once() {
        let state = std::sync::Arc::new(Mutex::new(ApprovalState::default()));
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let handles: Vec<_> = (0..2).map(|_| {
            let state = state.clone(); let barrier = barrier.clone();
            std::thread::spawn(move || { barrier.wait(); state.lock().unwrap().begin() })
        }).collect();
        barrier.wait();
        let mut accepted = Vec::new();
        let mut rejected = 0;
        for handle in handles {
            match handle.join().unwrap() { Ok(request) => accepted.push(request), Err(_) => rejected += 1 }
        }
        assert_eq!(rejected, 1);
        let (id, rx) = accepted.pop().unwrap();
        assert!(state.lock().unwrap().begin().is_err());
        let handles: Vec<_> = (0..2).map(|_| {
            let state = state.clone();
            std::thread::spawn(move || state.lock().unwrap().complete(id, 1))
        }).collect();
        for handle in handles { handle.join().unwrap(); }
        assert_eq!(rx.recv().unwrap(), Ok(1));
        assert_eq!(state.lock().unwrap().count, 1);
    }
}
