# Spatial Presence Main — P1.3b

Issue #42 · 디자인 기준 Issue #41 · 기준 main `13cc5e3cfec511affa1642cbb43227e055954afc`.

## 범위와 승인

사용자는 색·소재 개선판 A를 구현 기준으로 수용했다. 실제 실행 화면의 시각 승인, Ready 및 merge 승인은 별도다. AI/음성/기억/컴퓨터 제어를 연결하지 않는다. #38의 이전 Neural 작업과 #39 Visual Lab은 보존한다.

## 구성과 소유권

- `Shell`은 탐색, 로컬 초안, Context/Status, 명시적 Preview 선택을 소유한다. Runtime Console은 화면을 이동해도 mount를 유지한다.
- `MainParticleHost`는 Main에서만 단일 canvas/Renderer를 소유한다. Lab과 Main은 같은 `createParticleRenderer`, seed 3901, 입자 데이터, Shader, Motion, Z 전방 이동 계산을 사용한다. 기존 Neural Scene은 Main에서 mount하지 않는다.
- phase 변경은 Renderer를 재생성하지 않는다. instance/epoch, 품질, 장애 옵션 또는 명시적 재생성만 소유권을 교체한다. cleanup은 observer, listener, RAF, buffer/material/renderer/context를 해제한다. 오래된 callback은 mounted/dead 경계로 차단한다.
- 기존 Lab 진입은 `VITE_AETHER_VISUAL_LAB=particle` 빌드 옵션을 유지한다. 기존 검증 Lab 앱을 덮어쓰지 않는다.

## actual / Preview 계약

`ParticleVisualAdapter`는 기존 VisualGate의 instance/contextEpoch/visualSequence 검증을 유지한다. actual은 offline/disconnected만 시각 idle로 해석한다. 실제 inputLevel/outputLevel은 null이며 합성 envelope를 실제 입력값으로 채우지 않는다. Preview는 idle/listening/thinking/speaking만 허용한다. 잘못된 source/phase/identity/sequence/audio 값은 거부한다.

Main은 기본적으로 로컬 시각 효과이며 AI 미연결이다. 명시적 Diagnostics에서 시작한 16초 합성 패턴에만 PREVIEW와 종료 버튼을 표시한다. 종료 시 같은 Renderer에서 idle로 감쇠한다. Motion OFF, reduced-motion, 숨김/비활성, 가용 영역 0에서는 지속 RAF를 정지한다. 정지 중 Preview 시계도 정지한다. 사용자는 종료 버튼이나 화면 이동으로 해제할 수 있다.

일반 Main은 프레임별 React 갱신을 하지 않는다. 보고는 ref로 전달하고 의미 있는 health 변화만 표시한다. Developer Diagnostics를 명시적으로 열었을 때만 해당 Host 안에서 최대 약 4Hz 지표를 표시한다. Command Center의 보고는 마지막 보고 시점의 스냅샷이다.

## 배경 합성

Main만 alpha canvas/transparent clear와 premultiplied material을 사용한다. 기존 tone mapping → output color 변환 뒤 alpha premultiply를 한 번 적용하고 additive 합성한다. Lab은 기존 opaque clear와 material 옵션을 유지한다. topology, vertex motion, seed, 입자 밝기·색 공식은 바꾸지 않았다. 배경은 DOM의 낮은 대비 남청색이며 별/이미지/원형 halo를 사용하지 않는다.

이는 목업의 lighten 합성을 복사한 결과가 아니다. 실제 WKWebView 화면에서 경계와 색감을 검토하며, 서로 다른 배경과 viewport를 가진 옛 Lab 캡처와 픽셀 동일성을 주장하지 않는다.

## 레이아웃과 소재

기본 64px 아이콘 레일, 열린 Scene, 기본 닫힌 패널, 독립 Dock 행을 유지한다. 700px 이하에서는 상단 메뉴로 전환한다. 메뉴/패널이 열린 작은 창은 Scene만 숨겨 RAF를 멈추며 Runtime과 입력은 유지한다. 넓은 창은 패널을 제외한 canvas 폭으로 Camera를 계산한다.

Dock은 60px 기본, 120px 상한이며 textarea는 40–100px 범위와 내부 스크롤을 갖는다. 배경 #06090F/#101B2C, 표면 #172635, accent #91DDF5, 본문 #E7EEF6, 보조 #A5B6C9를 사용한다. Dock 상단 중앙의 짧은 하이라이트와 어두운 아래 경계를 사용한다. backdrop blur는 Dock 12px/열린 패널 14px에 한정하고 미지원 환경에는 읽을 수 있는 기본 표면을 둔다. blur/compositor 비용은 GPU draw call과 별개다.

Selected/Hover/Focus를 구분하고 기존 접근 가능한 이름·숨긴 제목·Esc·닫기 후 Focus 복귀를 유지한다. 400px에서도 버튼을 축소해 공간을 확보하지 않는다. 실제 blur 성능, 장시간 배터리, VoiceOver 전체 품질은 별도 실측 항목이다.

## 보안 및 후속

새 의존성, native 권한, Rust/Swift/Node/IPC/CSP 변경 없음. Renderer는 Runtime/승인 명령을 소유하지 않는다. #35의 실제 복원 후 재초기화 실패 검증은 NOT_RUN을 유지한다. P0 E5b #27/#34 및 이전 앱/캡처는 별도 보존한다.

실제 검증 범위와 한계는 [검증 기록](../validation/spatial-presence-main.md)을 참조한다. 다음 단계는 실제 앱의 사용자 시각 리뷰이며 자동 merge나 실제 오디오 개발이 아니다.
