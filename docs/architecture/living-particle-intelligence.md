# Living Particle Intelligence — 독립 Visual Lab

관련 Issue: #39. 기준 main: `1dc502e6380918b5e7d2b15863d807e1849365e0`.

## 승인 범위와 이력

2026-11-13 사용자 요청으로 Neural / Holographic Network 대신 파란 미세 입자 기반의 Living Particle Intelligence를 독립 Visual Lab에서 검토한다. #38의 미커밋 Neural/Plasma 소스와 앱·캡처는 원래 worktree에 보존한다. 현재 Main Core를 교체하지 않는다.

후속 사용자 피드백은 기본 입자 방향을 수용하고, 전면 재설계 없이 입자 계층과 실제 공간 수축을 강화하도록 승인했다. 이 수용은 Motion 최종 시각 승인이나 병합 승인이 아니다. 첨부 GIF의 대화상 이미지는 확인했으나 제공된 로컬 GIF 파일은 존재하지 않아 원본 애니메이션의 프레임을 추출·측정하지 못했다. 구현은 사용자 명시 기준과 로컬 합성 패턴에 근거한다.

## 진입과 소유권

`VITE_AETHER_VISUAL_LAB=particle`로 패키징한 앱만 Visual Lab으로 시작한다. 환경 변수가 없는 기본 빌드는 기존 Shell로 시작한다. `AETHER Shell 열기`는 Lab을 unmount한 뒤 기존 App을 mount한다. Reload는 빌드 진입 모드로 돌아온다.

- `ParticleLab.tsx`: React mount/cleanup, 단일 canvas, ResizeObserver, Preview Controls.
- `renderer.ts`: WebGL2 초기화, 기존 capability probe / renderPolicy / VisualGate 재사용, loop, quality, Camera, diagnostics, context loss / restore / dispose.
- `particles.ts`: seed 3901, 정적 base position/traits buffer, 단일 THREE.Points, vertex 변위와 soft point fragment.
- `motion.ts`: 결정적 합성 envelope, smoothing, 공간 반경 목표, 상태 전환. 오디오 측정기가 아니다.
- 앱 instance / context epoch / visual sequence는 기존 읽기 전용 VisualGate를 통과한다. generation과 mounted/dead 경계가 오래된 callback을 무효화한다.
- 실제 snapshot의 `inputLevel` / `outputLevel`은 null이다. Lab diagnostics의 숫자는 별도 Synthetic Preview 값이다. 실제 P3 입력·출력 계약을 Renderer가 직접 수집하지 않는다.

새 의존성, Runtime/Swift/Rust/Node, IPC, CSP, native 권한 변경은 없다. Three.js 0.187.0과 기존 라이선스 고지를 재사용한다. 마이크·녹음·TTS·AI 호출을 하지 않는다.

## 입자와 깊이

Balanced 12,000 / High 20,000, 각 1 draw call. 두 품질은 동일 master seed의 prefix를 공유한다. 입자는 구 표면이 아닌 부피 내부에 분포하고 외곽 밀도가 완만하게 감소한다. CPU attribute buffer는 각각 336,000 / 560,000 bytes이며 VRAM 추정치가 아니다.

trait 임계값으로 약 72% 미세 입자, 26% 중간 입자, 2% accent를 구성한다. 실제 표본 비율의 정확한 보증이 아닌 설정상 비중이다. 중앙은 blue, 중간은 Ice Blue, 일부 accent만 cyan/white를 사용한다. camera-relative depth에 따라 후경의 크기·알파·softness를 줄이고 전경은 더 선명하게 한다. Additive blend의 밀도 보정으로 High에서 단순 밝기 증가를 억제한다.

연결선, 구체 mesh, 중심 광원 공, 금속링, Bloom, 후처리 target은 없다. 제한된 투명 입자 overdraw와 겹침으로 체적 인상을 만든다. 이는 실제 체적 산란/DOF가 아니며 투명 입자 사이의 물리적으로 정확한 가림을 주장하지 않는다.

