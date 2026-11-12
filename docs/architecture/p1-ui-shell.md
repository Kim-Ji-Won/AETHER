# P1.1 UI Shell · Visual Reference 구현 및 검증

Refs #28. 기준 main: `5dd1ae9f8323d96ddac842374706c7fe9edda220`.
브랜치: `feat/28-ui-shell`. E5b worktree·계측·결과는 변경하지 않는다.

## 승인과 범위

사용자가 2026-11-12 승인한 것은 Visual Reference 기반 P1.1 구현 및 Immersive AI Workspace 구조 변경 계획이다.
레퍼런스 3번의 제품 구조, 2번의 Neural Plasma 품질, 1번의 공간·Orbital·조명을 기준으로 한다.
이미지는 목표이며 실제 구현 증거가 아니다. 이미지 전체를 배경으로 사용하지 않는다.
현재 2D Core는 최종 디자인으로 승인되지 않았다.

```text
P1_1_PLAN_APPROVED: YES
VISUAL_REFERENCE_SELECTED: YES
VISUAL_DIRECTION_APPROVED: NO
MERGE_ALLOWED: NO
MAIN_MERGED: NO
NEXT_PHASE_STARTED: NO
```

## 구조와 데이터 경계

- `Shell.tsx`: 탐색·Preview 선택·로컬 초안·Motion 설정 소유. 선택한 제품 화면을 마운트하고 제목으로 포커스를 이동한다.
- `ShellParts.tsx`: 실제 Navigation, StatusArea, ContextPanel, CommandDock. 모든 조작은 DOM/React이며 배경 이미지에 그린 컨트롤이 아니다.
- `Core.tsx` / `ReactorScene.tsx`: 기존 2D placeholder와 상태 인터페이스. 3D 구현은 없음. 기존 SVG Geometry는 이번 Shell 재구성에서 변경하지 않는다.
- `useMotion.ts`: 시스템 reduced-motion, 사용자 설정, 문서 숨김, 창 비활성, 미리보기 닫힘을 반영한다. native IPC·capability·timer를 추가하지 않는다.
- `FakeRun.tsx`: 변경 없이 유지. 화면을 이동해도 마운트를 유지해 합성 작업을 중단하지 않는다. Rust 원장·승인 소유권은 변하지 않는다.
- `main.tsx`: 기존 코어 점등과 macOS/Accessibility/승인 카운터 핸들러를 Shell에 전달한다.

Main은 Provider·기억 미연결, 음성 준비 중을 표시한다. Context는 실제 대화·활동 기록이 없음을 표시한다.
입력 초안은 Shell 메모리에만 남고 앱 종료 시 사라진다. 저장·로그·네트워크 전송은 하지 않는다.
전송·음성·첨부·Provider 연결·기억 검색/관리는 준비 중이며 비활성화한다.
Fake Runtime의 실행 상태를 실제 AI의 상태로 매핑하지 않는다.

`idle`, `listening`, `thinking`, `speaking`, `awaiting_approval`, `executing`, `error`, `offline`은
Diagnostics에서 시각 Preview로 선택할 수 있다. 실제 Provider 연결 상태는 별도로 미연결을 유지한다.
상태명과 설명을 제공하며 색만으로 상태를 구분하지 않는다.

## 레이아웃과 디자인 시스템

남청색 공간, 시안 포커스, 실버 경계, 제한된 바이올렛 강조를 사용한다.
유리 느낌은 정적인 배경·경계·하이라이트로 표현하며 화면 전체 blur를 사용하지 않는다.
상단 native titlebar·창 버튼·창 크기 제한을 유지한다. Dark window theme는 이전 UI 변경에 포함돼 있다.

- 900×760: 72px Navigation 레일, 작은 AI 연결 상태 버튼, 확장된 Scene 영역과 하단 Dock.
- 400×680: 60px Navigation 레일. 탐색 이름을 펼칠 수 있다. Context와 연결 상세는 Scene 위의 닫을 수 있는 패널.
  입력을 줄이지 않고 Dock 버튼을 다음 줄로 재배치한다. 다른 화면은 본문만 스크롤한다.
