import { ReactorScene } from './ReactorScene';
import './reactor.css';
export const coreStates = {
  idle: ['대기', '대기 상태 미리보기'],
  listening: ['듣는 중', '음성 입력 상태 미리보기 · 마이크 미사용'],
  thinking: ['생각 중', '처리 상태 미리보기 · AI 미연결'],
  speaking: ['응답 중', '응답 상태 미리보기 · 음성 출력 없음'],
  awaiting_approval: ['승인 대기', '승인 대기 미리보기 · 실제 승인 요청 없음'],
  executing: ['실행 중', '실행 상태 미리보기 · 작업 실행 없음'],
  error: ['오류', '오류 상태 미리보기 · Runtime 결과와 별개'],
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
