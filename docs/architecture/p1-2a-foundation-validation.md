# P1.2a Scene Foundation 구현·검증

Refs #32. PR #33. 기준 main `69f6ee1e6d203d253cd7e6c547a8f837d2703feb`.
2026-11-12 Apple Silicon macOS 26 / 실제 arm64 WKWebView에서 검증한다.
검증 Mesh는 Foundation Preview이며 최종 Orbital Intelligence, Neural Plasma, Bloom, Chamber 품질이 아니다.

## 구현 경계

- `SceneHost.tsx`: Main에서만 canvas를 소유. React effect cleanup에서 observer와 Renderer 해제.
  identity/epoch·품질·Camera·진단 재생성 때 새 generation. state/motion 변경만으로 context를 다시 만들지 않는다.
- `renderer.ts`: 단일 WebGL2 context, 4개의 불투명 Mesh, perspective camera, hemisphere/key/rim 기본광.
  linear 계산 → ACES tone mapping → sRGB 출력. 외부 자산/texture/render target/postprocessing 없음.
- `capabilities.ts`: WebGL2 shader compile/link probe와 의도적 fragment shader 실패. 실패 자원은 finally에서 삭제.
- `framing.ts`: 실제 Scene surface의 CSS 크기에 bounding sphere를 맞춘다. Dock 60~120px 및 안내 영역은
  DOM grid가 먼저 확보하고 남은 rect를 ResizeObserver로 전달한다. Context/Status/펼친 Nav exclusion 반영.
  최소 창에서 패널/탐색이 Scene을 가리면 canvas를 숨기고 연속 렌더를 중단한다.
- `useVisualState.ts` / `visual-state.ts`: Shell에서 기존 읽기 전용 get_runtime_session을 확인한다.
  instance/epoch 불일치, 중복·역순 sequence, 알 수 없는 source/phase, actual thinking 등 가상 실제 상태를 거부한다.
  이전 async session 결과는 generation으로 무효화한다. Renderer는 IPC/승인/음성/컴퓨터 제어를 호출하지 않는다.
- source actual/preview와 AI disconnected는 분리된다. 점등은 local visual이며 연결 성공을 만들지 않는다.
  Renderer health도 Runtime 오류와 별개다. GPU 오류 시 DOM fallback·Navigation·로컬 초안은 유지한다.
- hide/blur/Main 이탈은 RAF 취소. Motion OFF/reduced-motion은 상태·resize·복귀 때 단발 렌더만 허용.
  stale resize/restore callback은 disposed/mounted guard로 무효화한다. timer/polling/자동 복원 loop 없음.
- context loss 주입은 WEBGL_lose_context로 자기 canvas에만 수행한다. restore는 새 SceneHost generation으로 교체.
  cleanup은 RAF·observer·listener·geometry·material·Renderer를 해제하고 자기 context를 종료한다.
  live count는 앱 소유 자원 수이며 실제 VRAM/RSS 해제 증거와 동일시하지 않는다.

## 의존성 도입

사용자가 전이 prepare 발견 후 공식 registry exact tarball과 모든 설치 `--ignore-scripts` 조건으로 재개를 승인했다.
`npm install --ignore-scripts --no-audit --no-fund`를 사용했다. verbose log에 lifecycle run 항목 없음.
검토한 lockfile과 설치 후 전체 lockfile JSON이 일치하고 기존 package entry 변경은 0개다.
각 추가 tarball SHA-512 integrity, 실제 설치 버전, license 파일을 대조했다.

| 패키지 | 버전 | 역할 / 라이선스 |
|---|---|---|
| three | 0.187.0 | runtime / MIT |
| @types/three | 0.187.0 | dev / MIT |
| @dimforge/rapier3d-compat | 0.12.0 | dev 전이 / Apache-2.0 |
| @tweenjs/tween.js | 23.1.3 | dev 전이 / MIT; prepare 미실행 |
| @types/stats.js | 0.17.4 | dev 전이 / MIT |
| @types/webxr | 0.5.24 | dev 전이 / MIT |
| fflate | 0.8.3 | dev 전이 / MIT |
| meshoptimizer | 1.1.1 | dev 전이 / MIT |