- 넓은 창에서도 72px 레일을 유지한다. 메뉴 이름은 Hover/Focus로 표시하고, 펼침 버튼으로 216px 탐색 영역을 연다.
- Context와 상태 상세는 모든 크기에서 기본 접힘이며 동시에 하나만 열린다. 내부 스크롤을 허용하고 Dock 영역을 침범하지 않는다.
- Main은 작은 상단 조작 영역과 `minmax(0,1fr) / auto`의 Scene·Dock으로 구성한다. 큰 Main 제목과 Breadcrumb은 제거하되 숨겨진 h1은 유지한다.
- Main의 감성 슬로건과 반복 AETHER 워드마크는 제거했다. 작은 기술 식별 문구만 유지하며 실제 연결 상태를 별도 표시한다.

최신 사용자 수정 지시에 따라 기존 2D Core는 Diagnostics의 선택형 미리보기로 이동한다.
Main은 Scene reserved 영역을 명시하며 기존 SVG를 시각적 주인공으로 사용하지 않는다.
이 영역의 빈 공간은 P1.2 장면의 배치 계약이며 완성된 시네마틱 장면으로 주장하지 않는다.

## 접근성과 Motion

제품 화면의 `hidden` 토글만으로 WebKit 접근성 트리가 갱신되지 않던 관찰에 대응해
Main·Memory·Settings는 조건부로 마운트한다. Runtime Console은 작업 수명을 위해 유지하되 활성 제품 본문 뒤에 배치한다. 단순 조건부 마운트만으로는 재현됐고, 숨겨진 진단 영역 뒤의 형제 본문이 누락되는 순서도 수정했다.
실제 앱에서 Command → Memory → Settings → Command를 반복해 본문과 포커스가 나타나는 것을 확인했다. 합성 작업의 run_id·진행률도 전환 중 유지됐다.

탐색 이름·제목·입력 label·disabled 사유·skip link·focus-visible을 제공한다.
Context/상태 상세를 열면 닫기 버튼으로 포커스를 옮기고, 닫기 또는 Escape 후 실행 버튼으로 복귀한다.
탐색 펼침 영역도 Escape로 닫고 탐색 토글로 포커스를 돌려준다.
창 비활성도 보수적으로 Motion 정지 조건에 포함한다. 마지막 자동 정지 사유는 Diagnostics에서 확인한다.
실제 최소화·앱 숨김과 시스템 동작 줄이기는 별도로 GUI에서 검증한다.
기존 FakeRun의 원장 조회 polling은 이번 변경 범위에서 유지한다. Motion 중지를 모든 앱 작업의 중지로 표현하지 않는다.

## P1.2 Scene 요구사항 — 미착수

P1.1 Scene 영역을 실제 3D renderer로 교체하되 Navigation·상태·Dock은 DOM에 유지한다.
Renderer는 AI 연결·승인·작업 실행을 소유하지 않는다. GPU 실패 시 상태와 입력 UI는 계속 표시하고
명확한 정적 fallback을 제공해야 한다. 실제 데이터가 없으면 Preview 표시를 유지한다.

구현 전 다음 설계를 별도로 검토한다.

| 영역 | 요구사항 |
|---|---|
| Scene Architecture | 전경·중경·후경, 광원 중심, UI 안전 영역, resize/visibility/dispose 수명 |
| Orbital Geometry | 두께 있는 금속 구조, 실제 가림·원근·독립 운동. 원자 모형 같은 균일한 선 링 남발 금지 |
| Neural Plasma Shader | 3차원 공간의 밀도·필라멘트·에너지 흐름, 시간 변화, 투명도와 depth 처리. 단순 발광 구체는 최종 품질 아님 |
| Lighting | 청백색 중심광, 주변 궤도와 바닥의 일관된 반응. emissive와 실제 조명 기여를 구분 |
| Materials | 금속 roughness/반사, 제한된 유리 투과, 소재별 밝기 위계 |
| Camera | 고정된 기본 구도, 작은 창에서 Core·상태가 겹치지 않는 프레이밍, 자동 카메라 운동 절제 |
| Animation | 궤도와 유기적 흐름의 속도 위계, 급격한 번쩍임 금지, 상태 전환과 reduced-motion 정적 구도 |
| 품질 | 여러 각도에서 깊이·가림·두께가 확인될 것. 장면 전체가 같은 광원을 공유할 것. UI 가독성 유지 |

