# P1.2b Neural Nucleus 개선 · Visual Review Revision

Refs #36 / Draft PR #37. 기존 HEAD `ed302167cba8157e5848af9bfb341ef4d661cf08`은 시각적으로 승인되지 않았다.
기준 main `c74ba28b0a4afefb97de45c6e29610ff0239a4ab`과 기존 앱·캡처를 보존한다.
이 문서는 기존 [b 검증 기록](p1-2b-living-neural-core.md)을 삭제하지 않고 후속 요구와 결과를 추가한다.

## 사용자 피드백과 구조 변경

기존 형상은 중심핵이 약하고 방향이 비슷한 곡선이 얽혀 있으며, 각 선의 독립 신호가 전체 지능체로 읽히지 않았다.
노드를 무작정 늘리지 않고 기존 114개를 중심 48(8 roots + 40 volume nodes), 중간42, 외곽24로 재분포했다.
중심 노드는 하나의 표면에만 있지 않고 반경·깊이를 달리하며 교차 연결한다. 불투명 구체/중앙 sprite/배경 이미지는 없다.

- Nucleus: 작은 8개 root와 서로 다른 깊이의 40개 노드, 중복을 제거한 중심 연결100개.
- Intermediate: 중심에서 외부로 가는 연결56개. 짧고 서로 다른 방향의 3D bend를 사용해 공통 회오리 형태를 줄인다.
- Peripheral: 24개 가는 말단 가지. 이전 외곽의 다중 교차를 줄여 중심의 대비와 실루엣을 보조한다.
- 총 연결198→180, triangles56,640→52,320, draw calls4 유지. 신호용 vec2 attribute 때문에 CPU buffer byte는 별도로 비교한다.

## 시간·활성화 계약

각 primary path의 도착 시각을 부모 도착 시각 + 이동 거리로 산정한다. 연결 끝과 노드는 같은 도착 시각과 channel을 가진다.
8개 root channel을 시간차로 활성화하고, 짧게 상승한 신호가 연결을 따라 외부로 진행한 뒤 지수적으로 감쇠한다.
중심 보조 cross-link는 도착 시각을 보간하는 시각 표현이며 생물학적 신경 시뮬레이션으로 주장하지 않는다.
공유 연속 변위장의 최대 변위는 sqrt(3)×0.04 < 0.07이며 접합점이 함께 이동한다. 전체 Core 회전은 없다.

Preview thinking은 시간 진행과 신호 세기를 제한적으로 높인다. 모드 변경 시 uTime을 재설정하거나 누적 시간에 속도를 곱해
위상을 점프시키지 않는다. actual은 여전히 offline/disconnected이며 실제 AI 실행을 표현하지 않는다.
Motion OFF/reduced-motion은 현재 위상을 고정하고 새 geometry를 생성하지 않는다. renderer/VisualGate 수명주기는 유지한다.

## Framing 및 후속 Plasma

기존 local coordinate와 neuralEnvelope.radius2.65 / plasmaRadius1.45 / maxDisplacement0.07은 유지한다.
기존 radius2.65 + 8% Camera 여유는 말단보다 큰 빈 공간을 만들었다. 실제 새 형상+변위가 들어오는 fitted sphere2.25,
추가 여유2.5%로 Camera 거리만 조정한다. DOM Safe Rect와 FOV38, Dock60–120px, Navigation/Context 구조는 유지한다.
모든 vertex의 반경+변위 상한을 검사하고, 네 Safe Rect(최대 Dock 포함)에서 3개 간격으로 표본화한 vertex를 정면/±15° Camera에 투영하는 테스트를 추가한다.
이 변경은 Plasma의 최종 형상을 제한하지 않는다. c에서 envelope 전체를 사용하는 경우 fit radius를 다시 산정해야 한다.

## 성능·자원·보안 범위

