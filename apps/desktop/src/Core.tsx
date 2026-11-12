import { ReactorScene } from './ReactorScene';
import './reactor.css';
export const coreStates = {
  idle: ['대기', '다음 연결을 기다리는 고요한 궤도'],
  listening: ['듣는 중', '입력을 향해 열린 궤도'],
  thinking: ['생각 중', '중심으로 모이는 신호'],
  speaking: ['응답 중', '바깥으로 퍼지는 에너지'],
  awaiting_approval: ['승인 대기', '사용자의 결정을 기다리는 경계'],
  executing: ['실행 중', '방향을 가진 선명한 궤도'],
  error: ['오류', '확인이 필요한 중단 상태'],
  offline: ['연결 안 됨', 'AI Provider가 연결되지 않았습니다'],
} as const;
export type CoreState = keyof typeof coreStates;
// Renderer boundary for P1.2. CSS illustration, not a WebGL scene.
export function Core({ state, lit, preview }: { state: CoreState; lit: boolean; preview: boolean }) {
  return <div className={`core-stage state-${state} ${lit ? 'is-lit' : ''}`}>
    <ReactorScene />
    <div className="core-caption"><span className="badge">{preview ? 'Preview · 시각 상태' : 'AI · 미연결'}</span>
      <p role="status">{state} <span> / {coreStates[state][0]}</span></p>
      <p className="muted">{coreStates[state][1]}</p>
    </div>
  </div>;
}
