# P2.2b-1 · Native Credential & Consent Boundary

[Issue #58](https://github.com/Kim-Ji-Won/AETHER/issues/58) · 기준 main `142d3762f526c0e23c12c902f5fb922f20df4ca3` · [P2.2b-0 승인 설계](p2-2b-controlled-live-ai-readiness.md)

개발용 Local BYOK의 첫 검증 대상은 **사용자가 승인한 그 요청만 native Host가 받아들이는가**이다. 이번 구현은 오프라인 Rust library 경로에 한정한다. 앱의 Command Dock은 계속 Synthetic을 사용한다.

## Credential 소유권

기존 `CredentialResolver`가 credential과 borrowed lease를 소유한다. 앱이 생성할 수 있는 resolver는 `DisabledCredentials`이며 Secret과 Lease 타입은 `Infallible`이다. Mock은 `cfg(test)` 내부의 두 enum 값만 받는다. 문자열·키 바이트·파일·환경변수를 입력받는 생성자가 없다. Native 상태는 다음과 같다.

| 상태 | Resolve 및 편집 |
| --- | --- |
| Disabled | Production 및 shutdown 뒤 resolve는 Disabled. 종료한 resolver에는 재등록할 수 없다. |
| Missing / Revoked | Resolve는 Missing. 명시적인 새 register가 필요하다. 철회 뒤 복원해도 과거 동의는 사용할 수 없다. |
| Available | 한 번에 하나의 mutable borrow를 통해 lease를 발급한다. 중복 register는 AccessDenied. |
| AccessDenied / Unavailable | 각각의 기존 credential 오류를 유지한다. 실패한 접근을 Provider 오류로 변환하거나 다른 저장소에서 재시도하지 않는다. |

Register·rotate 성공 시 credential generation이 증가한다. 활성 run 또는 미확인 cleanup이 있으면 편집은 Busy다. Borrowed lease가 사용되는 동안 replace/shutdown을 호출하는 두 코드 예제는 컴파일 단계에서 거부되어야 한다.

### Credential 철회 순서

Native owner는 철회 요청을 받으면 먼저 admission을 잠그고 승인 슬롯을 비운 뒤 활성 run을 취소한다. 단순 동의 철회는 credential 삭제를 요청하지 않는다. 호출자는 `revocation()`으로 아래 진행 상태를 확인한다. `revoke_credential()`의 성공 반환에는 cleanup 대기 중인 요청 접수도 포함된다.

| 진행 상태 | 삭제·재등록 계약 |
| --- | --- |
| `AwaitingCleanup` | Run이 없거나 이미 terminal이고 cleanup이 끝났으면 즉시 resolver 삭제를 시도한다. Queued 취소는 자원이 없어 `NotRequired`로 진행한다. Dispatch 뒤에는 adapter와 runtime 양쪽 자원 해제를 확인할 때까지 삭제·등록·회전·새 admission을 차단한다. Deadline 초과나 한쪽 ACK로는 삭제하지 않는다. Runtime loss 후에는 소유자가 확인한 정리 경로를 사용한다. |
| `Confirmed` | Resolver 삭제 성공. 명시적 새 등록에 성공하면 generation이 증가하고 상태가 `NotRequested`로 돌아간다. 다음 요청에는 새 동의가 필요하다. |
| `Failed(error)` | 삭제가 실패했거나 확인되지 않았다. 기존 안전한 credential 오류를 보존하고 resolve·승인·등록·회전을 막는다. 중복 철회로 삭제를 재시도하지 않는다. Resolver가 Available을 보고하거나 외부 쓰기가 성공해도 잠금은 유지된다. |

지연된 삭제 결과는 native 철회 상태에 기록한다. 유효한 cleanup ACK는 Host에서 처리되며 이미 확정한 terminal·partial output은 보존된다. 철회 대기 중 shutdown은 Host를 닫고 cleanup과 삭제 시도 뒤 resolver를 종료한다. 현재 잠금은 메모리의 owner 수명에 적용된다. 영구 credential 도입 전에는 이전 항목 삭제를 확인하고 재시작 후에도 불확실성을 복원하는 명시적 reconciliation 계약이 필요하다. 이 단계에는 복구 API와 실제 Keychain 접근이 없다.

## 동의와 admission

1. 신뢰된 native 호출에서 기존 Host가 **후보** Identity를 계산한다. 세션, 중복 요청, 활성 run, cleanup, 입력·rate 한도를 기존 start와 같은 함수로 검사한다. 이 시점에는 run도 비용 예약도 발생하지 않는다.
2. 명시적 승인 동작은 테스트 전용 fixture에서만 제공한다. Native가 선택한 고정 endpoint, 모델을 포함한 정확한 요청 body, Host의 run/request/attempt·session generation, UI session/epoch, credential generation, policy version을 메모리의 한 슬롯에 결속한다. 앱이 승인 입력을 받는 IPC는 없다.
3. `start_consented`는 단일 mutable borrow에서 슬롯을 꺼내고 현재 Host·resolver 상태와 provider ledger의 정책을 다시 확인한다. 모든 필드가 일치할 때만 lease를 resolve하고 해제한 뒤 기존 Host에 start를 commit하고 기존 ledger에 예약한다. 승인 누락·불일치·resolve 실패는 admission 거부다. 가짜 terminal, ID 소비, 중복 예약을 만들지 않는다.
4. 재사용·입력 수정 후 원상 복구 재시도는 거부한다. 승인 철회와 정책 버전 변경은 이전 승인을 폐기한다. Runtime loss 뒤에는 새 native context를 붙이고 다시 승인해야 한다. 재시작한 provider에는 승인 슬롯이 비어 있다. 향후 native UI에는 편집·취소 시 즉시 pending 승인을 폐기하는 연결이 필요하다.
5. 이미 시작된 작업은 Rust Host가 관리한다. dispatch 전 취소는 cleanup=NotRequired, dispatch 후에는 adapter/runtime 양쪽 확인이 필요하다. 늦은 ACK는 기존 terminal을 갱신하지 않는다. 실제 원격 결과나 청구를 로컬 cleanup으로 확정할 수 없다.

Legacy P2.2a 가상 SSE start는 disabled resolver에만 존재한다. Credential을 가진 fixture는 검증된 start 경로를 거친다. 두 경로는 기존 OfflineProvider의 Host와 ledger를 재사용한다. 프로덕션 `live_dispatch()`는 계속 ProviderDisabled를 반환한다. 실제 모델·가격·예산 설정 또는 새 권한은 없다.

## 검증 근거

| IMPLEMENTED / TESTED 영역 | 판정 기준 |
| --- | --- |
| Credential lifecycle | Disabled/Missing/denied/unavailable/revoked, 중복 등록, exclusive lease, 교체·철회·재등록·종료 결과 |
| Binding / replay | 정확한 입력(공백 포함), request/body/model/endpoint, run·attempt·session, generation/version 변경 거부. 미동의·불일치에서 resolve 0회 |
| Native admission | Host identity 예측과 commit 일치, 비용 정책 거부, stale context, 단일 소비, 위조 IPC 승인 필드 거부, cleanup 이전 Busy |
| Non-disclosure | Mock Secret·Lease·binding의 Clone/Debug/IPC 응답 타입 금지 컴파일 검사, snapshot/event/error에 동의 payload 미포함 |
| 기존 회귀 | Rust Host/Provider의 duplicate·partial·late·cancel, Native/FakeRun, Frontend Synthetic·Dock, Typecheck. macOS network-denied 범위는 library와 binary의 agent_runtime 테스트 |

실행 명령, 최종 커밋 SHA, 실제 개수와 GitHub CI는 Issue와 연결된 Draft PR에 기록한다. Network-denied는 해당 시험 프로세스에 적용한다. UI나 전체 앱의 외부 통신을 관측했다는 의미로 확대하지 않는다.

## Keychain 결정 대장과 다음 승인

| 미결정 항목 | 실제 접근 전 필요한 결정·증거 |
| --- | --- |
| Team ID / Bundle ID | 사용자 소유 개발 팀과 전용 개발 번들 식별자를 정하고 production과 분리한다. |
| Signing identity / entitlement | 사용할 인증서·프로비저닝·access group 일치, 업데이트·재설치·서명 변경 시 접근 실패/재등록 동작을 서명된 앱에서 검증한다. |
| Keychain / test item 격리 | 전용 service/account, 기기·잠금 정책, 비동기화, 잔존 항목 폐기·철회 실패 처리, 타 항목 접근 금지를 사용자와 정한다. |
| Secure entry / consent | native secure field와 표시·철회 흐름, crash capture 제외·메모리 수명 감사, 실제 OS 접근을 별도 승인한다. |
| b-2 진입 | b-1 코드·회귀 증거 리뷰와 별도 구현 승인 후 고정 endpoint/TLS, offline transport fixture, backpressure·abort/join·비용 정책 연결을 검토한다. live 실행은 b-3의 독립 승인이다. |

**NOT_IMPLEMENTED / NOT_RUN** · 실제 API Key 관리, Keychain register/resolve/replace/revoke, native 등록 UI, signing/entitlement 접근 시험, 실제 HTTP·Provider·usage·billing. P0 E5b/E5c PARTIAL, E5d ADR PROPOSED, 전역 Runtime 미선정 상태를 유지한다.