## 수축·확산 계약

정규화한 합성 envelope → attack 55ms / release 300ms → 감쇠된 목표 반경과 입자별 국소 변위로 전달한다.

- 목표 반경: `1 + voice × (-0.06 + 0.25 × sqrt(level) - 0.10 × compact)`.
- compact는 약 0.4~1.5초 침묵에 따라 증가한다. 짧은 쉼은 부분 응집, 긴 쉼은 더 조밀한 공간 분포를 만든다.
- 반경 보간: 확산 160ms / 수축 360ms. 실제 관측값은 프레임 간격에 따라 달라진다.
- vertex의 입자별 반응률, 방향별 local susceptibility, 접선 흐름을 결합한다. `object.scale`이나 alpha fade로 수축을 대체하지 않는다.
- 14.2~15.8초에 voice 영향이 감소하고 반경은 Idle로 부드럽게 돌아온다. 완전 수렴은 그 이후까지 이어진다.
- 매 프레임 업데이트는 소수 uniform뿐이다. CPU에서 전체 particle buffer를 재생성하지 않는다.

## 상태와 16초 Preview

| 구간 | Listening 목적 |
| --- | --- |
| 0~0.65s | 기본 조밀 상태에서 대기 |
| 0.65~1.75s | 작은 발화 두 번과 부분 수축 |
| 2.7~4.01s | 중간 발화와 짧은 응집 |
| 4.4~6.6s | 강한 확산과 짧은 재활성화 |
| 6.6~8.4s | 분명한 공간 수축 |
| 8.4~11.98s | 중간/강한 재확산과 감쇠 |
| 12~14.2s | 긴 쉼과 깊은 응집 |
| 14.2~16s 이후 | 자연스러운 Idle 분포 복귀 |

Listening과 Speaking은 서로 다른 syllable event 배열을 사용한다. Speaking은 짧은 음절 묶음·문장 강세와 약간 다른 접선 흐름을 갖는다. Thinking은 반경보다 내부의 국소 집중과 다방향 흐름을 강조한다. Idle은 작은 비동기 drift만 유지한다. 실제 상태는 모두 미연결이며 화면에 PREVIEW를 표시한다.

합성 음성은 16초 **단회** 재생이다. 자동 주기 반복은 없다. `16초 Preview 다시 시작`은 seed를 유지하고 시계·smoothing을 초기화한다. 상태를 선택한 후 이 버튼을 누르면 해당 상태의 전체 패턴을 확인할 수 있다. `시점 고정`은 120Hz의 결정적 motion 적분으로 해당 시점까지 이동하고 Motion을 끈다. 이는 실제 연속 재생 기록과 구분되는 정적 비교 도구다.

## 수명주기와 성능 경계

DPR 상한 Balanced 1.25 / High 1.5, 목표 30 / 60fps. 통계는 최근 1,200개 앱 frame interval / CPU submit p50·p95다. 브라우저 clock 정밀도 때문에 CPU 0ms가 나와도 비용이 없다는 뜻이 아니다. GPU time / 실제 VRAM / WebKit 귀속 RSS는 UNAVAILABLE.

Motion OFF/reduced-motion은 단발 정적 렌더, 숨김/비활성/최소화는 지속 RAF 정지. 재개 시 누락 시간을 한꺼번에 적용하지 않는다. Context loss는 DOM fallback, restore는 React 재생성 경계에서 한 번 새 Renderer를 만든다. geometry/material/listener/RAF/observer를 cleanup한다. 공유 WebKit 프로세스를 종료하지 않는다.

검증 결과와 한계는 [Visual Lab 검증 기록](../validation/particle-visual-lab.md)을 참조한다. 사용자 시각 승인 전 Main 통합, 실제 오디오 연결, Ready 전환 및 merge는 금지한다.