초기 검토용 성능 예산이며 측정 결과나 확정 요구가 아니다:
활성 60fps 목표 / 필요 시 30fps, DPR 상한 1.5, draw call 60 이하·삼각형 15만 이하를 출발점으로 평가한다.
투명 레이어·볼류메트릭·굴절·반사·shadow·Bloom의 동시 비용을 Apple Silicon WKWebView에서 측정한다.
숨김·최소화는 지속 렌더링 중지, reduced-motion은 정적 장면과 상태 변경 시 갱신이 기본이다.
의존성 채택·버전·라이선스·CSP 영향은 P1.2 계획 승인 전에 검토한다. 이번 단계에 Three.js/R3F 설치 없음.

### Cinematic Spatial Interface 통합 요구사항

2026-11-12 추가 사용자 요구다. 최종 목표는 Core만 3D인 대시보드가 아니라,
실제 3D Core와 고품질 2.5D 인터페이스가 하나의 공간으로 느껴지는 제품이다.
아래 항목은 P1.2의 명시적 설계·검증 범위이며 현재 구현 완료 또는 조기 착수 선언이 아니다.
Navigation·Context·Command Dock의 조작과 접근성은 DOM/React가 유지한다. 모든 UI의 WebGL 전환은 요구하지 않는다.

| 통합 항목 | P1.2 설계 및 검증 기준 | P6 최종 조정 |
|---|---|---|
| 소재 | 유리의 투명도·밝은 가장자리, 금속의 실버 경계·하이라이트를 공통 토큰으로 설계. 텍스트 아래는 읽을 수 있는 불투명도를 확보 | 실제 콘텐츠 밀도와 배경 밝기별 소재·대비 조정 |
| 공간 계층 | 후경 Chamber, 중경 Core/Orbital, 전경 DOM 조작의 깊이와 겹침 규칙. 패널은 미세한 높이 차이를 갖되 입력·Focus 위치는 안정적 | 네 화면 및 전환 사이 깊이·간격·macOS titlebar 일체감 정제 |
| 공유 조명 | 청백색·시안 중심광, 절제된 바이올렛·실버. Scene과 UI가 같은 광원 방향·강도 범위·상태 팔레트를 사용 | Core Bloom과 UI rim light의 강도·색 균형을 실제 앱에서 조정 |
| Core 반응 | Core 상태 변화에 패널 가장자리와 Dock 강조가 함께 반응. 전체 UI의 밝기 맥동·텍스트 색 흔들림은 금지 | 상태 전환의 속도·지연·명암을 다듬고 시각적 과부하 검사 |
| 상호작용 | Hover는 제한된 소재 강조, Focus는 항상 식별 가능한 독립 윤곽. Transition은 조작 위치와 읽기 순서를 바꾸지 않음 | 키보드·마우스·텍스트 입력에서 일관성과 피로도 평가 |
| 비용과 대체 | DOM blur·합성 레이어와 3D 비용을 함께 측정. reduced-motion·숨김·최소화·GPU 실패 시 Core와 UI 모두 같은 정지 정책 적용 | 장치별 품질 단계, 배터리·발열·장시간 사용 최적화 |

Renderer와 DOM은 동일한 읽기 전용 시각 상태를 소비하도록 설계한다. 상태·광원 방향·제한된 에너지 값·Motion 허용 여부를
공유하되 Renderer가 승인·실행·음성 입력을 생성하지 않는다. 연결되지 않은 상태는 명시적 Preview fixture에서만 재생한다.
매 프레임 React 전체 트리를 갱신하거나 GPU readback으로 UI 빛을 추출하는 설계는 피하고,
상태 전환과 제한된 시각 파라미터 갱신의 비용을 먼저 비교한다. 구체 API·라이브러리는 후속 설계 승인 대상이다.

| 상태 | Core 연출 방향 | 주변 UI와 상태 텍스트 |
|---|---|---|
| `idle` | 낮은 에너지와 느린 구조 운동 | 안정된 실버/시안 경계, 명확한 대기 상태 |
| `listening` | 실제 입력 envelope에 제한적으로 반응 | Dock 입력 표시와 작은 광량 반응. 마이크 미연결이면 Preview만 허용 |
| `thinking` | 내부 흐름·궤도에 완만한 집중 | 제한된 시안 강조, 실제 처리 상태 문구. 가상 분석 수치 금지 |
| `speaking` | 실제 출력 envelope에 따른 유기적 에너지 | Dock/상태 표시에 동기화한 작은 반응. 패널 전체 맥동 금지 |
| `awaiting_approval` | 운동을 낮추거나 정지 | 승인 필요 텍스트·명확한 조작에 집중. 광원/모션은 승인 의미를 생성하지 않음 |
| `executing` | 실제 단계 전환만 반영 | 작업 진행·취소 가능 여부를 텍스트로 표시, 장식에서 진행률 추정 금지 |
| `error` | 비섬광 정지 구도 | 오류·복구 안내를 우선하고 색 외 아이콘·텍스트 제공 |
| `offline` | 약한 고정광 | 미연결·비활성 이유 유지, idle/Ready로 오인시키지 않음 |

