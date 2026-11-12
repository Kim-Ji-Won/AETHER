# RFC 001 — AETHER 통합 시스템 · System Architecture

| 항목 | 값 |
|---|---|
| 상태 | **PROPOSED** · DRAFT |
| 기준 | `main` · `9683f8a1154ef02f8b72b952db8cd5e20df73a50` |
| 작성·자료 확인 | 2026-11-11 |
| 작업 | [E4a Issue #15](https://github.com/Kim-Ji-Won/AETHER/issues/15), 선행 [E3b Issue #13](https://github.com/Kim-Ji-Won/AETHER/issues/13) / [PR #14](https://github.com/Kim-Ji-Won/AETHER/pull/14) |
| 후속 | [E4 Runtime 비교](e4-runtime-comparison-plan.md) · [P0~P7 로드맵](../engineering/development-roadmap.md) · [ADR 기준](../decisions/README.md) |

문서 제출·병합, 아키텍처 승인, 실험·구현 착수는 각각 별도 승인입니다. 아래 설계는 대안을 비교하기 위한 제안입니다. **OBSERVED**는 이 작업에서 읽은 소스·Git 상태, **DOCUMENTED**는 공식 자료 또는 출처가 있는 과거 실행 기록, **INFERRED**는 근거에서 도출한 해석, **UNKNOWN**은 확인하지 못한 사실, **BLOCKED**는 명시된 선행 조건 때문에 현재 진행할 수 없는 항목입니다. `PASS`와 `NOT_RUN`은 각 검사·대상 SHA에 귀속됩니다.

## 제품 목표와 출시 범위

Apple Silicon 우선의 설치 가능한 macOS AI Assistant를 목표로 합니다. 한국어 음성 대화, 권한과 작업별 승인에 따른 컴퓨터 제어, 사용자가 관리하는 기억, A+B 3D AI Core, 서명·공증한 직접 배포 경로를 포함합니다. ALPHA TWIN과 Axiom은 이 설계의 필수 의존성이 아닙니다.

| 단계 | 사용자 범위와 기능 | 출시 조건 |
|---|---|---|
| 개발용 실험 | 첫 창·로컬 번들, IPC와 승인 검증, 격리된 Fake Provider, 명시적 개발용 BYOK 후보 | 승인된 로컬 환경과 합성 입력. 각 결과는 해당 실험만 설명 |
| 첫 사용자용 버전 | 제한된 테스터에게 텍스트·Push-to-Talk, 선택적 로컬 기억, 첫 A+B Core 제공. 컴퓨터 제어는 독립 검증을 마친 좁은 도구만 노출 | 서명한 테스트 번들, 사용자 안내, 권한 거부·철회, 전송·삭제, 장애 시 중지 검증 |
| 정식 공개 버전 | 직접 설치·업데이트 가능한 Apple Silicon 앱과 지원 경로. 검증된 범위의 컴퓨터 제어와 기억 관리 | P7의 Developer ID, Hardened Runtime, 공증·stapling, 다른 Mac, 서명된 업데이트·복구 및 운영 승인 |

무제한 shell, 자동 권한 획득, 승인 없는 외부 전송은 기본 제품 범위에서 제외합니다. 지원 macOS 하한, 테스터 범위, 비용·계정·라이선스 정책은 미결정입니다.

## E0~E3b 근거 감사

검증 소스 SHA는 당시 실험 대상입니다. PR이 있는 다섯 단계는 PR HEAD와 squash commit의 추적 파일 트리가 동일함을 이번에 `git diff --quiet`로 확인했습니다. 이후 변경된 현재 트리의 실행 성공은 그 비교로 판정하지 않습니다. 이번 작업에서는 앱·Rust 테스트·GUI를 실행하지 않았습니다.

| 단계 | 검증한 범위 | 검증 소스 SHA | Issue/PR 근거 | 이번에 직접 확인한 내용 | 과거 실행 기록 | 남은 한계 |
|---|---|---|---|---|---|---|
| E0 | Apple Silicon 도구 실행 | `5265b1b20ee86ae08542587d5cdbe6231ac38d83` | [#3](https://github.com/Kim-Ji-Won/AETHER/issues/3), [Rust 후속](https://github.com/Kim-Ji-Won/AETHER/issues/3#issuecomment-6686457531) | 기준 commit과 현재 빌드 구성 대조 | Swift·Node 실행, 후속 Cargo offline 빌드·실행과 arm64 확인. 초기 Rust 미확보 기록 보존 | 현재 도구 버전 및 다른 Mac은 UNKNOWN |
| E1a | 첫 창, 로컬 앱 | `8f603e1a6e11f3df34a1d8992026515c0dbcf74d` | [#4](https://github.com/Kim-Ji-Won/AETHER/issues/4#issuecomment-6686709228) / [#5](https://github.com/Kim-Ji-Won/AETHER/pull/5) | React 상태·스타일, Tauri 창 설정, 병합 트리 대조 | TypeScript/Vite, arm64 .app, 개발 서버 없이 창과 코어 점등 클릭 | 3D·음성·AI·공개 배포 미검증 |
| E1b-1 | 로컬 DMG와 복사본 | `c72d536e64340b4ed083cbc4729808db3d0d39b3` | [#6](https://github.com/Kim-Ji-Won/AETHER/issues/6#issuecomment-6687130601) | E1a 병합 commit과 Issue 소스 일치, 별도 PR 없음 | 이미지 검증, 읽기 전용 마운트·복사·분리, arm64 복사본 PID·GUI | Developer ID·공증·stapling·다른 Mac NOT_RUN |
| E2a | Rust–Swift C ABI | `8d6ed100136397d7acb7ad0cbad6362d805e7b76` | [#7](https://github.com/Kim-Ji-Won/AETHER/issues/7#issuecomment-6687361206) / [#8](https://github.com/Kim-Ji-Won/AETHER/pull/8) | Swift 컴파일·링크, CLI 진단, 병합 트리 대조 | 번들과 외부 복사본의 OS major 26, `sw_vers` 26.6과 일치, GUI | 이 함수만 검증. 일반 native API 호환성과 서명 배포 UNKNOWN |
| E2b | React→Rust→Swift IPC | `1a9a9a7cb1d7f71705bd07a72149634721c268c8` | [#9](https://github.com/Kim-Ji-Won/AETHER/issues/9#issuecomment-6687712809) / [#10](https://github.com/Kim-Ji-Won/AETHER/pull/10) | AppManifest, local `main` capability, CSP, 병합 트리 대조 | 정상 창·CLI·macOS major 일치. label만 바꾼 번들에서 실패 UI | raw IPC rejection 미수집. 일반 명령 보안 검증 아님 |
| E3a | Accessibility 신뢰 조회 | `207faf19e80dd4f7a5256fd7f798d584ed1ee182` | [#11](https://github.com/Kim-Ji-Won/AETHER/issues/11#issuecomment-6688159816) / [#12](https://github.com/Kim-Ji-Won/AETHER/pull/12) | Swift `AXIsProcessTrusted()`, Rust 0/1 변환, capability·병합 트리 대조 | 두 번 false 조회, 권한 팝업·설정 창 미관찰, 미허용 창 실패 UI | 실제 허용·철회와 다른 Mac NOT_RUN. false의 이유 UNKNOWN |
| E3b | 작업별 승인과 메모리 카운터 | `dd58a11961892ec824d1124f7aba685aee4b44f4` | [#13](https://github.com/Kim-Ji-Won/AETHER/issues/13#issuecomment-6688672930) / [#14](https://github.com/Kim-Ji-Won/AETHER/pull/14) | pending ID 소비, Swift 고정 sheet·callback, 읽기 전용 getter 및 local ACL, 병합 트리 대조 | GUI 자동화 취소 0→0, 승인 0→1, 새 취소 1→1. 위조·미허용 창 확인, 중복·경쟁 Rust 2 tests | 사용자의 직접 클릭, sheet가 열린 부모 창 강제 종료, 다른 Mac NOT_RUN. 외부 부작용 exactly-once UNKNOWN |

E3b의 병합 SHA는 이 RFC 기준 commit입니다. [해당 main push governance CI](https://github.com/Kim-Ji-Won/AETHER/actions/runs/42246697311)는 문서·정책과 whitespace 검사입니다. 앱 실행 근거는 별도 Issue 기록입니다. 현재 [Rust 승인 코드](../../apps/desktop/src-tauri/src/approval.rs)는 응답 채널을 기다리며 만료 시각을 두지 않습니다. **INFERRED**로, native callback이 유실되면 pending이 풀리지 않거나 IPC 수신자가 사라진 뒤 같은 ID의 승인으로 카운터가 갱신될 수 있습니다. 현재 외부 작업은 없으며 실제 재현은 NOT_RUN입니다. E4의 reload·종료 시험과 P4의 만료·callback 지연 검증으로 다룹니다.

## 모듈과 상태 소유권

아래 표는 **PROPOSED**입니다. 현재 구현은 마지막 열에만 표시합니다.

| 모듈 / 실행 위치 | 소유 상태 | 입력 → 출력 | 접근 데이터·필요 권한 | 실패 시 동작 | 현재 구현 |
|---|---|---|---|---|---|
| React / 3D Presentation · WKWebView | 창별 보기·입력·렌더 상태 | 사용자 입력 → 좁은 command, host snapshot·event → UI | 표시용 projection, locale, 오디오 envelope. local 창 capability | snapshot 재동기화, 오류 표시, 2D fallback | React/CSS 첫 창과 진단·카운터 UI |
| Tauri Rust Host · 앱 프로세스 | 작업 원장, `run_id`, 세션 세대, 정책 및 저장소 진입점 | IPC 검증 → 상태 전이, Runtime·Swift 명령 → 결과 | 작업 metadata와 필요한 본문, Keychain 참조. 고정 command | 잘못된 전이 거부, 취소·reconcile, 민감값 가림 | 좁은 commands, native 연결, 카운터·pending |
| Swift Native Bridge · 앱 프로세스 / main thread UI | sheet·OS 핸들, 오디오 리소스 후보 | 형식화된 native 요청 → typed 결과 | OS 상태, 승인 요약, 필요한 장치·대상 권한 | 거부·철회·닫기 → handle 정리와 명시 오류 | OS major, AX 상태, NSAlert |
| Agent Runtime · A WebView / B Node Sidecar | 일시적 모델 대화·계획 진행, 취소 핸들 | 제한된 task context → 모델 요청·tool 제안 | 작업에 지정된 projection, host IPC. 자격 증명·범용 실행권 없음 | 연결 단절 통지, host 원장과 재결합 | 미구현 |
| Model Provider Adapter · Rust 서비스 또는 Managed Backend | provider session·rate limit·취소 토큰 | 최소 텍스트/음성 context → delta·결과·제안 | 해당 요청 데이터, 서버 키 또는 host가 읽는 BYOK | timeout·quota·offline 상태, 허용 범위의 retry | 미구현 |
| Voice Session · Swift 오디오 또는 검증된 WKWebView 전송, Rust 세션 제어 | 장치·buffer·capture/playback 상태, session generation | PTT·PCM·Realtime events → 재생·상태·envelope | 명시 세션의 오디오, 마이크 OS 권한, 필요 시 단기 secret | 캡처·재생 정지, 버퍼 폐기, 사용자 주도 재연결 | 미구현 |
| Policy / Approval / Tool Execution · Rust, Swift native 표시 및 어댑터 | allowlist, 승인 record, attempt 및 receipt | 제안 → 검토 요약·native 결정 → 허용 도구·결과 | 정확한 대상·정규화 인자, OS 권한과 작업별 승인 | deny 기본값, 모호한 결과는 `outcome_unknown` | 고정 카운터 실험만 |
| Local Memory / Storage · Rust 전용 DB 접근 | 저장한 기록, 출처, 보존·삭제 상태 | 사용자 CRUD·승인된 조회 → 최소 projection | 선택한 로컬 데이터와 파생 인덱스, app data 경로 | transaction rollback, 읽기 제한·복구 안내 | 미구현 |
| Managed Backend · 별도 운영 서비스 후보 | 사용자 인증·quota·키·운영 감사 | 인증 요청 → provider 요청 또는 Realtime 초기화 | 전송 동의가 있는 payload, 운영자 키는 서버 비밀 저장소 | rate limit·서비스 오류, 앱에 키 제공 없음 | 미구현 |

UI는 마지막으로 적용한 `seq`와 화면 상태만 소유합니다. 작업·승인·도구 결과의 기준은 Rust 원장입니다. SQLite 후보를 사용하면 Rust가 기억과 영구 작업 이력을 저장하고, 재시작 시 영속화한 사실만 복원합니다. 개발 키는 host가 사용하는 OS 비밀 저장소 후보, 운영자 장기 키는 Managed Backend에만 둡니다. Runtime은 키·파일·브라우저 제어의 주체가 되지 않습니다.

## Runtime 배치 비교

잠정 추천은 **A, WebView 내부 TypeScript orchestration**입니다. 현재 패키징 경로에 추가 실행 파일 없이 동일한 orchestration을 붙일 수 있고, Rust가 작업 상태·승인·실행을 소유하면 UI와 권한 판단을 분리할 수 있습니다. 우선 작업을 foreground로 제한하고 reload 때 안전한 취소가 사용자 요구를 충족하는지 확인합니다. 아래는 가설이며 [E4 비교](e4-runtime-comparison-plan.md)와 E5 수명주기·성능 증거가 필요합니다.

| 비교 | A — WebView TypeScript | B — bundled Node.js Sidecar |
|---|---|---|
| 숨김·닫기·재열기 | WebView가 보존되는지에 좌우. 숨김·minimize 시 throttling을 측정 | host가 child와 연결을 유지하면 창과 독립. 재열기 snapshot 필요 |
| reload·프로세스 실패 | orchestration context 유실 예상. 새 세대가 원장을 조회 | UI reload와 child crash를 분리. child 단절·재시작 처리 필요 |
| 장시간 작업 | 창 없는 지속 요구에는 불리할 수 있음. host가 timeout·취소 기록 | 장시간 작업에 유리할 가설. backpressure와 종료 보장이 필요 |
| 취소·중복 | capability 뒤 typed IPC, Rust의 ID/seq/상태 검증 | 동일 상태 규칙과 framing·handshake·generation 추가 |
| Quit | host가 작업·오디오·IPC 취소, WebView 종료 확인 | pipe 종료 → 유예 → 필요 시 이번 child만 종료 → wait/reap. host 비정상 종료도 시험 |
| 개발·테스트 | 기존 TS 도구 재사용, 브라우저용 dependency 제한 | Node API 사용 가능, 바이너리·프로토콜·crash 재현·검증 축 추가 |
| 크기·배터리·메모리 | 추가 Node 없음. WebView CPU·GPU 및 renderer 메모리 관찰 | Node binary·script 크기, child RSS/CPU, idle wakeups를 추가 측정 |
| 패키징·서명·업데이트 | 기존 .app 경로, CSP·WKWebView 호환 확인 | arm64 Node와 잠긴 script를 bundle 내부에 포함. target triple·nested code 서명과 전체 앱 업데이트 검증 |
| 자격 증명 | WebView에 운영 키 금지. 필요한 텍스트 요청만 host 경유 | child 환경·args·stdin에 장기 키 금지. host/backend adapter 경유 |
| IPC·보안 | local 명령 allowlist. 렌더러 손상 시 허용 명령도 host가 재검증 | private stdio의 제한된 frame. 일반 Node의 OS 파일·네트워크 권한은 별개 |

A가 필수 창 재생성·장시간 작업에서 실패하거나 기능에 필요한 TS 라이브러리가 WebView에서 동작하지 않고, B가 같은 기능과 안전성 및 E5 예산을 충족하면 B로 바꿉니다. B의 경우 창 독립성이 사용자 요구로 확인돼야 하며, 패키징·clean-machine 실행·종료·보안에 실패하거나 A가 요구를 충족하면 A를 우선합니다. 둘 다 실패하면 백그라운드 요구를 재검토하거나 별도 host orchestration 후보를 연구합니다.

Node 프로세스가 직접 `fs`, 네트워크, child process, addon 등을 호출하면 Rust broker를 우회할 수 있습니다. 번들 무결성·잠긴 dependency·최소 환경·고정 실행 경로·금지 import 검토와 합성 접근 검사로 노출을 줄입니다. Node Permission Model은 선택된 버전에서 방어 보조 수단으로 평가하며 악성 코드 격리로 간주하지 않습니다. 임의 코드를 실행해야 하는 요구가 생기면 운영체제 격리 검증까지 도구 출시가 BLOCKED입니다. 공개 앱은 전역 `node`·개발자 PATH 없이 설치와 업데이트를 검증합니다. [Tauri Sidecar](https://v2.tauri.app/develop/sidecar/) · [Node.js Permissions](https://nodejs.org/api/permissions.html) · [Rust Child](https://doc.rust-lang.org/std/process/struct.Child.html)

## 작업 식별과 복구

Rust가 작업마다 `run_id`, 입력 요청과 실행 시도마다 `request_id`·`attempt_id`, Runtime 연결마다 `generation`을 발급합니다. 클라이언트 재전송 식별자와 검증된 payload를 묶고 같은 ID에 다른 payload가 오면 거부합니다. `seq`로 progress와 snapshot을 맞추며, 동일 요청은 기존 상태를 반환하고 terminal 상태를 되돌리지 않습니다. 승인 record는 `approval_id`, run/attempt, tool·버전, 정규화한 대상·인자의 digest, native 창·generation, 발급 시각·만료 시각, 단일 소비 여부를 묶습니다. TTL 초안은 표시 후 60초이며 기능별 실험에서 조정합니다. native 응답과 실행 직전에 만료·대상·정책·권한을 다시 확인합니다.

`queued → running → awaiting_approval → executing → succeeded/failed`를 기본으로, 사용자 취소는 `cancel_requested → cancelled` 또는 이미 부작용이 나갔으면 `outcome_unknown`으로 표시합니다. 완료와 취소는 host가 직렬화합니다. 이전 세대, 만료·소비된 승인, terminal 이후의 delta·결과는 상태를 바꾸지 않습니다. 앱 재시작 시 pending 승인은 무효, 시작 전 작업은 `interrupted`, dispatch 여부가 불명확한 외부 작업은 `outcome_unknown`입니다. Receipt나 읽기 전용 확인으로 결과를 찾고, 없으면 사용자가 대상 상태를 검토한 뒤 새 요청으로 판단합니다. 공급자가 idempotency를 보장할 때만 해당 범위를 기록하며 자동 재실행을 기본값으로 삼지 않습니다.

| 시나리오 | 요청부터 종료까지 |
|---|---|
| 텍스트 | 사용자 전송과 데이터 범위 표시 → Rust가 run 생성·quota 검사 → Runtime이 허용된 context 선택 → Adapter가 Responses 호출 → delta를 host가 정렬해 UI에 투영 → 완료·오류·취소 기록 |
| 음성 | PTT·마이크 상태 확인 → 명시 세션·장치 활성화 → 허용된 오디오를 Realtime에 전송 → 발화 종료·응답 수신 → 재생 envelope로 Core 반응 → 중단/완료 시 캡처·재생 종료 |
| 도구 | 모델의 구조화된 제안 → Rust schema·대상·데이터 분류 및 정책 검사 → Swift가 고정 형식으로 정확한 작업 제시 → native 결정 → Rust가 승인 소비·실행 직전 재검증 → 좁은 어댑터 dispatch → receipt 또는 결과 불명확 표시 |
| 기억 | 명시 저장 요청 → 출처·본문·보존 범위 미리보기 → 사용자 확인 → Rust DB transaction → 조회·수정·삭제 → 파생 데이터 무효화·재생성 또는 삭제 완료 표시 |
| 단절·실패 | UI cancel / 연결 종료 / child exit → Rust가 세대 폐기·승인 무효화 → 오디오/IPC/child 정리 → receipt 기반 reconcile → snapshot 재연결 또는 결과 불명확·수동 확인 |

E3b 카운터는 단일 프로세스 메모리에서 동일 pending 완료의 중복 증가를 막습니다. 파일·전송·원격 작업은 process crash와 effect/receipt 사이의 장애를 포함하므로 위의 결과 확인 정책이 필요합니다.

## 권한과 승인 경계

1. **Tauri capability**는 어떤 local 창이 어느 command에 들어올 수 있는지 제한합니다. local 창에서도 schema, 길이, origin/label, generation, 범위를 검사합니다.
2. **macOS 시스템 권한**은 해당 프로세스의 실제 접근을 OS가 결정합니다. 필요한 기능에서 권한을 설명하고 사용자가 설정을 관리합니다. 거부·철회 시 장치·대상 접근을 즉시 멈추고 pending을 무효화합니다.
3. **작업별 승인**은 특정 tool·대상·인자·전송 데이터·attempt의 일회성 동의입니다. Rust가 만든 snapshot을 Swift가 표시합니다. 모델·WebView의 `approved` 필드는 권한을 만들지 못합니다. 실행 직전에 대상 해석과 권한을 재검사하고 변경되면 새 승인을 요구합니다. [Tauri Capabilities](https://v2.tauri.app/security/capabilities/) · [AppKit NSAlert](https://developer.apple.com/documentation/AppKit/NSAlert)

E3b에서 GUI 자동화가 native 승인 버튼을 눌렀습니다. NSAlert만으로 사람이 직접 클릭했음을 증명할 수 없습니다. AETHER의 제어 대상에서 자기 bundle·PID·창과 권한 관련 UI를 제외하고, 승인 대기 동안 자체 UI 입력/AX 및 간접 키보드·포인터 경로를 broker에서 막습니다. frontmost 전환·alias·간접 브라우저/단축키·queued 입력으로 자기 창에 도달하려는 합성 테스트와 사용자 직접 승인 테스트를 분리합니다. 식별 실패 시 실행을 차단합니다. 다른 로컬 자동화나 손상된 프로세스는 여전히 클릭을 합성할 수 있습니다. 고위험 도구는 추가 신뢰 수단과 독립 검증 전 노출을 보류합니다.

| 위협 | 경계 | 검증 계획 |
|---|---|---|
| 웹·파일·모델 prompt injection | 콘텐츠를 불신 데이터로 태그. 모델 출력은 tool 제안만 가능. 정책·승인 변경은 host 전용 | 위장 정책/승인 문자열을 세 출처로 넣고 도구·전송 0 확인 |
| 임의 파일·외부 전송 | 선택 파일의 정규화 경로와 handle을 사용, symlink/TOCTOU 재검증. 목적지·필드·범위를 동의 화면에 표시 | 임시 폴더 traversal·symlink 교체, 합성 전송 목적지 변경 거부 |
| IPC 위조·replay | 좁은 command·versioned schema·size cap, ID/세대/순번·TTL. 승인 callback은 native 경로 | 잘못된 label/형식, 위조 approval, duplicate, stale callback, race, 부모 창 닫기 |
| 키와 개인정보 로그 | 구조화한 event code·익명 run ID·기간만 기록. 키·payload·음성·문서·홈 경로는 제외 | 합성 canary로 UI·stderr·crash/diagnostics·로그·backup의 비노출 검사 |

## AI 연결과 자격 증명

OpenAI Responses API는 일반 텍스트·tool 제안의 후보입니다. Rust host 또는 Managed Backend의 adapter가 `POST /v1/responses`를 Bearer 인증하고 앱 사용자는 별도의 앱 인증을 사용합니다. 공개 앱 번들, WebView, sidecar, 로그에 운영자의 장기 키를 제공하는 설계는 금지합니다. 모델·SDK·비용·quota 수치는 미결정입니다. [Responses create](https://developers.openai.com/api/reference/resources/responses/methods/create) · [API 인증](https://developers.openai.com/api/reference/overview)

| 경로 | 장기 키·요청 경로 | 한도·교체 | 운영 부담 |
|---|---|---|---|
| 개발용 BYOK 후보 | 사용자 자신의 개발 키를 macOS Keychain 후보에 보관. Rust adapter가 필요할 때만 읽고 provider에 직접 요청 | 개발 프로젝트 quota, 앱 단위 속도 제한, 설정에서 참조 삭제·교체, provider 폐기는 사용자 수행 | 개발 간단. 사용자의 청구·키 관리, 로컬 프로세스 노출, 서명 변경 후 접근 검증 필요 |
| 공개용 Managed Backend 후보 | 운영자 키를 서비스 비밀 저장소에서 관리. 앱 로그인 → TLS backend → Responses 또는 Realtime 초기화 | 사용자·프로젝트별 quota, abuse 감시, 운영자 키 rotation/revoke와 세션 중단 절차 | 서비스 가용성·인증·개인정보·비용·지원·사고 대응 필요 |

**DOCUMENTED · 2026-11-11.** Realtime WebRTC는 서버가 `POST /v1/realtime/calls`로 SDP를 초기화하는 방식과, 서버가 `POST /v1/realtime/client_secrets`로 단기 secret을 발급하고 클라이언트가 직접 WebRTC를 연결하는 방식이 있습니다. 단기 secret의 `expires_after`는 `created_at` 기준 10~7200초, 생략 시 600초이며 응답 `expires_at`을 읽습니다. 이 만료는 새 세션 생성에 적용되고 이미 시작한 세션의 종료를 보장하지 않으며 유효 기간 내 복수 생성이 가능하므로 일회성 승인으로 취급하지 않습니다. 앱은 메모리에서만 필요 시간 동안 보유하고 세션 종료·서버 quota를 별도로 적용합니다. 서버 간 WebSocket과 일반 Responses 인증의 장기 키는 신뢰 서비스 경계에 둡니다. 구체 경로는 P3 패키징 실험에서 결정합니다. [Client secrets](https://developers.openai.com/api/reference/resources/realtime/subresources/client_secrets/methods/create) · [WebRTC](https://developers.openai.com/api/docs/guides/voice-webrtc?voice-api=realtime) · [WebSocket](https://developers.openai.com/api/docs/guides/voice-websockets)

## 음성

초기 제안은 Push-to-Talk입니다. 누름과 실제 캡처 시작을 따로 표시하고, 해제 시 신규 프레임 수집을 중단해 마지막 버퍼만 전송합니다. 명시적 해제를 기본 turn boundary로 두고 VAD는 과도한 침묵과 한국어·영어 기술 용어가 섞인 발화를 같은 fixture로 비교합니다. 응답 중 다시 누르면 재생을 즉시 중단하고 provider 응답을 취소하며 재생된 지점 이후의 오디오를 버립니다. 늦은 프레임은 session generation으로 폐기합니다.

장치 변경·echo 검증은 내장 스피커·헤드셋과 입력 장치 제거를 포함합니다. 장치 교체 시 capture/playback을 멈추고 다시 선택한 뒤 시작합니다. 네트워크 단절은 `offline`으로 표시하고, 이전 turn 자동 재전송 없이 명시적으로 재연결합니다. Quit·권한 철회·실패에서 capture·playback·buffer·전송을 정리합니다. 음성 레벨은 제한된 envelope만 UI에 전달하며 원음 저장은 기본값에서 제외합니다.

WKWebView WebRTC는 브라우저 경로의 사용 편의성을, Swift AVFoundation 경로는 장치·재생 수명주기 제어를 비교합니다. 각각 패키징된 앱에서 microphone purpose string, OS 권한·entitlement, local origin, 장치·echo·Quit를 확인해야 합니다. 두 경로의 AETHER 실행은 NOT_RUN입니다. Wake Word는 별도 opt-in 후보이며 상시 클라우드 전송을 전제로 하지 않습니다. [Apple macOS media capture](https://developer.apple.com/documentation/bundleresources/requesting-authorization-for-media-capture-on-macos) · [WKUIDelegate](https://developer.apple.com/documentation/webkit/wkuidelegate/webview%28_%3Arequestmediacapturepermissionfor%3Ainitiatedbyframe%3Atype%3Adecisionhandler%3A%29)

## 기억과 개인정보

SQLite 기반 Local-first는 Rust가 transaction·backup·migration을 소유하는 제안입니다. 아래 기간은 초안이며 사용자가 더 짧게 설정·즉시 삭제할 수 있게 설계합니다.

| 데이터 | 출처 / 보존 제안 | 조회·수정·삭제·복구 |
|---|---|---|
| 대화 기록 | 사용자 메시지·응답, 저장 opt-in 후 기본 30일 | 목록/내보내기, 내용 삭제·보존 변경, 삭제 반영 후 복구 검증 |
| 승인된 장기 기억 | 사용자 편집 또는 제안을 확인한 항목, 삭제까지 | 출처·수정 이력 최소화, 수정·삭제와 파생 데이터 동기화, 선택 백업 |
| 작업 이력 | Rust의 결과·시간·대상 요약, 기본 30일 | 실패·불명확 결과 조회, 삭제·내보내기, payload와 승인 토큰 제외 |
| 현재 실행 상태 | host의 pending·generation·오디오 buffer, 프로세스 수명 | 메모리 정리. 재시작 후 승인 복구 금지. 모호한 dispatch는 이력의 최소 receipt로 reconcile |

원본 ID로 요약·검색 인덱스·임베딩·캐시를 추적합니다. 삭제 transaction에서 조회를 즉시 차단하고 파생 데이터를 제거하거나 재생성하며, 실패 시 삭제 대기 상태를 표시하고 재시도합니다. 사용자 내보내기·과거 백업에 남을 수 있는 사본은 UI에서 범위를 알리고, 복원은 삭제 목록·보존 정책을 적용한 뒤 공개합니다. WAL·임시 파일·FTS 흔적과 복구 테스트를 별도 검증합니다. SQLite 자체를 암호화 보장으로 취급하지 않으며 저장소 접근 정책, 암호화·키 복구·백업 방법은 별도 결정입니다. 클라우드 전송은 로컬 저장과 분리해 데이터 범위·목적·보존·수신자를 표시하고 명시 동의를 받습니다. 원격 임베딩은 별도 승인 전 후보입니다. [SQLite Backup](https://www.sqlite.org/backup.html) · [SQLite secure_delete와 제한](https://www.sqlite.org/pragma.html#pragma_secure_delete)

## Frontend와 A+B 3D Core

2026-11-12 사용자 범위 구분: P1.1은 DOM 기반 UI Shell·탐색·입력·상태·접근성에 집중하고 실제 3D는 P1.2로 분리합니다.
P1.2는 3D Core와 유리·금속 소재의 2.5D Navigation·Context·Command Dock을 하나의 **Cinematic Spatial Interface**로 통합하는 설계·검증을 포함합니다.
Core와 UI의 공유 조명·색상·상태별 연출, 깊이 계층, 절제된 Hover/Focus/Motion, reduced-motion과 GPU·배터리 예산을 함께 검증합니다.
P6에서는 이 통합 체계의 최종 시각 품질과 성능을 다듬습니다. [구체 요구사항과 상태 연출표](p1-ui-shell.md#cinematic-spatial-interface-통합-요구사항)를 따르며,
이 기록은 P1.2 착수·의존성 채택·최종 시각 승인·PR 병합 승인이 아닙니다.

P1에서 첫 실제 3D Core를 만든다는 제안입니다. **A — Orbital Intelligence**는 정교한 궤도, 다중 회전 링, 구조적 깊이를, **B — Neural Plasma**는 중심의 흐름과 맥동하는 발광을 담당합니다. Three.js / React Three Fiber는 후보입니다. P1은 실제 3D 장면과 명시된 Mock 상태를 만들고 P2~P5에서 Runtime·음성·승인·기억 이벤트를 연결합니다. P6에서 시각 품질과 성능을 조정합니다. [Three.js](https://threejs.org/docs/) · [R3F 성능](https://r3f.docs.pmnd.rs/advanced/scaling-performance)

- **Main**은 입력·PTT·현재 상태·코어. **Command Center**는 작업, 승인 요청·결과, 취소와 결과 불명확 확인. **Memory Center**는 출처·검색·수정·삭제·보존. **Settings**는 provider·오디오·권한 상태·motion·경량 모드·개인정보.
- `idle` 느린 궤도, `listening` 입력 envelope에 반응, `thinking` 제한된 회전, `speaking` 출력 envelope에 맥동, `awaiting_approval` 정지한 링과 명시 텍스트, `executing` 단계 진행, `error` 정지와 오류 안내, `offline` 약한 고정광과 재연결 안내. 색만으로 상태를 전달하지 않습니다.
- Rust snapshot과 ordered event를 화면으로 투영합니다. microphone 시작·끝과 playback 상태는 Voice Session 실제 이벤트를 사용합니다. mock fixture에는 `Mock` 표시를 두고 실제 버튼 권한이나 실행 상태를 생성하지 않습니다.
- Resize 시 카메라·pixel ratio를 조정하고, 숨김/최소화 시 animation loop를 정지합니다. WebGL context 실패 시 2D 정적 코어와 입력을 유지합니다. `prefers-reduced-motion`과 사용자 설정은 회전·맥동·섬광을 줄이고, 경량 모드는 DPR·postprocessing·particle을 제한합니다.
- 성능 **제안값**은 기준 Apple Silicon Mac에서 visible p95 frame time ≤ 33 ms, hidden 렌더 frame 0, 정적 fallback 대비 추가 RSS ≤ 150 MiB입니다. 정적/3D/경량 모드의 GPU·CPU·프로세스 RSS·에너지와 배터리 변화, device/OS·전원·화면 밝기·부하를 고정해 E5·P6에서 측정합니다. 실제 측정 NOT_RUN입니다.

## 중요 결정과 변경 조건

| 선택지 | 추천안·근거 | 단점과 남은 불확실성 | 필요 실험 | 추천을 바꿀 조건 |
|---|---|---|---|---|
| WebView / bundled Node | A 우선, 기존 앱에 작은 변경 | 창 생명주기와 긴 작업 한계 | E4 기능·실패, E5 자원·긴 작업 | A가 필수 지속성 실패, B가 종료·보안·배포·성능 통과 |
| UI 승인 / Rust broker + native / OS 추가 인증 | Rust + native, E3b에서 좁은 경로 확인 | 합성 클릭, 외부 효과, 만료 미검증 | P4 자기 승인·철회·close·receipt | 고위험 작업의 사용자 의도 증거가 부족하면 별도 인증 또는 도구 제외 |
| BYOK / Managed | 개발 BYOK, 공개 Managed 후보 | 로컬 키 노출 또는 서버 운영비·전송 | P2 비밀·quota, P3 세션, P7 운영 | 서버 운영·개인정보 요건을 감당하지 못하면 공개 provider 범위 축소 |
| WKWebView / Swift audio | PTT, WKWebView부터 짧은 패키징 검증 | 장치 전환·echo·권한·Quit | P3 동일 음성 fixture와 중단 조건 | WKWebView에서 장치·수명주기 실패, Swift 경로가 통과 |
| Local-first SQLite / 원격 저장 | SQLite 후보, 사용자가 저장·삭제 관리 | WAL/파생·암호화·복구 | P5 CRUD·삭제·backup·migration | 로컬 삭제·암호화 요건 실패 시 엔진/저장 범위 재평가 |
| R3F/Three.js / 2D fallback | P1 A+B 3D + 2D fallback | 장치별 GPU·배터리·motion | P1 scene 확인, E5/P6 자원 | 제안 예산이나 접근성 미달 시 효과 축소·fallback 기본 |
| Developer ID DMG / Store | 직접 배포 우선 후보, 로컬 DMG 기록 보유 | 서명·공증·다른 Mac·업데이트 UNKNOWN | P7 nested code·stapling·Gatekeeper·복구 | 직접 배포 조건 실패 또는 유통 요구 변경 시 별도 Store 검토 |

공식 자료는 API·플랫폼 동작 범위의 DOCUMENTED 근거입니다. AETHER의 실제 환경·서명·권한·비용·장치별 호환은 실험과 별도 승인으로 판단합니다. [검증 정책](../engineering/testing-policy.md) · [배포 정책](../engineering/release-policy.md) · [Apple 공증](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