4개 merged geometry/material과 생성 단계 임시 geometry dispose를 유지한다. frame에는 uniform만 갱신한다.
새 의존성·Rust/Swift/Node·Runtime/IPC/CSP/권한·계측 변경 없음. c/d/e 구현 없음.
단일 WebGL2, context loss/restore/fallback, stale callback/StrictMode 계약 회귀를 유지한다.
후속 #35 실제 restore 이후 재초기화 실패는 NOT_RUN이다. GPU time·VRAM·귀속 RSS는 UNAVAILABLE로 유지한다.

## 검증 결과

2026-11-12 macOS26 / Apple Silicon의 실제 production arm64 앱으로 확인했다.
최종 앱 입력과 커밋 일치 여부·전체 source HEAD·최신 Governance CI는 PR #37에 기록한다.
CI는 TS/Rust/Node/GUI 검증을 실행하는 것으로 해석하지 않는다.

| 검사 | 결과 | 범위·한계 |
| --- | --- | --- |
| TypeScript / Vite / sidecar production build | PASS | 패키징 beforeBuildCommand에서 typecheck 포함; 기존 500kB chunk 경고 유지 |
| Node / Scene 테스트 26개 | PASS | Runtime8 + Foundation6 + SceneHost 계약7 + neural5 |
| Rust 회귀19개 | PASS | 이번 수정에서 release/locked/offline 실행; 이후 Rust 입력 변경 없음 |
| arm64 패키징 / 실제 WKWebView Shader | PASS | Mach-O arm64, WebGL2 실제 4 draw calls / 52,320 triangles |
| 900×760 / 400×680 / 1200×800 | PASS | 기본/최소/넓은 Main과 Dock 안전 영역, 최소 창 6행 확장·내부 스크롤 |
| 정면 / ±15° / 동일 Camera 비교 | PASS | Motion OFF에서 시점별 캡처; Camera 거리 비교 옵션 추가 |
| 네 화면·AX 본문·상태 정확성 | PASS | 실제 AI 미연결, 저장소 미연결, Preview thinking 별도 표시 |
| Context·상태·Focus·Esc | PASS | 최소 창에서 Dock 미가림, 닫은 뒤 실행 버튼 Focus 복귀 |
| A/B Runtime / Native | PASS | A/B succeeded, B cancelled/reaped, native 취소 후0·승인 후1, macOS26/AX 권한 없음 조회 |
| Main 왕복20회 | PASS | 매 이탈 geometry/material/liveRenderer/RAF0, created=disposed |
| Motion OFF / reduced-motion | PASS | OFF 새 Scene 1회 렌더; macOS reduced-motion ON에서 frames1/RAF0, 설정 원래 OFF 복원 |
| 숨김 / Main 이탈 | PASS | 숨김 RAF0, 복원 도구 입력 문제는 사용자 Dock 클릭으로 해결; Main 이탈 dispose |
| 최소화 RAF 정지·복원 | BLOCKED | 최소화 후 AX가 이전 frames543/RAF1을 반환, 메뉴 복원 elementHasNoFrame. 실제 정지 여부 판정 불가; 이전 버전 PASS를 이 앱에 이월하지 않음 |
| 자체 canvas loss→restore | PASS | DOM fallback 후 generation1→2, 이전 Renderer dispose, live1 유지 |
| 초기 Shader probe 실패 fallback | PASS | Navigation과 로컬 초안 입력 유지, 정상 설정 복귀 |
| StrictMode / stale callback / restore 실패 | PASS(계약) | 실제 SceneHost 코드+React/WebGL 대역. 실제 scheduler/GPU의 증거 아님 |
| 이번 앱 reload / Runtime 강제 오류 | NOT_RUN | 관련 기존 코드 무변경·Rust/Node 계약 회귀 유지; 이전 GUI 결과는 별도 기록 |
| 실제 restore 후 재초기화 실패 / WebKit crash | NOT_RUN | 후속 #35 보존, 공유 WebKit 프로세스 강제 종료 없음 |
| GPU time / VRAM / 귀속 WebKit RSS | UNAVAILABLE | CPU buffer/submit 시간과 구분 |
| 장시간 성능·전력·VoiceOver·연속 동영상 | NOT_RUN | 짧은 실제 연속 캡처만 제공 |
| Quit·자기 프로세스 정리 | PASS | 개선판 PID40781, 20:54:01 시작, 전용 target 경로 대조. Cmd-Q 후 21:05:53 검사에서 앱/빌드/테스트 부재 |