P1.2 착수 전 산출물은 Scene Architecture, Geometry/Shader/Camera/Material/Lighting 명세,
상태 전환표, DOM/Scene 연동 계약, 작은 창 프레이밍, 품질 단계·성능 측정 계획이다.
기본·최소·넓은 창에서 Core의 두께·가림·원근과 주변 UI의 일관된 광원 반응을 실제 패키징 앱으로 확인해야 한다.
단순 회전 구체, 원자 모형 링, 강한 Glow 또는 2D 영상 배경만으로 최종 품질을 통과시키지 않는다.

초기 예산은 위의 draw call/DPR/삼각형 제안과 RFC의 visible p95 frame time ≤33ms,
정적 fallback 대비 추가 RSS ≤150MiB를 함께 평가한다. 60fps는 상위 품질 목표이며 30fps 모드와 구분한다.
숨김·최소화 중 지속 렌더 frame 0, reduced-motion에서 지속 회전·카메라 이동·맥동 0을 검증한다.
GPU/CPU 시간과 에너지·배터리 변화는 동일 장치·전원·밝기·콘텐츠에서 정적/3D/경량 모드를 비교한다.
GPU·배터리 수치 예산은 아직 미확정이며 P1.2 측정 계획 승인 때 정한다. P0 E5b Runtime 결과와 섞지 않는다.
가독성은 각 상태의 가장 밝은 프레임에서도 텍스트·Focus 대비, 키보드 순서, 입력 및 상태 접근성을 유지하는지 검사한다.
P6는 통합을 처음 시도하는 단계가 아니라 P1.2에서 검증한 공간·조명 체계의 최종 품질과 성능을 다듬는 단계다.
사용자 콘셉트 선택, 실제 앱 시각 승인, 최신 HEAD 병합 승인은 각각 별도이며 자동 승인은 없다.

## 검증 기록

검증일: 2026-11-12. Apple Silicon macOS 26, 패키징된 arm64 앱의 WKWebView에서 확인했다.
CI governance는 실제 macOS 앱의 검증이나 사용자의 시각 승인을 대신하지 않는다.


### 패키지와 검증 SHA

- 최종 앱 소스: `b467b399c072bd21600b4eef8217e58b17347ee1`.
- 실행 파일 SHA-256: `f42778e57ccf7415fb481b0ac69728199ccc66b100e9e411ab319d81a2c985a0`.
- worktree 기준 앱 경로: `target/p1-ui-issue-28/release/bundle/macos/AETHER.app`.
- `file .../Contents/MacOS/aether-desktop`: Mach-O 64-bit executable arm64.
- 로컬 unsigned 검증 패키지다. 배포 서명·공증·다른 Mac 실행 검증은 NOT_RUN.
- 이후 문서·화면 증거 커밋은 앱 소스를 바꾸지 않는다. PR HEAD CI와 실제 앱의 소스 SHA를 구분한다.

### 명령과 결과

모든 명령은 UI worktree에서 실행했다. 기존 의존성만 사용했다.

| 검사 | 결과 | 증거 / 한계 |
|---|---|---|
| `npm run build --prefix apps/desktop` | PASS | TypeScript, Vite, sidecar build. 최종 JS 247.40 kB / gzip 77.30 kB |
| `PATH="$HOME/.cargo/bin:$PATH" CARGO_TARGET_DIR="$PWD/target/p1-ui-issue-28" cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --offline` | PASS | Rust 19 tests. 이후 변경은 React/CSS이며 Rust 입력은 동일 |
| desktop 디렉터리에서 `node --experimental-strip-types --test tests/fake-provider.test.mjs tests/sidecar-protocol.test.mjs` | PASS | Node 8 tests. 이후 변경은 React/CSS이며 Node 입력은 동일 |
| `PATH="$HOME/.cargo/bin:$PATH" CARGO_TARGET_DIR="$PWD/target/p1-ui-issue-28" CARGO_NET_OFFLINE=true npm --prefix apps/desktop run tauri -- build --bundles app --no-sign -- --locked --offline` | PASS | 최종 소스 arm64 패키지 116.43 MiB, 종료 코드 0 |
| `git diff --check` / `python3 scripts/validate_governance.py` | PASS | whitespace / governance 21 artifacts |
| GPU·에너지 정량 측정 / E5b 반복 측정 | NOT_RUN | 이번 GUI 검증은 Runtime benchmark 결과가 아님 |
| VoiceOver 실제 음성 낭독 전체 검사 | NOT_RUN | native AX tree와 키보드 조작을 검사했으며 음성 낭독 인증을 주장하지 않음 |
| 최종 사용자 시각 승인 / merge | NOT_RUN | 사용자 승인 대기. Draft 유지 |