OSV exact version 8개 조회에서 advisory 미반환. 안전성 보장은 아니다.
`public/THIRD_PARTY_NOTICES.txt`를 Vite 로컬 배포 자산에 포함한다. 타입 전이 패키지는 runtime import하지 않는다.
Tauri native binding, Node pin, Rust/Swift/IPC/CSP/capability와 E5 계측 소스는 변경하지 않았다.
설치 hook 우회 실행, npm rebuild, force, legacy-peer-deps는 사용하지 않았다.
프로젝트 build/typecheck/sidecar build는 사용자가 명시적으로 허용한 명령이다.

## 로컬 및 실제 앱 검증

아래는 앱 source `4153e67a8e16c87bbd1a616866d4e896e144da6d`의 기존 검증이다.
뒤의 병합 전 리뷰 보완은 문서·테스트만 변경하며 새 GUI 실행 결과로 이 표를 승계하지 않는다.

| 검사 | 결과 / 근거 |
|---|---|
| TypeScript 7 / Vite 8 / React 19 호환 | PASS: npm run typecheck 및 production build. Three 포함 JS 약 791kB, 500kB chunk 경고는 남음 |
| 순수 Scene 계약 | PASS: Node 5 tests; freshness·invalid source/phase·camera sphere fit·motion policy. Generation 도우미 검사는 실제 SceneHost 비동기 수명 검증이 아님 |
| 기존 Node | PASS: 8 tests; 실제 pinned sidecar framing/취소/회복 포함 |
| Rust | PASS: 19 tests --locked --offline; 원장/승인/reload/종료/실제 Node 회귀 |
| arm64 package | PASS: 독립 target/p1-2a-issue-32, --no-sign app; 기존 P1.1 bundle 유지 |
| 실제 WebGL2 | PASS: WKWebView shader probe 및 Three material compile/render, 4 draws / 408 triangles |
| Shader 실패 | PASS: 의도적 실패 → DOM fallback. 로컬 다중행 초안 입력 유지 |
| GPU loss/restore | PASS: loss RAF 0, DOM fallback; restore 새 generation, live renderer 1 |
| Main↔Command Center 20회 | PASS: 매 이탈 live renderer/geometries/materials/textures/RAF 0, created=disposed |
| 900×760 / 400×680 / 1200×800 | PASS: 실제 창 JPEG 2x 크기 대조; 네 화면 AX 본문·스크롤·Main Dock 확인 |
| 400×680 최대 Dock | PASS: 입력 5행으로 최대 높이, 입력 내부 스크롤 및 3개 비활성 버튼 유지 |
| Camera -15° / +15°, High | PASS: 실제 앱 시점 변경 및 DPR 1.5; Balanced DPR 1.25 |
| Motion OFF | PASS: 활성 창에서 frames 1 / RAF 0, 반복 조회 시 동일 |
| 시스템 reduced-motion | PASS: OS 설정 ON → 정적 렌더, 앱 Settings 적용 표시. 원래 OFF로 복원 |
| Hide | PASS: 숨김 RAF 0; 복귀 전후 frame 증가가 수 초 동안 지속되지 않음 |
| Minimize | PASS: 최소화 중 frames 1208 / RAF 0 유지. 도구 복원 실패 뒤 사용자 Dock 복원으로 렌더/입력 재개 확인 |
| Focus/Esc/Navigation | PASS: Context·Status 닫기 Focus 복귀, Shift-Tab/Return 페이지 이동, Nav 펼침·Esc |
| FakeRun A/B | PASS: 각각 succeeded와 25% cancelled, Node reaped |
| Reload | PASS: 대기 중 A → failed terminal 1, context epoch 증가, 새 Scene 정상 |
| Native | PASS: macOS26·AX 권한 없음 조회, 진단 counter 승인 0→1 / 취소 후1. AI 자동화이며 사용자 수동 승인 검사 아님 |
| StrictMode 개발 setup/cleanup/setup | PASS: NODE_ENV=development 실제 패키지. Main 재진입 generation3 / created3 / disposed2 / live1; 이탈 disposed3 / live0. 최종 리뷰는 production으로 재패키징 |
| Quit | PASS: Cmd-Q 후 검증 앱 종료. 최종 패키지 QA 후 자기 PID·시작 시각·경로를 대조한 잔존 검사 결과는 최종 보고에 별도 기록 |

