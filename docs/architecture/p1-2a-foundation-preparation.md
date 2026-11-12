# P1.2a Scene Foundation 착수 준비

Refs #32. 기준 main `69f6ee1e6d203d253cd7e6c547a8f837d2703feb`. 2026-11-12 KST.
선행 #29 / #31 MERGED, #30 CLOSED. [승인 설계](p1-2-spatial-interface-design.md)와
[PR #31 추가 승인 조건](https://github.com/Kim-Ji-Won/AETHER/pull/31)을 적용한다.

## 실행 게이트

E5b 실제 측정 상태는 **UNKNOWN**이다. #27 OPEN 여부나 process 부재로 측정 종료를 추정하지 않는다.
사용자가 실측 완료 또는 명시적 중단과 P1 전용 시간을 확인하기 전 설치·빌드·앱·GUI·GPU 시험은 BLOCKED다.
현재는 분석·의존성 검토·Issue·준비 문서만 작성한다. 준비 완료 뒤 구현을 자동 시작하지 않는다.
이번 브랜치는 `feat/32-scene-foundation`, 현재 diff는 이 문서 하나뿐이다.
기존 UI worktree를 재사용하되 P1.1/설계 브랜치·앱·캡처·E4/E5a 산출물은 보존한다.
P0 worktree·프로세스·측정 환경·결과에 접근해 조작하거나 정리하지 않는다.

## 의존성 사전 검토

| 후보 | 고정 버전 | 라이선스 | 조회 결과 |
|---|---|---|---|
| three | 0.187.0 | MIT | runtime dependencies 없음, install lifecycle hook 없음, deprecated 필드 없음 |
| @types/three | 0.187.0 | MIT | dev 후보, scripts 없음, 전이 dependency 6개 |

공식 npm exact-version metadata를 설치 없이 HTTPS로 읽었다.
OSV querybatch에 npm ecosystem과 위 exact version 두 개를 전달한 결과 `results: [{}, {}]`였다.
이는 **조회 시 알려진 직접 패키지 advisory 미반환**이며 안전성 보장·전체 감사·설치 승인이 아니다.
검색 결과만으로 취약점 부재를 판단하지 않았다.

`@types/three`의 전이 범위는 fflate `~0.8.3`, @types/webxr `>=0.5.17`, meshoptimizer `~1.1.1`,
@types/stats.js `*`, @tweenjs/tween.js `~23.1.3`, @dimforge/rapier3d-compat `~0.12.0`이다.
실제 해석 버전·그 하위 그래프·install scripts·advisory·라이선스·tarball integrity는 아직 미검증이다.
전용 시간과 도입 승인 후 제한된 dependency 변경으로 lockfile을 만들고 전체 그래프를 검토한다.
원치 않는 lifecycle script는 실행하지 않으며 기존 React/Vite/Tauri/Node pin 업그레이드는 포함하지 않는다.
MIT 고지는 배포물에 보존한다. 타입 패키지의 runtime import, 외부 자산/CDN, R3F/Drei/WebGPU 도입은 제외한다.
advisory 재조회·실제 번들 provenance/integrity 대조·기존 TS 호환성을 도입 전후 게이트로 둔다.

근거: [three metadata](https://registry.npmjs.org/three/0.187.0),
[types metadata](https://registry.npmjs.org/@types%2fthree/0.187.0),
[OSV query API](https://google.github.io/osv.dev/api/#tag/api/operation/OSV_QueryBatch),
[Three WebGLRenderer](https://threejs.org/docs/pages/WebGLRenderer.html),
[자원 해제](https://threejs.org/manual/pages/how-to-dispose-of-objects.html).

## 구현 예정 경계 — 지금 코드 생성 없음

| 파일/영역 | 책임 |
|---|---|
| `apps/desktop/src/scene/SceneHost.tsx` | React mount/unmount, resize와 외부 상태 전달; canvas/DOM fallback 경계 |
| `scene/renderer.ts` | 단일 WebGL2 context, scheduler, init/loss/restore/dispose; Runtime invoke 금지 |
| `scene/capabilities.ts` | context·필요 target·shader 기능 확인, 실패 원인 분류 |
| `scene/framing.ts` | CSS pixel safeRect와 Camera projection, Nav/Dock/패널 exclusion |
| `scene/visual-state.ts` | 읽기 전용 snapshot 검증과 freshness; 실제/Preview·renderer health 분리 |
| `Shell.tsx` / `useMotion.ts` / 필요한 CSS | Main Scene visibility·안전 영역 연결. 기존 FakeRun 마운트·AX 순서·초안 유지 |
| 기존 package manifest/lockfile | 승인한 두 exact dependency와 필요한 라이선스 기록만 변경 |
| 순수 상태/프레이밍/수명 테스트 | 오래된 상태·비동기 콜백·dispose 및 resize 계약 검사. GPU 실환경 결과와 구분 |

검증용 mesh는 깊이가 다른 불투명 도형과 기준 바닥, 고정 정면/±15° Camera로 구성한다.
실제 가림·원근·앞뒤 두께를 확인하고 `Foundation Preview`로 명시한다.
최종 Plasma/Orbital/Bloom/금속·유리 통합 품질, 실제 음성 입력, AI 연결, 컴퓨터 제어는 a의 완료 범위가 아니다.
단일 canvas의 후경이 Nav 아래까지 확장 가능한 구조를 확인하되 주변 UI 전체 재디자인은 하지 않는다.

## 최신성 계약

현재 Rust `lifecycle::session()`은 instance / epoch / intake 가능 여부를 읽기 전용으로 제공하며,
FakeRun은 이 값을 기존 명령 envelope에 사용한다. Renderer는 이 envelope를 만들거나 명령을 실행하지 않는다.

1. Shell 쪽 adapter에서 기존 read-only session 조회를 사용한다. Renderer 내부에 IPC·polling을 넣지 않는다.
2. snapshot은 `{instance, contextEpoch, visualSequence, source, phase, connection, motionPolicy}`를 가진다.
   instance/epoch는 adapter가 임의 생성하지 않는다. session 미확인/closing이면 actual snapshot 수용을 중지하고 정적 fallback.
3. 같은 instance/epoch에서 visualSequence는 단조 증가한다. 중복/역순은 거부하고 sequence를 Rust run.seq와 혼용하지 않는다.
4. 다른 instance/epoch는 들어온 snapshot만 보고 채택하지 않는다. 소유자의 새 session 확인·명시적 reset을 통해서만 전환한다.
   이전 session의 Promise, shader 완료·resize callback이 늦게 와도 새 상태를 덮지 않는다.
5. 별도 rendererGeneration을 mount·restore 재생성마다 증가시킨다. 모든 async callback은 캡처한 generation과
   현재 disposed 여부를 검사한다. GPU context restore를 WebView epoch 변경으로 취급하지 않는다.
6. visual sequence는 Shell adapter 범위에 유지하고 Main 재마운트만으로 초기화하지 않는다.
   session을 새로 확인해 바꿀 때만 reset한다. StrictMode의 첫 setup 결과가 두 번째 setup에 유입되지 않게 한다.
7. Preview는 실제 연결/승인 상태를 덮지 않는다. AI 실제 상태 기본은 offline, 음성 level은 null.
   session freshness 문제와 renderer 오류는 별도 표시하며 Runtime 결과를 변경하지 않는다.

검사 사례: A snapshot 뒤 늦은 이전 A, instance B 전환 뒤 A, epoch+1 뒤 이전 epoch,
동일 sequence 재전송, generation+1 뒤 오래된 compile 완료, unmount 뒤 resize/restore event, session 조회 실패/closing.
각각 거부·단발 초기화·fallback 여부를 순수 테스트와 실제 reload/context loss에서 분리 확인한다.
instance는 인증 토큰이 아니며 이 UI 검사는 Rust 승인·보안 검사를 대체하지 않는다.

## 허용 후 구현·검증 순서

1. **시작 확인:** E5b 종료/명시적 중단과 전용 시간, 사용자 설치·구현 재개 지시를 확보한다.
   새 main drift·미커밋 작업·기존 의존성 및 advisory를 다시 확인한다.
2. **의존성:** exact pin/lockfile·전이 공급망 검토. TS build와 현재 CSP에서 동작 여부를 확인한다.
3. **Foundation:** SceneHost·정적 fallback·capability·기본 camera·freshness·dispose 순으로 구현한다.
   상태/카메라 순수 테스트를 통과한 뒤 실제 GPU 결과를 확인한다.
4. **실제 arm64 WKWebView:** WebGL2 생성, 실제 shader compile/link와 의도적 오류, 필요한 render target 검증.
   WebGL2 실패는 DOM fallback, HDR target 미지원은 LDR 가능성만 기록한다. a에서 Bloom은 구현하지 않는다.
5. **장애/수명:** StrictMode setup/cleanup/setup, Main↔타 화면 20회, reload/Quit.
   지원되는 context-loss extension으로 자기 canvas loss/restore를 주입하고, 미지원이면 해당 시험 NOT_RUN.
   context-loss 모의시험은 실제 WebKit process crash PASS가 아니다. 공유 WebKit PID 종료 금지.
6. **회귀/시각:** 네 화면·세 크기·Dock 최대 높이·Nav 펼침·Context/상태·키보드/Esc/Focus/AX,
   Core 점등·macOS/AX 조회·E3b 승인·A/B 완료/취소/오류·session·Quit 및 Rust/Node 회귀.
7. **리뷰:** source SHA·bundle hash·실제 스크린샷·앱 경로·지표별 상태 제공. 자기 process 정리 후 빌드/GUI 중단.
   사용자의 Foundation 공간/동작 시각 승인과 최신 HEAD squash 승인을 별도로 받는다.

실패 시 Scene을 해제하고 기존 DOM Shell·입력 초안·상태·Diagnostics를 유지한다.
자동 재시도/복원 loop는 금지한다. Runtime 원장·승인·Node lifecycle은 기존 소유권을 유지한다.

## 화면·Motion·자원 합격 기준

- 900×760 / 400×680 / 1200×800에서 실제 Scene rect를 측정한다. 기본60px/최대120px Dock과
  안내문을 먼저 확보하고 Camera를 재구도한다. Nav/패널이 열려도 Dock 버튼·입력·Focus가 잘리지 않는다.
- 단일 renderer와 loop만 허용. hide/minimize/blur/Main 이탈은 RAF 취소, 지속 render/시각 timer 0.
  reduced-motion/사용자 Motion OFF는 정적 표현과 상태/resize 단발 render만 허용한다.
  숨김 중 state 이벤트도 frame을 예약하지 않고 복귀 시 최신 snapshot만 반영한다.
- cleanup: token 무효화→RAF 취소→listener/observer/구독 해제→mesh geometry/material/texture/target/pass dispose→
  renderer.dispose→참조 제거. renderer.dispose만으로 모든 앱 소유 자원이 해제됐다고 판단하지 않는다.
- 반복 mount/unmount 후 app-owned live 자원 수 기준선 복귀와 중복 callback 부재를 검사한다.
  renderer.info의 object count는 VRAM byte나 RSS가 아니다. driver 지연 해제와 지속 증가를 구분한다.
- canvas aria-hidden과 DOM fallback/status 이름, page 전환 AX 본문, native titlebar/창 조작을 유지한다.

## 성능 측정과 한계

설계 예산 유지: Balanced30/High60, DPR≤1.25/1.5, 총 draw≤60, scene triangle≤150k,
동일 Shell 정적 대비 귀속 가능한 RSS 증가≤150MiB. 전부 **실측 전 제안값**이다.
검증용 mesh의 성공은 최종 Plasma 부하에서도 통과한다는 증거가 아니다.

| 지표 | 수집/판정 계획 |
|---|---|
| frame interval | 실제 render 시각 간격 p95, n·품질·창 크기·warm-up 분리. 화면 presentation latency와 동일시하지 않음 |
| CPU update/submit | JS monotonic 구간, GPU time과 별도 |
| GPU time | 지원 timer query의 valid/disjoint 확인. 미지원/귀속 불가 UNAVAILABLE |
| RSS | Host/WebKit/Node를 구분하고 확인된 자기 process만 집계. WebKit 미귀속이면 총 RSS PASS 금지 |
| GPU 자원 | 소유 object count·할당 추정치와 실제 GPU memory를 구분 |
| 에너지 | a에서 측정하지 않으면 NOT_RUN. 지원/권한·귀속 문제면 UNAVAILABLE. 단기 %를 효율로 환산 금지 |

P0 Harness/측정 파일에 그래픽 표본을 추가하지 않는다. E5b가 재개되면 P1 실행을 중단한다.
향후 P1 target은 `target/p1-2a-issue-32`로 분리하는 계획이다. P1.1 bundle을 덮어쓰지 않는다.
PID·시작 시각·경로·부모 관계를 대조한 자기 process만 정리하고 실제 확인 뒤에만 GUI idle 완료를 선언한다.

## Governance / 현재 결과

Issue #32의 준비 문서로 Draft PR을 사용한다. 현재는 문서만 커밋하며 준비 보고 후 중단한다.
미래 구현을 같은 PR에서 계속하면 본문을 최종 구현 범위로 다시 작성하고 문서 검사를 GUI 검증으로 승계하지 않는다.
Closes/Ready/merge는 단계 완료 및 사용자 별도 승인 이후다. P1.2b는 자동 착수하지 않는다.

- PASS: 기준 main/중복 작업·소스·설계/정확한 직접 의존성 metadata 및 OSV 조회.
- BLOCKED: 전용 시간 미확인 상태의 설치·구현 재개·빌드·앱/GUI/GPU 시험.
- NOT_RUN: 실제 설치·lockfile 전체 공급망 감사·호환성·3D·수명·성능·실제 앱 회귀·시각 승인.
- 기존 P1.1 앱·브랜치·결과 보존. P0 상태/프로세스/결과 조작 없음.

```text
P1_2A_FOUNDATION_PREPARATION_READY
P1_2_DESIGN_APPROVED: YES
THREE_JS_INSTALLATION_COMPLETED: NO
P1_2A_GUI_TESTED: NO
FINAL_AETHER_VISUAL_APPROVED: NO
MAIN_MERGED: NO
```