### 비용 비교

| 지표 | 기존 ed30216 | 개선판 | 변화 |
| --- | ---: | ---: | --- |
| Nodes | 114 | 114 | 중심48 / 중간42 / 외곽24 재배치 |
| Connections | 198 | 180 | 중심100 / 중간56 / 외곽24 |
| Triangles | 56,640 | 52,320 | −4,320 (−7.63%) |
| Draw calls | 4 | 4 | 동일 |
| CPU geometry buffer bytes | 1,667,592 | 1,681,200 | +13,608 (+0.82%); signalRoute vec2 추가 |
| Balanced frame interval p95 | 34ms / 2,335 samples | 34ms / 333 samples | 별개 GUI 관찰 구간 |
| Balanced CPU submit p95 | 1ms | 1ms | GPU time 아님 |
| High frame interval / CPU p95 | 성능 미판정 | 17ms / 1ms, 2,182 samples | DPR1.5, 60fps 장시간 보증 아님 |

Balanced는 context restore 후 구간(restore-dispose 원시 기록)을 사용한다. High는 별도 탐색 구간이다.
통제된 전후 벤치마크나 성능 개선 인과관계를 주장하지 않는다. 향후 Plasma 비용과 P0 E5b 결과를 보증하지 않는다.

### 패키지·재현·보존

앱: `target/p1-2b-refinement-36/release/bundle/macos/AETHER.app`.
Bundle manifest SHA-256: `b82016dfd169494cdec91e6cbc8ac24689dacf61d369a1a8b5c7a69857826ae0`.
8개 일반 파일, 합계122,218,369 bytes. 해시 정의는 bundle 상대 경로 순의 `SHA256  path\n` manifest SHA-256이다.
기존 앱 hash `919b1bbf652825981a98d5536b4bc5ba8d0d206d651cc18f4b20b8e1e339ce63` 불변 확인.
새 의존성 설치 없음. 배포 signing/notarization 없음. 실제 앱·원시 AX·빌드 로그·입력 manifest는 전용 target/qa에 로컬 보존한다.

```sh
node --experimental-vm-modules --experimental-strip-types --test tests/neural-structure.test.mjs tests/scene-lifecycle.test.mjs tests/scene-foundation.test.mjs tests/fake-provider.test.mjs tests/sidecar-protocol.test.mjs
```

위 Node 명령은 apps/desktop에서, 아래 명령은 저장소 루트에서 실행했다. Cargo bin을 PATH에 추가했다.

```sh
CARGO_TARGET_DIR="$PWD/target/p1-2b-refinement-36" cargo test --release --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --offline
CARGO_TARGET_DIR="$PWD/target/p1-2b-refinement-36" CARGO_NET_OFFLINE=true NODE_ENV=production npm_config_ignore_scripts=true npm --prefix apps/desktop run tauri -- build --bundles app --no-sign -- --locked --offline
python3 scripts/validate_governance.py
git diff --check
```

P0 측정 프로세스가 없음을 읽기 전용 확인했다. P0 Worktree HEAD b70fe0b 및 clean 상태 불변,
#34/#27, 고정 앱, raw 증거와 P1.1/P1.2a 산출물을 수정하지 않았다. 추가 P1 빌드/GUI는 중지한다.

## 실제 앱 전후 비교

모든 새 이미지는 위 패키징 앱의 캡처다. 화면 공유 표시·포인터는 macOS/GUI 도구 표시이며 앱 디자인이 아니다.