캡처/원본 진단은 ignored `target/p1-2a-issue-32/qa/`에 보관한다. 모든 이미지는 실제 패키징 앱이며 콘셉트 이미지가 아니다.
한 차례 최소화 복원에서 native GUI 도구 입력이 실패하고 Dock 도구가 timeout했다. 사용자 수동 복원 뒤
Renderer와 Context/Esc가 정상 동작했다. 자동 복원 경로 성공으로 기록하지 않는다.

## 성능·접근성 한계

- Balanced30/High60, DPR1.25/1.5, draw≤60, triangle≤150k 등의 설계 예산을 유지한다.
- draw4/triangle408은 실제 Renderer info. GUI 조작 중 짧은 표본에서 frame p95 약34ms,
  CPU submit p95 약1ms가 관찰됐다. warm-up/120초×3 고정 조건 시험이 아니므로 frame 예산 PASS로 판정하지 않는다.
- GPU time/귀속 가능한 WebKit RSS/실제 VRAM: UNAVAILABLE. 에너지 및 장시간 예산: NOT_RUN.
  현재 fallback 대비 RSS≤150MiB 및 전력 예산 통과를 주장하지 않는다.
- 실제 WebKit renderer process crash: NOT_RUN. 공유 WebKit process 종료 없음.
- VoiceOver 실제 낭독·타 Mac·전체 색 대비 정량 검사: NOT_RUN. AX 본문/키보드와 별개의 검사다.
- 최종 Neural Plasma 성능·최종 시각 품질은 P1.2b~e에서 별도 검증한다.

## 재현 명령

루트에서 P0 idle 인계와 측정 process 부재를 다시 확인한 후에만 수행한다.

```sh
npm --prefix apps/desktop run build --ignore-scripts
cd apps/desktop
node --experimental-strip-types --test tests/scene-foundation.test.mjs tests/fake-provider.test.mjs tests/sidecar-protocol.test.mjs
```

Rust: `CARGO_TARGET_DIR=<P1 target> cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --offline`.
패키징: `CARGO_TARGET_DIR=<P1 target> CARGO_NET_OFFLINE=true npm_config_ignore_scripts=true npm --prefix apps/desktop run tauri -- build --bundles app --no-sign -- --locked --offline`.

## 승인 경계

Draft PR #33 유지. 실제 앱 경로·source SHA·bundle hash·최신 CI는 PR/사용자 보고에 기록한다.
사용자 Foundation 시각 승인 및 최신 HEAD squash 승인은 각각 별도이며 아직 없다.
P1.2b / 최종 Orbital·Plasma·Bloom·통합 Chamber 구현은 시작하지 않는다.

## 병합 전 리뷰 보완 — 2026-11-12

### 검사 경계와 재현

`tests/scene-lifecycle.test.mjs`와 `tests/scene-lifecycle-harness.mjs`를 추가했다.
설치된 TypeScript로 **수정하지 않은 실제 SceneHost / Renderer / Visual Adapter 소스**를 임시 디렉터리에
컴파일하여 Node VM에서 실행한다. 임시 출력은 검사 후 삭제하며 앱 dist/bundle을 덮어쓰지 않는다.
새 의존성·설치·앱 코드 변경은 없다.

Three Geometry/Material은 실제 라이브러리 객체다. WebGLRenderer/GL/DOM/RAF/ResizeObserver와 React hooks는
제어 가능한 대역이다. **ReactDOM의 실제 mount, StrictMode scheduler, WKWebView 또는 GPU 검증이 아닌 계약 테스트**다.
기존 5개 순수 함수 테스트, 실제 앱 StrictMode 검증과 이 7개 계약 테스트를 별개 근거로 유지한다.

```sh
cd apps/desktop
node --experimental-vm-modules --experimental-strip-types --test tests/scene-lifecycle.test.mjs tests/scene-foundation.test.mjs tests/fake-provider.test.mjs tests/sidecar-protocol.test.mjs
npm run typecheck --ignore-scripts
```

### 결과와 한계

