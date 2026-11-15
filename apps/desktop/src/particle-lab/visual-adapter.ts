import { VisualGate, type VisualSnapshot } from '../scene/visual-state';
import type { ParticlePhase } from './motion';

// A visual phase is not an AI connection state. Unknown preview phases are rejected,
// never promoted to listening/speaking. Audio fields remain absent, not synthetic input.
export class ParticleVisualAdapter {
  private gate: VisualGate;
  constructor(initial: VisualSnapshot, private readonly main: boolean) {
    this.gate = new VisualGate([initial.instance, initial.contextEpoch, true]);
  }
  accept(snapshot: VisualSnapshot): { phase: ParticlePhase; preview: boolean } | null {
    if (snapshot.inputLevel !== null || snapshot.outputLevel !== null) return null;
    if (snapshot.source === 'actual' && (!this.main || snapshot.phase !== 'offline')) return null;
    if (snapshot.source === 'preview' && !['idle','listening','thinking','speaking'].includes(snapshot.phase)) return null;
    if (!this.gate.accept(snapshot)) return null;
    return { phase: snapshot.source === 'actual' ? 'idle' : snapshot.phase as ParticlePhase,
      preview: snapshot.source === 'preview' };
  }
}
