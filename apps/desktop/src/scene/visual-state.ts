// Read-only visual contract. No commands, credentials, transcripts or audio.
export type Session = readonly [string, number, boolean];
export type Phase = 'offline' | 'idle' | 'listening' | 'thinking' | 'speaking' | 'awaiting_approval' | 'executing' | 'error';
export interface VisualSnapshot {
  instance: string; contextEpoch: number; visualSequence: number;
  source: 'actual' | 'preview'; phase: Phase;
  connection: 'disconnected'; illuminationEnabled: boolean;
  inputLevel: null; outputLevel: null;
}
export function validSession(value: unknown): value is Session {
  return Array.isArray(value) && value.length === 3 && typeof value[0] === 'string' && value[0].length > 0
    && Number.isSafeInteger(value[1]) && value[1] >= 0 && value[2] === true;
}
export class VisualGate {
  private session: Session;
  private sequence = -1;
  constructor(confirmedSession: Session) { this.session = confirmedSession; }
  accept(s: VisualSnapshot): boolean {
    if (!validSession(this.session) || s.instance !== this.session[0] || s.contextEpoch !== this.session[1]
      || !Number.isSafeInteger(s.visualSequence) || s.visualSequence <= this.sequence
      || !['actual', 'preview'].includes(s.source)
      || !['offline', 'idle', 'listening', 'thinking', 'speaking', 'awaiting_approval', 'executing', 'error'].includes(s.phase)
      || s.connection !== 'disconnected' || (s.source === 'actual' && s.phase !== 'offline')) return false;
    this.sequence = s.visualSequence;
    return true;
  }
}
// Each owner invalidates all captured callbacks before releasing its resources.
export class Generation {
  private value = 0;
  next() { return ++this.value; }
  current(token: number) { return token === this.value; }
}