| 리뷰 항목 | 결과 / 구체적 검사 |
|---|---|
| 복원 이벤트 미도착 | PASS (계약): 명시적 restore 1회 뒤 resize/효과 flush 30회에도 lost, fallback, RAF0, canvas1. 자동 timer/retry 없음. unmount 후 소유 자원·listener·observer 해제 |
| 복원 후 재초기화 실패 | PASS (계약): restored event → 새 generation의 getContext null. unavailable, fallback, live renderer0/RAF0. 반복 callback에도 canvas2에서 추가 생성 없음 |
| unmount 이후 늦은 ResizeObserver | PASS (계약): disconnect 이전 보관한 실제 callback을 cleanup 뒤 직접 호출. resize/draw/report/state write 변화 없음 |
| dispose 이후 늦은 restore event | PASS (계약): 제거 전 보관한 실제 event handler를 cleanup 뒤 호출. 재생성·report·state write 없음 |
| setup/cleanup/setup stale callback | PASS (계약): 효과 순서 재현 후 이전 resize/restore/RAF 호출. 새 generation과 report 유지, live1/RAF1, 최대 동시 renderer/RAF 각1 |
| 복원 뒤 이전 generation | PASS (계약): 정상 교체 후 이전 resize/restore callback이 새 Scene을 덮지 못함 |
| 이전 session Promise | PASS (계약): 실제 useVisualState hook 소스의 새 setup 응답을 먼저 완료하고 이전 응답을 뒤에 완료. instance/epoch 유지. unmount 뒤 응답도 state write 없음 |
| Runtime·승인 경계 | PASS (소스/계약 범위): renderer/SceneHost 검사 중 IPC 요청0, snapshot 불변. Adapter의 허용 호출은 get_runtime_session뿐. 실제 원장 상태의 복원 실패 전후 비교는 NOT_RUN |
| 실제 WKWebView 복원 실패 | NOT_RUN: 기존 자체 canvas 진단은 정상 restore와 독립 초기화 실패를 지원하지만 복원 후 생성만 실패시키는 주입 지점은 없음. 이번에는 배포 코드·native 권한·CSP 변경 없이 위 경계에서 주입. 기존 정상 restore/Shader 실패 GUI 결과를 복원 실패 성공으로 대체하지 않음 |
| 복원 실패 중 실제 Navigation/Dock | NOT_RUN: SceneHost 계약 테스트는 Shell Navigation/Dock을 실행하지 않음. 기존 Shader 실패에서 초안·탐색 확인은 별도 역사적 근거일 뿐 |
| 실제 WebKit process crash / GPU 드라이버 자원 해제 | NOT_RUN / UNAVAILABLE: 공유 process 종료 없음. 객체 dispose 확인은 VRAM/RSS 증명이 아님 |

최종 결과: Node 20 tests PASS (기존8 + 순수 Scene5 + 새 수명 계약7), TypeScript typecheck PASS.
Rust19/production build/arm64 GUI는 위 기존 source에서의 PASS를 보존하며 이번에는 재실행하지 않았다.
앱 소스·의존성·native·sidecar·빌드 설정이 그대로이므로 재패키징은 하지 않는다.
Governance CI는 문서 정책 validator와 commit whitespace만 실행한다. TypeScript/Rust/Node/GUI 성공 근거가 아니다.

### 앱 provenance와 보존

- 검증된 앱 빌드 source: `4153e67a8e16c87bbd1a616866d4e896e144da6d` (리뷰 보완 커밋과 구분).
- 앱: worktree 기준 `target/p1-2a-issue-32/release/bundle/macos/AETHER.app`.
- bundle manifest SHA-256: `d52af0ce87f022092636a21b550395469ff043aed83fc3f46d177e28ad2bf756`.
- 전체 bundle 파일을 다시 읽어 SHA-256 + 상대 경로로 정렬한 manifest와 기존 manifest가 일치하는지 확인한다.
- 리뷰 보완 diff는 위 테스트 2개와 이 문서뿐이다. `apps/desktop/src`, package/lockfile, native/sidecar,
  빌드 설정 및 scripts를 포함한 기존 tracked 파일 입력은 동일하다. 최신 HEAD에서 앱을 새로 빌드했다고 주장하지 않는다.
- 기존 캡처 `target/p1-2a-issue-32/qa/4153e67/` 보존. 새 계약 테스트는 실제 화면 캡처를 만들지 않는다.
- P0 Worktree·PR #34·Issue #27·측정 결과 변경 없음. 앱/GPU 실행·의존성 설치 없음.
- 장시간 성능·전력·VoiceOver는 NOT_RUN, GPU timer/귀속 RSS/VRAM은 UNAVAILABLE로 유지한다.
- 사용자 실제 앱 시각 승인 및 최신 HEAD squash merge 승인은 미완료. PR #33 Draft 유지.