### 실제 앱 회귀와 접근성

| 항목 | 결과 | 관찰 |
|---|---|---|
| 네 화면, 900×760 / 400×680 / 1200×800 | PASS | 세 크기의 전체 창 캡처, 본문 스크롤, 가로 잘림 없음. Main의 상태와 Dock 동시 표시 |
| Context / 상태 상세 안전 영역 | PASS | 모든 크기 기본 접힘. Scene 영역에 한정, Dock과 분리. 열기·닫기·Escape·포커스 복귀 확인 |
| 키보드 | PASS | Tab / Return 탐색, 제목 포커스, textarea 입력, Motion Space, Context Escape 및 토글로 포커스 복귀, 탐색 레일 키보드 닫기 |
| 페이지 전환 AX 본문 | PASS | Command 경유 후 Memory·Settings 본문과 조작 요소 모두 노출. 실행 중 전환에서도 유지 |
| 로컬 초안 | PASS | 한국어 입력 후 네 화면 이동·resize 후 유지. reload 시 초기화. 전송 기능은 disabled |
| macOS reduced-motion | PASS | 시스템 동작 줄이기 off → on 시 앱 정책 적용. 검사 후 off로 원복 |
| 앱 숨김 / 최소화 | PASS | 선택형 Core preview 동작 허용 → 문서 숨김 정책 전환 → 복원. CSS pause 정책 확인. GPU 계측 결과는 아님 |
| native 창 조작 | PASS | resize, zoom, 최소화, Raise 복원, Quit. native titlebar 유지 |
| Core 점등 | PASS | Diagnostics에서 점등 후 버튼 비활성·점등됨 표시. 기존 2D는 선택형 Preview |
| macOS / Accessibility 조회 | PASS | macOS 26 / 권한 없음 응답. 권한을 요청하거나 확대하지 않음 |
| E3b counter | PASS | native sheet 취소 후 0 유지, 시험 승인 후 1. 자동화 시험이며 사용자 디자인·병합 승인이 아님 |
| WebView A / Node B 완료 | PASS | 각각 succeeded, seq 5, terminal 1. Node exit 0 / reaped |
| WebView A / Node B 취소 | PASS | 25% 대기 후 cancelled, seq 3, terminal 1. Node reaped |
| Runtime session / 오류 / reload | PASS | A 실행 중 기존 Lifecycle 메뉴 reload → 동일 instance, context 1→2, failed·terminal 1 표시 |
| Quit / Node 수명 | PASS | Node 25% 대기 상태에서 앱 Quit, 기록한 앱·Node 자식 모두 종료 확인 |

위 앱 회귀는 최종 `b467b39` 패키지에서 재확인했다. Rust 19개·Node 8개 테스트는 `a5de7df`에서 실행했고,
후속 `b467b39` 변경은 Main 카피와 CSS 제거뿐으로 해당 테스트 입력은 동일하다.
GUI 도구의 창 복원 후 조작이 중단되어 세션을 reset한 뒤 화면 이동·resize를 재검증했다.
시스템 동작 줄이기는 off→on→off로 복원했다. 설정 앱 화면이나 개인 계정 정보는 캡처 자료에 포함하지 않는다.

### 실제 화면과 비교

아래는 생성 콘셉트가 아닌 실행 중인 패키징 앱의 창 캡처다. 원본 JPEG를 편집하지 않았다.
상단 보라색 화면 공유 표시는 macOS 캡처 세션 표시이며 AETHER UI가 아니다.

