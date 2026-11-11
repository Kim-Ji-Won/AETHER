# E4 Runtime 배치 비교 실험 계획 · Runtime Comparison

**PROPOSED / NOT_RUN** · 2026-11-11 · [E4a Issue #15](https://github.com/Kim-Ji-Won/AETHER/issues/15) · [RFC 001](rfc-001-aether-system.md)

E4a는 이 계획을 작성하는 단계입니다. 후속 E4는 배치의 기능·실패 동작, E5는 수명주기·성능의 반복 측정을 다룹니다. 이 문서는 실행 승인이나 Runtime 선정 기록이 아닙니다. 기준 소스는 `9683f8a1154ef02f8b72b952db8cd5e20df73a50`입니다. 실제 실험 SHA·환경·artifact hash는 실행 때 기록합니다.

## 공통 계약과 통제 조건

- **A**는 패키징된 WKWebView의 TypeScript 모듈, **B**는 패키징된 Node.js Sidecar의 같은 TypeScript orchestration입니다. 두 변형의 고정 fixture, UI·Rust 상태 규칙·로그·timeout·취소 정책을 같게 두고 Runtime adapter와 번들 자원만 바꿉니다. 같은 Rust host가 상태와 승인 결정을 소유합니다.
- Fake Provider는 네트워크·파일·장치·비밀 없이 고정 UTF-8 입력 `테스트 질문`과 fixture 버전을 받아 100 ms 간격의 progress 25/50/75와 `FAKE_OK` terminal 결과를 냅니다. 테스트 barrier로 각 이벤트를 멈추고 재개해 취소와 late reply 순서를 재현합니다. 별도 fixture는 부작용 없는 60초 task와 fake 승인 대기 단계를 사용합니다. 시간 편차는 측정 대상으로 남기며 기대 event 순서는 고정합니다.
- 공통 frame은 `version`, `type`, `run_id`, `request_id`, `generation`, `seq`, bounded payload를 갖습니다. Rust가 허용 type·caller·size cap·ID·상태를 검사합니다. A는 local-only Tauri capability, B는 host가 생성한 비공개 stdin/stdout과 고정 frame 크기·handshake를 씁니다. stdout의 frame과 stderr의 제한된 진단을 분리하고 읽기·쓰기 backpressure를 검사합니다. TCP listener나 shell command를 WebView에 열지 않습니다.
- B는 버전·출처·해시를 고정한 arm64 Node executable과 잠긴 script를 app bundle에 포함하는 후보입니다. host가 검증한 bundle-relative 경로, 최소 환경, 고정 인자로만 시작합니다. 전역 `node`, 빌드 경로, dev server 없이 실행합니다. 일반 Node의 직접 접근 위험은 [RFC의 Runtime 비교](rfc-001-aether-system.md#runtime-배치-비교)에 따릅니다. [Tauri Sidecar](https://v2.tauri.app/develop/sidecar/) · [Node.js Permissions](https://nodejs.org/api/permissions.html)
- 앱 종료와 창 닫기를 구분합니다. 창 숨김·최소화의 허용 정책을 먼저 고정하고, WebView가 파괴되면 A는 승인 무효화와 명시적 중단을 기본으로 합니다. B는 읽기 전용 fake 작업만 창이 없어도 host가 이어받도록 평가합니다. 창 소멸 중 승인이나 실제 외부 작업을 이어서 실행하지 않습니다. 복구 후 UI는 host snapshot과 event seq를 받아 상태를 일치시킵니다.
- 환경은 같은 Apple Silicon Mac·OS build·전원 상태·앱 설정·해상도·SDK/lockfile·부하로 고정합니다. 격리한 테스트 사용자 데이터와 임시 산출물만 사용합니다. 정상 Quit, crash injection 대상, PID, bundle-relative executable을 식별한 뒤 이번 인스턴스에만 작용합니다. 오류 후 앱/child·fixture·임시 mount를 정리하고 기존 번들과 사용자 데이터를 보존합니다.

## 시나리오별 절차와 판정

각 행은 별도로 실행할 검증 단위입니다. 공통 조건에 추가되는 전제와 코드를 명시합니다. 아래 값은 모두 예상 판정 기준이며 관찰 결과는 **NOT_RUN**입니다.

| 목적 / 시나리오 | 동일 비교 조건·사전 조건 | 필요한 최소 코드 | 수집할 증거 | 성공·실패 기준 | 정리·복구 | 후속 결정 |
|---|---|---|---|---|---|---|
| 정상 요청→progress→완료 | 동일 packaged build 설정·fixture. fresh host, runtime ready | 공통 frame, Fake Provider, Rust ledger, 표시용 이벤트 | start/25/50/75/terminal의 PID·ID·순서·결과 | 두 후보가 각각 한 start, 순서대로 progress, terminal 한 번, `FAKE_OK`. 누락·중복 실패 | fresh run, 정상 Quit | parity 이후만 실패 시험 확대 |
| 실행 중 취소와 late reply | progress 25에서 barrier, host가 취소 확정 후 지연 event·terminal 주입 | cancel token·terminal 전이·event reject | cancel ACK, late drop, terminal·실행 수 | cancelled 한 번, 뒤늦은 결과가 상태를 바꾸지 않음. 응답 후 UI·ledger 불일치 실패 | barrier 해제, 채널·timer 종료 | 양쪽이 통과할 때만 실제 provider 취소 검토 |
| 잘못된 IPC | 동일 논리 payload, unknown type/버전/ID·size 초과, B에는 잘못된 framing도 추가 | schema·size 검증, 에러 응답 | reject code와 start 0, 새 정상 요청 결과 | 정상 작업·승인 전이 0, 제한된 시간 내 정상 재요청 가능 | 입력 drain 또는 연결 재생성 | 검증 실패 candidate 차단 |
| 동일 메시지 재전송 | 같은 run/ID/seq·payload 재전송 및 같은 ID에 다른 payload | host dedupe·digest·terminal record | accepted/replayed/conflict, start·terminal count | 같은 payload 한 실행, conflict 거부, terminal 회귀 0 | record TTL 범위 내 비교 후 초기화 | 중복 키의 수명과 보존 범위 결정 |
| WebView reload·종료 | 동일 barrier에서 reload와 renderer crash를 따로 실행 | generation 교체, attach snapshot, A cancel hook | renderer PID/세대, host ledger, B child 지속 여부 | A 안전 중단·복원된 UI, B 허용 fake 작업만 지속·재구독, stale event 0. silent drop 실패 | WebView 재생성, 재연결 안 되면 명시 중단 | 창 독립성이 필수일 때 B 판단 근거 |
| Sidecar 비정상 종료 | B 실행 중 child kill, A에서 동일 지점의 Runtime 종료를 대조. B 없음은 A에서 해당 없음 | exit 감지, generation invalidate, 제한 재시작 | child PID·exit code, host terminal, 새 PID/세대 | host 생존, 이전 작업 자동 재실행·승인 복원 0. 사용자가 새 요청 가능 | 해당 child wait/reap, 새 연결만 허용 | crash 복구·비용이 감당되는지 판단 |
| 숨김·닫기·재열기 | 60초 fixture. hide/minimize/close 별도, 재열기 시점 고정 | window event, 상태 projection, reconnect | 실제 창 이벤트, host/Runtime PID, progress gap, 재열기 상태 | 숨김/최소화 정책대로 동작, close 후 A 중단·B 허용 작업 정책 일치, 재열기 일관된 snapshot | window 복원, 남은 fake task 취소 | 지속성 요구와 실제 throttle 비교 |
| 앱 Quit 및 잔존 프로세스 | idle/active/승인 대기에서 정상 Quit, host 강제 종료는 별도 case | stop intake, IPC close, child 종료 유예·wait/reap, B EOF exit | Quit 전후 parent/child PID·시작시각·경로, 종료 시각, 잔존 상태 | 정상 Quit 후 제안 5초 이내 이번 child 0. host crash 후 정해진 감시 구간에도 orphan 0. 불명확 PID 실패 | 관련 PID만 검증 후 종료하고 실패 기록 | 종료 실패 B 제외. OS 보장·대체 구조 검토 |
| 승인 대기 중 중단·재시작 | 동일 fake 승인을 요청한 뒤 부모 창 close와 host restart를 따로 수행, 이전 응답 재전송 | native sheet 결합, expiry·generation invalidate, fake effect counter | approval_id·run, native close·cancel·reject, effect count | pending 해제, 과거 승인으로 effect 0, 재시작 후 새 승인만 허용 | sheet·monitor·채널 종료 확인 | E3b의 강제 종료 NOT_RUN 해소 근거와 P4 gate |
| 빌드 폴더 밖 독립 실행 | 동일 arm64 .app을 새 폴더에 복사, cwd 분리, dev server 없음, 최소 PATH, 깨끗한 기기 확인은 별도 | A bundle assets, B bundle Node·script locator, build manifest | artifact hash, arm64·executable 경로, A/B 실행과 `FAKE_OK` | dev 경로·전역 Node 참조 없음. B child가 bundle 내부에서 실행. 실패 시 배포 후보 보류 | 복사본과 시험 데이터만 제거, 원본 보존 | E4 local feasibility, P7 서명·공증·업데이트 검증으로 연결 |

## 증거 수집과 E5 계획

화면 관찰은 UI 상태의 증거로만 사용합니다. Rust ledger와 각 프로세스가 `trial_id`, `variant`, `component`, process PID/parent PID, app instance, generation, run/request ID·seq, event type, monotonic elapsed, result/exit code를 구조화해 기록합니다. Runtime clock은 서로 직접 빼지 않고 host가 IPC 전후 시각을 기록해 latency를 산출합니다. PID 재사용 방지를 위해 시작시각·bundle-relative 경로를 함께 확인합니다. fixture hash, app/source SHA, 명령·exit code·실패 이유를 보존하고 공개 기록에는 본문·원음·키·개인 경로·원본 캡처를 제외합니다.

- **E4 표본 제안**은 각 기능 시나리오 A/B 각 5회, cold launch와 취소·replay·crash race는 최소 각 10회입니다. 실패·timeout도 표본에 포함하고 분모와 오류 코드·순서를 남깁니다. 기능 gate는 요구한 불변식 위반 0회입니다. 이는 안전성 전체에 대한 보장은 아닙니다.
- **E5 표본 제안**은 cold launch와 첫 progress/cancel latency를 A/B 교차 순서로 각각 30회, 별도 warm-up 3회를 제외합니다. 정적 대조, foreground, hidden, active 60초, Quit 전후를 분리합니다. CPU·GPU·RSS(프로세스별과 합산), 에너지 지표, 배터리 변화 및 idle wakeups는 고정 간격으로 수집하고 15분 조건당 3회 이상 반복합니다. .app·DMG bytes, Runtime 구성 자원 bytes도 별도 비교합니다.
- 정렬한 실제 값의 `ceil(p × n)`번째 값으로 p50/p95를 산출합니다. 단위·표본 수·실패/누락 수와 min/max를 함께 남깁니다. timeout은 성공 latency에서 제외하되 timeout 횟수·설정값을 보고하고 비교 gate에 포함합니다. 한 차례 실행으로 순위를 정하지 않습니다.
- 초기 **제안값**은 start→첫 progress p95 ≤ 250 ms(고정 100 ms fixture 포함), cancel ACK p95 ≤ 250 ms, 정상 Quit 잔존 0 및 종료 ≤ 5초입니다. 작업 지속 요건, 지원 Mac 기준과 자원·배터리 허용치는 E5 실험 승인 시 확정합니다.

| 측정 | A | B |
|---|---|---|
| 시나리오별 통과/실패 횟수 | NOT_RUN | NOT_RUN |
| cold launch·progress·cancel p50/p95 | NOT_RUN | NOT_RUN |
| CPU·GPU·RSS·에너지·배터리·wakeups | NOT_RUN | NOT_RUN |
| 번들 크기·Quit/잔존·복구 | NOT_RUN | NOT_RUN |

A가 사용자 요구의 지속성을 충족하면 단순한 패키징을 우선합니다. A가 지속성에서 실패하고 B가 공통 안전·종료·배포 및 합의된 E5 예산을 충족하면 B를 선택할 근거가 생깁니다. 배치 결정은 E4/E5 결과와 [ADR 승인](../decisions/README.md)으로 진행합니다.

## 다음 최소 검증 단위

**E4b 제안 — Option A 정상 경로 한 건.** 패키징된 A에서 네트워크 없는 고정 fixture 1건을 시작하고, Rust ledger와 UI가 동일 `run_id`로 progress 25/50/75와 `FAKE_OK` terminal 1건을 기록하면 완료입니다. 공통 계약의 작은 기준을 만들고 B 패키징·취소·장시간·성능은 후속 단위에서 검증합니다. E4a에서 착수하지 않았습니다.