| 기존 · 같은 Camera | 개선 · 같은 Camera | 개선 · 새 Framing |
| --- | --- | --- |
| ![Before](assets/p1-2b/main-900x760-motion-off.jpg) | ![Same camera](assets/p1-2b-refinement/same-camera-900-off.jpg) | ![Refined](assets/p1-2b-refinement/main-900-off.jpg) |

동일 비교는 900×760 / 정면 / FOV38 / 이전 distance / Motion OFF. 초기 정지 시점의 shader 위상은 버전별로 다를 수 있다.
기존 Camera 거리의 재현은 독립 테스트로 확인했다. 새 기본 Camera는 약19.4% 가까워지며 geometry 확대와 구분한다.

| 기본 Motion ON | 최소 Motion OFF | 넓은 Motion OFF |
| --- | --- | --- |
| ![Default](assets/p1-2b-refinement/main-900-on.jpg) | ![Small](assets/p1-2b-refinement/main-400-off.jpg) | ![Wide](assets/p1-2b-refinement/main-1200-off.jpg) |

| −15° | +15° | Thinking Preview |
| --- | --- | --- |
| ![Minus](assets/p1-2b-refinement/camera-minus15.jpg) | ![Plus](assets/p1-2b-refinement/camera-plus15.jpg) | ![Thinking](assets/p1-2b-refinement/thinking-preview.jpg) |

| Command Center | Memory Center | Settings |
| --- | --- | --- |
| ![Command](assets/p1-2b-refinement/command.jpg) | ![Memory](assets/p1-2b-refinement/memory.jpg) | ![Settings](assets/p1-2b-refinement/settings.jpg) |

| 최소 창 Context | 최소 창 Status | 최대 Dock |
| --- | --- | --- |
| ![Context](assets/p1-2b-refinement/context-400.jpg) | ![Status](assets/p1-2b-refinement/status-400.jpg) | ![Dock](assets/p1-2b-refinement/dock-400-expanded.jpg) |

### 실제 움직임 표본

아래는 생성 영상이 아닌 순서 있는 실제 앱 프레임이며 연속 녹화가 아니다.
최종 저장 이미지별 [캡처 UTC 시각](assets/p1-2b-refinement/capture-times.json)을 제공한다.
ON 간격은 약7.2초/9.2초이며 순간적인 전파의 전체 경로를 이 세 장만으로 입증하지 않는다. 실제 앱으로 확인한다.

| ON 1 | ON 2 | ON 3 |
| --- | --- | --- |
| ![1](assets/p1-2b-refinement/activation-01.jpg) | ![2](assets/p1-2b-refinement/activation-02.jpg) | ![3](assets/p1-2b-refinement/activation-03.jpg) |

| Thinking 1 | Thinking 2 |
| --- | --- |
| ![Thinking1](assets/p1-2b-refinement/thinking-01.jpg) | ![Thinking2](assets/p1-2b-refinement/thinking-02.jpg) |

![Shader fallback](assets/p1-2b-refinement/shader-fallback.jpg)
![Reduced motion](assets/p1-2b-refinement/reduced-motion.jpg)

## 사용자 리뷰와 남은 한계

중심핵·깊이·활성화가 사용자 요구에 충분한지는 승인되지 않았다. b의 현재 표현은 명확한 선 구조이며,
볼륨을 채우는 Plasma와 주변 공간 조명은 아직 없다. 정지 화면에서의 응집감과 활성화 인지성을 실제 앱에서 검토한다.
최소화 실환경 재검증은 남아 있으며 병합 전 확인 또는 사용자 예외 승인이 필요하다. c는 자동 시작하지 않는다.

P1_2B_VISUAL_APPROVED: NO
FINAL_AETHER_VISUAL_APPROVED: NO
MAIN_MERGED: NO
NEXT_PHASE_STARTED: NO