| 화면 | 기본 900×760 | 최소 400×680 | 넓은 1200×800 |
|---|---|---|---|
| Main | [기본](assets/p1-ui-shell/main-900.jpg) | [최소·입력 Focus](assets/p1-ui-shell/main-400.jpg) | [넓은 창](assets/p1-ui-shell/main-wide.jpg) |
| Command Center | [기본](assets/p1-ui-shell/command-900.jpg) | [최소](assets/p1-ui-shell/command-400.jpg) | [넓은 창](assets/p1-ui-shell/command-wide.jpg) |
| Memory Center | [기본](assets/p1-ui-shell/memory-900.jpg) | [최소](assets/p1-ui-shell/memory-400.jpg) | [넓은 창](assets/p1-ui-shell/memory-wide.jpg) |
| Settings | [기본](assets/p1-ui-shell/settings-900.jpg) | [최소](assets/p1-ui-shell/settings-400.jpg) | [넓은 창](assets/p1-ui-shell/settings-wide.jpg) |

[최소 창 Context](assets/p1-ui-shell/context-400.jpg),
[기본 창 Context](assets/p1-ui-shell/context-900.jpg),
[기본 창 상태 상세](assets/p1-ui-shell/status-900.jpg),
[최소 창 상태 상세](assets/p1-ui-shell/status-400.jpg),
[탐색 펼침](assets/p1-ui-shell/navigation-expanded.jpg),
[탐색 키보드 Focus](assets/p1-ui-shell/navigation-focus.jpg),
[최소 창 하단·Focus](assets/p1-ui-shell/settings-400-bottom.jpg),
[키보드 Motion](assets/p1-ui-shell/keyboard-motion.jpg),
[시스템 reduced-motion](assets/p1-ui-shell/system-reduced-motion.jpg),
[reload 오류와 원장](assets/p1-ui-shell/runtime-error-reload.jpg).
비교용 before 파일을 제외한 최종 화면은 모두 `b467b39` 패키지다.
Retina 캡처는 논리 창 크기의 2배 픽셀이다.

[이전 Reactor Main](assets/p1-ui-shell/before-reactor.jpg)은 이전 `b2c6c47` 시안의 비교 자료이며 최종 결과가 아니다.
[직전 Shell Main](assets/p1-ui-shell/before-immersive.jpg)은 `a52a75f`의 비교 자료다.
이전 Reactor는 SVG가 중심이었고, 직전 Shell은 큰 제목·상태 카드·상시 Context가 Scene을 나눴다.
새 화면은 숨겨진 화면 제목, 작은 연결 버튼, 기본 접힌 Context, 얇은 탐색 레일, 실제 textarea Dock으로
Scene 공간을 확보한다. 금속 경계·유리 소재는 정적 레이어로 통일했다. Main 슬로건은 대체 문구 없이 제거했다.
현재 Main의 빈 Scene은 의도적인 구현 경계다. 이 화면을 레퍼런스의 최종 3D 시각 품질을 달성한 결과로 평가하지 않는다.

### 보안과 프로세스 정리

Rust/Swift/Node runtime, IPC, capability, CSP, Node pin, manifest/lockfile, Benchmark Harness 변경 없음.
새 UI 의존성·외부 폰트·원격 이미지·데이터 수집 없음. 참조 이미지 자체는 앱·저장소에 삽입하지 않았다.
공유 Git metadata 외 P0 E5b worktree·계측·결과를 변경하지 않았다.

2026-11-12 15:44 KST 프로세스 대조 결과:

| 항목 | 결과 | 확인 |
|---|---|---|
| P1 패키징 앱 | PASS | PID 50197, 시작 15:31:14, UI worktree bundle의 aether-desktop → Quit 후 없음 |
| P1 Node 자식 | PASS | PID 52898, 시작 15:41:22, PPID 50197, 같은 bundle의 Resources/sidecar/node → Quit 후 없음 |
| P1 GUI 작업자 | PASS | PID 52778/52779, 시작 15:40:18, GUI 세션 reset 후 부모 52776/52777 포함 없음 |
| 빌드·테스트 | PASS | 실행 세션 전부 종료, worktree 실행 경로·cwd 대조에서 잔존 빌드/시험 없음 |
| 사용자·P0 프로세스 | PASS | 종료 명령을 보내지 않음. 공유 도구 runtime과 현재 Codex 에이전트는 별도 소유권으로 보존 |

P1_GUI_IDLE_READY_FOR_E5B

이후 P1 빌드·앱·GUI 실행을 중지했다. E5b의 재개를 자동으로 실행하지 않는다.
시각 승인 후에도 최신 PR HEAD에 대한 별도 squash merge 승인이 필요하다. P1.2는 시작하지 않았다.
