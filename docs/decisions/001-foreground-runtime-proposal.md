# ADR 001 · 개발용 Foreground Text Conversation Runtime

- 상태: **PROPOSED**
- 날짜: 2026-11-15
- Issue: [#44](https://github.com/Kim-Ji-Won/AETHER/issues/44)
- 선행: [RFC 001 PROPOSED](../architecture/rfc-001-aether-system.md), [E4 비교 계획](../architecture/e4-runtime-comparison-plan.md), [독립 Recovery 결과](../architecture/e5-recovery-results.md)
- 결정 권한: 사용자 승인 전 Runtime 선정·ACCEPTED·P2 실행 없음.

## 배경과 근거 범위

첫 개발용 대화는 메시지 입력 → 실제 모델 요청 → 응답 표시 → 진행·취소·오류·사용자 재시도까지만 목표로 한다. 마이크·TTS·기억·컴퓨터 제어·상시 백그라운드 실행은 제외한다. 기존 Rust 원장, native 승인 경계, A/B 독립 이벤트 계약을 재사용하며 새 아키텍처를 처음부터 설계하지 않는다.

기존 E5b #27/#34는 144회 중 본 측정 A normal/cancel n30, B 각각 n29와 TIMEOUT 2건이 있는 PARTIAL이다. 이번 #44도 144회 계획 중 30회에서 중단해 유효 본 측정 n4~5, ON/OFF NOT_RUN, E5c BLOCKED이다. 두 실행의 환경·source가 다르므로 표본을 합치지 않는다. 새 실행이 원본 PARTIAL을 해결했다고 볼 수 없다.

[2026-11-12 E4e 기록](../architecture/e4e-lifecycle-safety.md)은 A reload 시 context loss를 failed terminal로 표시하고, B는 Host/child 작업 유지·재구독이 가능함을 보여준다. 전체 matrix는 당시 이전 실행 파일, 최종 파일은 수정 후 주요 Quit·정상/회귀 검증에 해당한다. 이번 작업에서 matrix를 재실행하지 않았다. 실제 renderer crash는 여전히 NOT_RUN이다.

## 선택지와 요구사항

| 질문 | A · WKWebView Runtime | B · Bundled Node Sidecar | 이번 판단 |
|---|---|---|---|
| Foreground 짧은 텍스트 대화 | 기존 UI/TS와 연결, 추가 child 없음 | 독립 실행 단위·TS 라이브러리 선택 폭 | 둘 다 기능 후보. 새 실측만으로 우승자 없음 |
| Reload 중 작업 지속 필수인가 | context loss로 명시 실패/중단하는 정책 필요 | fake 작업의 지속·snapshot 재연결 기록 있음 | 첫 개발 버전은 중단 허용을 **제안**. 사용자가 지속성을 필수로 정하면 B 재평가 |
| 취소·실패·Quit 회수 | Rust terminal 직렬화, renderer 소멸 처리 | child handle·pipe·wait/reap 추가 책임 | E4 기록과 이번 완료 trial의 좁은 범위만 확인. E5c 자원 회수 반복은 미수집 |
| 시작·첫 progress·terminal 지연 | 이번 normal p50 첫 progress 약103ms | 이번 normal p50 약130ms, Node 준비 약25ms | n5/n4이고 100ms synthetic fixture. 실제 대화 UX의 실질 차이·인과 우위로 해석 불가 |
| Host CPU/RSS, Node overhead | WebKit 귀속 미확인 | Node별 귀속 가능한 경로를 준비 | 이번 E5c 미실행으로 실행 자원 차이 UNKNOWN |
| bundle 크기 | 추가 Node executable을 요구하지 않는 구조 | 고정 앱의 Node executable 약111.5MB | combined 앱 구성 크기 근거만 있음. A-only/B-only 배포 비교 아님 |
| 전력·상주 효율 | 미측정 | 미측정 | 어느 쪽도 에너지 효율 우위 주장 불가 |

## 제안 · 조건부 개발 범위

**Runtime 최종 결정을 유보한다.** 첫 foreground-only 개발 경로의 조건부 우선 후보로 A를 제안할 수 있다. 이유는 현재 UI와 같은 실행 환경을 쓰며 추가 child 배포·회수 책임을 만들지 않는 구조적 단순성이다. 새 latency/resource 실험이 A의 성능 우위를 입증했기 때문이 아니다. 기존 RFC의 A 잠정 선호를 확정 결정으로 승격하지 않는다.

이 제안을 사용하려면 사용자가 다음 조건을 명시적으로 수용해야 한다: Reload/Quit에서 진행 대화를 명시 중단하고 자동 재시도하지 않음, hidden 지속성·상시 실행을 약속하지 않음, 응답/원장은 Rust 기준, 장기 API key는 WebView/Node에 제공하지 않음. 이 조건이 부적합하면 A를 채택하지 않고 B의 window-independent 요구·resource/배포 비용 검증을 우선한다. 두 후보 모두 보안·수명주기 기준을 만족하지 못하면 제3 후보 구현을 조용히 시작하지 않고 요구를 재검토한다.

전체 Runtime 비교 완료를 위해서는 child discovery 보완의 별도 실행 검증, 새로운 사전 등록 지연 계획의 충분한 유효 n, 귀속 가능한 Host/Node CPU/RSS 및 lifecycle recovery 관찰이 남는다. 이번 실패 plan을 지우거나 성공 수를 채워 완료로 변경하지 않는다. 제한된 foreground 개발을 먼저 승인하는 경우에도 이 공백과 confidence가 낮다는 점을 결정 기록에 남겨야 한다.

## P2 진입 계약 · 구현 전 결정할 최소 범위

| 경계 | 제안과 완료 기준 |
|---|---|
| Rust Run Lifecycle | Rust가 run/request/attempt/generation/seq와 terminal을 소유한다. 취소/완료 경쟁에서 terminal 1, 이전 세대·late delta 무효, snapshot 재동기화 테스트가 필요하다. 현재 메모리 원장을 재시작 후 영속으로 표현하지 않는다. |
| Runtime 독립 Event Contract | version·run_id·request_id·generation·seq·event type을 유지한다. 실제 text delta와 bounded error/cancel outcome의 schema·크기·순서를 계약으로 추가하고 A/B 공통 conformance fixture를 만든다. UI 자체 완료 판단 금지. |
| Provider Adapter | Rust 또는 별도로 승인된 backend가 인증·TLS provider 요청·응답 파싱·rate limit·취소 핸들을 소유한다. Runtime은 최소 projection과 상태를 다룬다. provider/model/지원 stream/요금·quota·전송 데이터 범위는 실제 호출 승인 전에 고정한다. 이번 ADR은 특정 API/SDK 버전을 채택하지 않는다. |
| Cancellation / Timeout | Host 기준 요청 deadline, 첫 응답/stream idle timeout, abort 뒤 정리 완료를 정의한다. 원격 요청 중단이 공급자의 처리·과금 취소를 보장한다고 표현하지 않는다. 오류·timeout·사용자 취소를 구분하고 재시도는 사용자의 새 attempt로만 시작한다. |
| Reload / Quit | foreground 최소 범위에서는 UI context 소멸 시 원장을 failed/interrupted로 종결하고 provider transport를 취소하는 정책을 제안한다. silent success, 자동 재전송, 이전 승인 복원 금지. B 지속성을 채택한다면 별도 명시 정책과 snapshot 회복 검증 필요. |
| Credential / Keychain | 개발 BYOK 또는 Managed 경로는 별도 보안 결정이다. Keychain 접근은 native/Host 경계로 한정하고 키를 IPC 응답·WebView state·child env/args/stdin·로그·bundle에 전달하지 않는다. 저장·조회·교체·삭제·서명 변경 후 접근과 canary 비노출을 검증하기 전 실제 키를 넣지 않는다. 이번 작업에서 Keychain 조회/변경 없음. |
| 최소 Tauri IPC | local main capability만 사용한다. 입력 전송·취소·snapshot/event 조회의 좁은 계약으로 제한하고 길이/허용 필드/run ownership/generation을 검증한다. 임의 URL/path/shell·범용 도구 command와 wildcard 권한을 추가하지 않는다. |
| UI 응답 투영 | 수락 전 전송 중, 수락 후 실행 중, Host 확인 delta, 완료/취소/오류 및 사용자 재시도를 표시한다. 실제 모델 응답이 아닌 fixture는 Mock/Preview로 구분한다. 응답은 텍스트로 안전하게 표시하고 임의 HTML·자동 링크 실행을 하지 않는다. |

사용자가 승인할 다음 일관된 범위는 위 계약·보안 경계의 결정과 별도 fixture 기반 구현 검증이다. 실제 모델 요청을 포함하려면 provider/model·키 보관·요금 한도·전송 범위·취소 한계까지 승인 범위에 명시해야 한다. 이번 승인으로 P2 구현·모델 호출을 시작하지 않는다.

## 불확실성, 재검토와 복구

E5c CPU/RSS·반복 recovery, WebKit 귀속, GPU/energy, battery, 장시간 상주, 실제 renderer crash, 다른 Mac, 서명/공증은 새 근거가 없다. 추가 child가 없다는 사실만으로 A의 전체 CPU/RSS가 작다고 추정하지 않는다. 작은 표본의 p95나 fake 100ms 지연을 실제 모델 요청의 tail latency로 사용하지 않는다.

Reload 지속성이 필수로 바뀜, 긴 tool chain/백그라운드 요구, WebView library 제약, 반복 측정에서 deadline/회수 실패, 실제 provider 오류·보안 경계 미충족, 자원/에너지 예산 초과가 확인되면 결정을 재검토한다. 같은 Event Contract와 Rust 원장으로 Runtime을 교체할 수 있도록 책임을 분리하되, migration 가능성을 실험 없이 완료로 주장하지 않는다. 이번 변경은 측정 도구와 문서뿐이므로 앱의 데이터 migration이나 Runtime 전환은 없다.

RUNTIME_SELECTED: NO
ARCHITECTURE_ACCEPTED: NO
P2_IMPLEMENTATION_STARTED: NO
MODEL_CALL_EXECUTED: NO
MERGE_APPROVED: NO
