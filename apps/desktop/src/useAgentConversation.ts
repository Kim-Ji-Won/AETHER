import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AgentRuntime, type View } from './agent-runtime';

export type Connection = 'reconnecting' | 'connected' | 'disconnected';
// One owner for the entire Shell. Navigation never mounts a second producer or timer.
export function useAgentConversation() {
  const owner = useRef<AgentRuntime | null>(null);
  if (!owner.current) owner.current = new AgentRuntime((command, payload) => invoke<View>(command, payload));
  const runtime = owner.current;
  const [view, setView] = useState<View | null>(null);
  const [busy, setBusy] = useState(false);
  const [submitted, setSubmitted] = useState(runtime.submitted);
  const [connection, setConnection] = useState<Connection>('reconnecting');
  const connecting = useRef(false);
  async function reconnect() {
    if (connecting.current) return;
    connecting.current = true; setConnection('reconnecting');
    try { await runtime.bootstrap(); setConnection('connected'); }
    catch { setConnection('disconnected'); }
    finally { connecting.current = false; }
  }
  useEffect(() => {
    let mounted = true;
    runtime.onChange = () => {
      if (mounted) { setView(runtime.view); setBusy(runtime.busy); setSubmitted(runtime.submitted); }
    };
    // StrictMode repeats effect setup; AgentRuntime coalesces the bootstrap promise.
    void runtime.bootstrap().then(() => { if (mounted) setConnection('connected'); })
      .catch(() => { if (mounted) setConnection('disconnected'); });
    let reading = false;
    const timer = setInterval(async () => {
      if (!runtime.view || reading) return;
      reading = true;
      try { await runtime.refresh(); if (mounted) setConnection('connected'); }
      catch { if (mounted) setConnection('disconnected'); }
      finally { reading = false; }
    }, 100);
    return () => { mounted = false; clearInterval(timer); };
  }, [runtime]);
  return { runtime, view, busy, submitted, connection, reconnect };
}
export type Conversation = ReturnType<typeof useAgentConversation>;
