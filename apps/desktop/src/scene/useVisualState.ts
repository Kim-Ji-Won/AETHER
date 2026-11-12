import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Generation, validSession, type Phase, type Session, type VisualSnapshot } from './visual-state';

// Shell owns session discovery. Renderer only consumes confirmed visual snapshots.
export function useVisualState(preview: Phase | '', lit: boolean) {
  const [session, setSession] = useState<Session | null>(null);
  const [snapshot, setSnapshot] = useState<VisualSnapshot | null>(null);
  const sequence = useRef(0);
  const generation = useRef(new Generation());
  useEffect(() => {
    const token = generation.current.next();
    invoke<unknown>('get_runtime_session').then(value => {
      if (generation.current.current(token)) setSession(validSession(value) ? value : null);
    }).catch(() => { if (generation.current.current(token)) setSession(null); });
    return () => { generation.current.next(); };
  }, []);
  useEffect(() => {
    if (!session) { setSnapshot(null); return; }
    setSnapshot({ instance: session[0], contextEpoch: session[1], visualSequence: ++sequence.current,
      source: preview ? 'preview' : 'actual', phase: preview || 'offline', connection: 'disconnected',
      illuminationEnabled: lit, inputLevel: null, outputLevel: null });
  }, [session, preview, lit]);
  return snapshot;
}
