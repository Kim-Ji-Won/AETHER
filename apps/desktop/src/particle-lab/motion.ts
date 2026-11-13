export type ParticlePhase = 'idle' | 'listening' | 'thinking' | 'speaking';
export const previewDuration = 16;
// One-shot, deterministic synthetic syllables. No microphone, playback or measured audio.
const listening = [[.65,.55,.24],[1.45,.3,.16],[2.7,.65,.52],[3.65,.36,.3],
  [4.4,1.0,1],[5.7,.32,.4],[6.2,.4,.7],[8.4,.72,.65],[9.5,.5,.44],
  [10.5,.8,.82],[11.6,.38,.28]];
const speaking = [[.5,.27,.22],[.95,.34,.4],[1.5,.25,.18],[2.6,.4,.56],
  [3.2,.27,.32],[4.5,.9,.98],[5.6,.25,.3],[6.1,.38,.66],
  [8.35,.35,.4],[8.9,.55,.76],[9.7,.27,.24],[10.3,.65,.65],[11.3,.32,.28]];
export function syntheticEnvelope(time:number,phase:ParticlePhase='listening') {
  if(time<0||time>=previewDuration)return 0;
  let level=0;
  for(const [start,length,peak] of phase==='speaking'?speaking:listening) {
    const x=(time-start)/length;
    if(x>=0&&x<=1)level+=Math.sin(Math.PI*x)**1.4*peak;
  }
  return Math.min(1,level);
}
export function damp(current:number,target:number,dt:number,attack=.055,release=.3) {
  const delta=Math.min(.1,Math.max(0,Number.isFinite(dt)?dt:0));
  return current+(target-current)*(1-Math.exp(-delta/(target>current?attack:release)));
}
const smooth=(a:number,b:number,x:number)=>{const t=Math.min(1,Math.max(0,(x-a)/(b-a)));return t*t*(3-2*t);};
export function createMotion() {
  let time=0,level=0,thinking=0,radius=1,voice=0,speakingWeight=0,silence=0;
  return {
    step(dt:number,phase:ParticlePhase,gain:number) {
      const delta=Math.min(.1,Math.max(0,Number.isFinite(dt)?dt:0)); time+=delta;
      const voiced=phase==='listening'||phase==='speaking';
      const normalizedGain=Number.isFinite(gain)?Math.min(1,Math.max(0,gain)):0;
      const raw=voiced?syntheticEnvelope(time,phase)*normalizedGain:0;
      level=damp(level,raw,delta);
      silence=raw<.025?silence+delta:0;
      const ending=1-smooth(14.2,15.8,time);
      voice=damp(voice,voiced?ending:0,delta,.25,.55);
      thinking=damp(thinking,phase==='thinking'?1:0,delta,.65,.65);
      speakingWeight=damp(speakingWeight,phase==='speaking'?1:0,delta,.5,.5);
      // Silence changes spatial radius, never opacity. Short/long rests have different targets.
      const compact=smooth(.4,1.5,silence);
      const target=1+voice*(-.06+.25*Math.sqrt(level)-.10*compact);
      radius=damp(radius,target,delta,.16,.36);
      return {time,level,raw,thinking,radius,voice,speakingWeight,silence,
        inputLevel:phase==='listening'?level:0,outputLevel:phase==='speaking'?level:0};
    },
    reset(){time=0;level=0;thinking=0;radius=1;voice=0;speakingWeight=0;silence=0;},
  };
}
