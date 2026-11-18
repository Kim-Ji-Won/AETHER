# P2.2a — Native Responses Provider Foundation

- Issue: [#54](https://github.com/Kim-Ji-Won/AETHER/issues/54)
- 기준 main: `c053627c8dcb2445e59ccba266ff6ce4a242eb1d` (PR #53 MERGED, #52 COMPLETED 확인)
- Branch: `feat/54-native-responses-provider`
- 상태: 오프라인 Rust 구현 / Draft 리뷰 대상. 실제 HTTP Provider·Keychain·과금·P2.2b·병합 승인 없음.
- 공식 Schema 확인 및 로컬 검증: 2026-11-18. 정확한 제출 HEAD/CI는 PR 본문에 기록한다.

## 범위와 경계

P2.0 계약, P2.1a `Host`, P2.1b Lifecycle/Timer, P2.1c UI를 재사용한다. 새 `provider::OfflineProvider`가 기존 Host를 소유하고 신뢰 native 진입점에서만 `source_kind=provider`를 정한다. 기존 `Host::start`와 앱의 모든 IPC는 여전히 Synthetic이다. Renderer Start/Frame에 provider 출처를 넣으면 기존 closed schema가 거부한다. OfflineProvider는 Tauri command로 등록하지 않았으며 실제 앱의 두 번째 controller가 아니다.

목표 live 구조는 `Dock → 기존 gated IPC → Rust Host → Native Adapter → 승인된 Responses Endpoint`이다. 이번 구현은 마지막 native adapter 경계의 메모리 전용 검증 기반이다. `live_dispatch()`는 항상 `ProviderDisabled`이고 설정 boolean이나 모델 allowlist만으로 HTTP 권한을 만들 수 없다. 새 HTTP client, network transport, Keychain 구현, shell/URL 실행 경로, UI 활성화 경로가 없다. 기존 Lifecycle Gate와 CSP/Capability를 바꾸지 않았다.

`provider`는 이 library fixture가 검증하는 adapter 경계의 출처이지 실제 원격 응답을 받았다는 표시가 아니다. 앱에서는 기존 **SYNTHETIC / LOCAL TEST / AI Provider 미연결** 표시만 사용한다. P2.2b에서 실제 HTTP task와 기존 Lifecycle Gate를 연결하고 이 경계의 실환경 증거를 추가해야 한다.

## 변경 파일과 책임

모든 Rust 경로의 prefix는 `apps/desktop/src-tauri/`다.

| 경로 | 책임 |
| --- | --- |
| `Cargo.toml`, `Cargo.lock` | 기존 lock의 `serde_json =1.0.154`를 bounded JSON parser의 직접 의존성으로 명시. 패키지 버전/체크섬 업그레이드 없음 |
| `src/agent_text/mod.rs` | Host가 출처를 선택하는 private admission; 기존 Synthetic budget과 native policy Ledger 분리 |
| `src/agent_text/synthetic.rs` | SyntheticAdapter가 provider run에 dispatch되지 않도록 guard |
| `src/agent_text/provider/mod.rs` | OfflineProvider: identity binding, Host clock/terminal/cleanup 연결, live disabled |
| `src/agent_text/provider/sse.rs` | byte incremental SSE framing, UTF-8 line 검증, frame/stream hard bound |
| `src/agent_text/provider/mapper.rs` | official SSE → text-only typed projection; Provider sequence와 canonical sequence 분리 |
| `src/agent_text/provider/policy.rs` | 고정 endpoint/request preview, host allowlist, 가상 비용 예약/rate/usage |
| `src/agent_text/provider/error.rs` | bounded HTTP body와 status/code/type → safe enum; 원본 메시지 폐기 |
| `src/agent_text/provider/credential.rs` | native Credential lifecycle trait, 호출 불가능한 Disabled 구현 |
| `src/agent_text/provider/fixtures.rs`, `tests.rs` | 가상 transport/clock/barrier, 비민감 fictional schema fixtures, 49개 테스트 |
| `docs/architecture/README.md`, 본 문서 | 범위·공식 출처·실제 검증·미실행 기록 |

Frontend/Particle/Dock/Navigation, FakeRun, Native Bridge/Approval, Lifecycle, Tauri commands/Capability/CSP, Node dependency/lockfile은 변경하지 않는다. UI 재디자인·음성·도구·Memory는 없다.

## SSE Schema와 정규화

공식 [Streaming 가이드](https://developers.openai.com/api/docs/guides/streaming-responses)와 [Streaming Event Schema](https://developers.openai.com/api/reference/resources/responses/streaming-events)를 조회했다. 아래는 API 전체 schema를 복제한 것이 아니라 bounded text-only projection이다. 소비하는 필드는 타입·상태·identity를 검사하며, 소비하지 않는 Response metadata는 canonical event로 전달하지 않는다. 알 수 없는 event/content 종류는 fail closed한다.

| Event | 확인 필드 / 처리 |
| --- | --- |
| `response.created` | `type`, integer `sequence_number`, `response.id/object/model/status/output/error/incomplete_details`. 현재 선택 model 일치, in_progress·빈 output으로 Response binding 생성 |
| `response.output_text.delta` | `item_id/output_index/content_index/delta/logprobs/sequence_number`. 현재 assistant message와 단일 text part에만 허용, bounded nonempty text만 Host delta로 변환 |
| `response.completed` | 동일 response/model, status=completed, error/incomplete_details=null, 최종 output과 누적 delta 일치, bounded usage 검사. 명시적 성공 후보만 Host Completed에 전달 |
| `response.failed` | 동일 response/model, status=failed, `response.error.code/message`. code만 allowlist로 분류, message 미투영. usage가 있으면 별도 기록 |
| `response.incomplete` | status=incomplete, `incomplete_details.reason`. safe Incomplete 분류와 partial output 유지, usage가 있으면 기록. 성공 아님 |
| `error` | top-level `code/message/param/sequence_number/type` (Response wrapper 아님). code만 분류; created 이전에도 실패 허용 |

실제 text lifecycle의 `response.in_progress`, `response.output_item.added/done`, `response.content_part.added/done`, `response.output_text.done`도 처리한다. 단일 `message/assistant/output_text`만 지원하며 done의 text/id/status와 최종 output이 누적 delta와 같아야 한다. 빈 output + 명시적 completed + valid usage는 빈 성공 응답으로 허용한다. Tool/function/audio/refusal/reasoning/annotations/비어 있지 않은 logprobs 및 여러 item/part는 이번 subset에서 UnsupportedContent다. 실행하거나 승인으로 해석하지 않는다.

SSE는 LF/CRLF/CR, 최초 BOM, comments, multiline data, event 생략을 지원한다. `id/retry`는 metadata로 무시하며 재연결·재전송을 구현하지 않는다. frame 및 전체 stream byte 수는 comments도 포함한다. byte별 framing으로 chunk 크기와 UTF-8 분할 위치에 독립적이며 한 번에 한 bounded message만 JSON parse한다. JSON parser의 기본 recursion bound를 유지한다.

Provider sequence의 최초 값은 이 subset에서 0 또는 1을 허용하며 이후 모든 metadata 포함 +1을 요구한다. 동일 sequence는 Duplicate, 역행/누락은 Sequence 오류로 해당 bound run을 실패시킨다. 이는 자동 replay 복구를 하지 않는 보수적 정책이며 sequence 시작값의 제품 보장은 아니다. Canonical sequence는 검증 후 Host가 별도로 생성한다. provider/response/item/model identity는 Host run/attempt/session identity와 별개이며 JSON identity만으로 native sender 권한이 생기지 않는다.

중간 EOF/끊긴 UTF-8/종료 구분자 없는 completed JSON/[DONE]만 수신은 성공이 아니다. completed 이후 owned reader를 닫고 잔여 CRLF를 버린다. 닫힌 stream에 직접 주입한 늦은 이벤트는 거부한다. `RunSucceeded`는 Host가 Adapter와 Runtime cleanup 둘 다 확인한 뒤에만 발생한다.

## Credential 및 요청 정책

`CredentialResolver`는 register/resolve/replace/revoke/shutdown 책임만 정의한다. Lease는 resolver를 빌려 쓰는 GAT여서 생존 중 resolver 교체/종료를 막는 타입 경계가 있다. 실제 Secret/Lease 타입은 Debug/Serialize/Clone/IPC 금지와 bounded lifetime·zeroization 감사를 향후 구현 조건으로 둔다. 메모리 zeroization·OS swap 제거·Keychain entitlement를 이번 코드가 검증했다고 주장하지 않는다. 현재 Disabled 구현의 Secret/Lease는 `Infallible`이며 resolve/revoke는 Disabled다. 실제 Secret/환경 변수/Keychain을 읽거나 쓰지 않았다.

RequestPreview는 메모리 전용이고 Debug/Serialize를 구현하지 않는다. native `body()`는 향후 transport/현재 fixture용이며 사용자 text를 포함하므로 로그·diagnostics로 보내면 안 된다. [Responses create schema](https://developers.openai.com/api/reference/resources/responses/methods/create)에 맞춰 고정 endpoint `https://api.openai.com/v1/responses`, host allowlist model, user/input_text, stream=true, store=false, background=false, tools=[], tool_choice=none, parallel_tool_calls=false, max_output_tokens만 생성한다. 임의 headers/URL/query/credential이나 Renderer 지정 model을 받아들이는 IPC는 없다.

실제 model ID·가격·월별 예산은 미선정이다. 반환 model은 현재 allowlist 선택과 정확히 같아야 한다. P2.2b에서 선택 모델의 alias/snapshot, 실제 SSE item 형태와 usage/cache schema의 호환성을 재검증해야 한다.

`store:false`는 response 저장 옵션이며 전체 데이터 보존 0을 보장하지 않는다. [Data controls](https://developers.openai.com/api/docs/guides/your-data)의 abuse monitoring, prompt caching 등 별도 보존 정책과 사용자 전송 동의를 P2.2b에서 검토해야 한다.

## 비용·오류·취소

공식 가격 출처는 [API Pricing](https://developers.openai.com/api/docs/pricing)이다. 실제 가격은 코드에 넣지 않았다. fixture-model의 input은 **UTF-8 byte 기반 가상 추정 unit**, 출력은 가상 max_output_tokens, 가격은 가상 정수 unit/token이다. 실제 tokenizer/청구 hard cap/월별 집계가 아니다.

Host admission 성공 시 예상 입력 + 최대 출력 비용을 예약한다. Host active run과 cleanup gate, 기존 Host rate 및 Provider rate window가 모두 적용된다. Provider Ledger는 Synthetic budget을 소비하지 않는다. Reject는 quota를 소비하지 않으며 취소·불확실한 결과의 예약은 반환하지 않는다. usage의 input/output/total 정합성, cached/reasoning subset, output/input budget과 비용 초과를 검사한다. reasoning은 output에 포함되므로 이중 합산하지 않는다. 비용 미설정 cache write는 거부한다. completed뿐 아니라 failed/incomplete에 usage가 있으면 기록하되 **provider-reported usage ≠ invoice**, billing_confirmed=false를 유지한다. 초과 비용은 다음 admission에 반영하고 성공으로 표시하지 않는다. 실제 tier/캐시 할인·캐시 write/추정 정확도는 미검증이다.

[공식 Error Codes](https://developers.openai.com/api/docs/guides/error-codes)에 따라 400 InvalidRequest, 401 Authentication, 403 Permission, 429 RateLimit/Quota/Credit/SpendLimit/Unknown429, 500~599 Server를 구분한다. 429는 status만으로 RateLimit이라 하지 않는다. credit_balance_exhausted, project/organization_spend_limit_exceeded, organization_usage_limit_exceeded/insufficient_quota, rate_limit_exceeded/slow_down 및 type fallback을 구분한다. 모르는 code는 Unknown이다. body는 8192 bytes 이내에서 code/type만 읽고 message/param/raw body/URL/header는 projection에 남기지 않는다.

native safe enum의 Credit/Spend/Quota는 기존 canonical vocabulary에서 QuotaExceeded로 투영한다. 400/Incomplete/Unknown은 ProviderError, malformed/identity/sequence는 ProtocolError로 투영한다. 상세 enum은 native inspection에만 있으며 현재 UI 오류 schema 확장을 가장하지 않는다. 자동 Retry는 코드 경로가 없으며 0회다.

Cancellation/Timeout은 기존 Host clock/state/terminal을 재사용한다. queued 취소는 cleanup=NotRequired; dispatch 후 CancelRequested는 local abort와 Runtime release가 각각 필요하다. false ACK로 자원을 해제하지 않는다. local abort barrier는 offline buffer 소유권만 검증한다. live socket/task 종료 증거의 대체가 아니다. Cleanup deadline 초과는 Failed/CleanupUnconfirmed, 늦은 ACK는 cleanup만 갱신하며 terminal을 바꾸지 않는다. Runtime loss는 generation을 올려 old ACK/event를 거부하고 owner-only cleanup 경로를 분리한다. 원격 처리/과금은 local cleanup 후에도 Unknown이다.

## 검증 및 한계

| 실행 | 결과 / 범위 |
| --- | --- |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --offline` | PASS 128: Library 95 (신규 Provider 49 + 기존 Host 46), Binary 33 (Runtime bridge 14 + 기존 Native/FakeRun/Lifecycle/Sidecar 등 19). Doc-test 0 |
| `node --experimental-strip-types --experimental-vm-modules --test tests/*.test.mjs` (desktop) | PASS 56: Command Dock/Runtime/SSR/Synthetic/Fake Provider/Particle·scene/real pinned Node 회귀 |
| `npm run typecheck` (desktop) | PASS |
| `sandbox-exec -p '(version 1)(allow default)(deny network*)' <library-test-binary> --quiet` | PASS 95; macOS에서 실제 network-denied 실행 |
| 동일 sandbox의 `<binary-test-binary> agent_runtime::tests --quiet` | PASS 14; 합계 109, 전체 GUI/전체 앱 sandbox 증거 아님 |
| `python3 scripts/validate_governance.py`, `git diff --check` | PASS; CI는 해당 제출 HEAD에서 별도 확인 |
| 실제 HTTP Provider / Keychain / Real Billing·실제 Token Usage | **NOT_RUN** (승인 없음) |
| Packaged App 재빌드·GUI / 실제 VoiceOver / 실제 장문 AI 출력 | **NOT_RUN**. 이번에는 source/UI 불변 + 자동 회귀를 확인했으며 이전 Bundle 증거를 새 HEAD PASS로 재사용하지 않음 |
| rustfmt | **NOT_RUN**: 설치된 stable toolchain에 component 없음. 새 도구 설치/업그레이드하지 않음 |

신규 49개 테스트는 `provider/tests.rs`에 이름별로 추적 가능하다. 정상/빈 응답, UTF-8 한 바이트 분할과 여러 chunk 크기, CR/LF/BOM/multiline/comments, malformed JSON/UTF-8, 모든 terminal 이전 event 경계 EOF, duplicate/conflict/gap/stale/숫자형 오류, response/item/content/model binding, done·completed text/status 불일치, 필수 필드 누락, unsupported content, safe failed/incomplete/error, byte/output cap, queued/running cancel, completion race·late input·cleanup pending/timeout, clock/Runtime loss/shutdown, fixed request/disabled credential/live dispatch, input/rate/active/cost/usage 경계와 synthetic budget 분리를 포함한다.

초기 검증 실패는 수정 후 재검증했다: Cargo PATH를 절대 경로로 해결, 새 Worktree에 복사된 Swift cache의 원본 module 경로 충돌은 복사본만 옆으로 이동해 재생성, 테스트의 private 필드 접근 제거, CRLF byte barrier가 completed 이후 닫힌 가상 reader에 남은 LF를 재전달하던 fixture 수정. 최초 89/90과 최종 PASS를 구분한다.

P2.0의 54개 설계 fixture 전체 PASS를 주장하지 않는다. 기존 자동 Host/bridge 회귀와 신규 Provider 49개는 서로 다른 집계다. 실제 Provider의 네트워크·Credential·Billing·app integration은 후속 검증이다.

## 보안·P0 보존 및 P2.2b 선행 조건

External Provider Call **0회**의 근거는 새 모듈에 HTTP client/socket/process/env/Keychain/logging 호출이 없고 live dispatch가 항상 거부되며, 모든 입력이 fictional fixture이고 network-denied 테스트가 실제 통과했다는 점이다. fake counter=0을 실측으로 표현하지 않는다. GitHub/공식 문서 조회는 모델 inference 호출이 아니다. 실제 Secret을 사용하는 테스트는 없으며 synthetic canary raw error가 Host snapshot/event/native error에 나오지 않는 것을 확인했다.

기존 10개 Worktree의 HEAD/status/diff/untracked hash를 작업 전후 비교한다. 별도 Worktree의 cache/Node/node_modules는 기존 검증본을 COW 복사해 준비했고 원본을 변경하지 않았다. main/P0 고정 App·원본 evidence/#38 미커밋 작업을 보존한다. P0 측정 chat은 시작 전과 검증 중 completed로 확인했으며 재개 시 Build/Test/GUI를 중단한다. ADR PROPOSED, E5b/E5c PARTIAL, Runtime A 제한적 Foreground 경로만 유지한다.

P2.2b는 별도 승인 후에만 시작한다. 최소 선행 조건은 모델·snapshot/가격·token/cost/rate 한도 확정, 사용자 데이터 전송/보존 동의, Rust native Credential UI/Keychain 등록·조회·교체·폐기와 secret lifetime 검증, TLS/고정 endpoint/redirect 차단/stream backpressure 구현, 기존 Lifecycle gate 및 단일 controller에 live adapter 연결, 실제 monotonic driver와 socket/task abort confirmation, 401/429/5xx·network loss·remote outcome unknown의 controlled live 검증이다. 현재 엄격한 단일 message/text subset과 모델별 실제 event/usage 형식 호환성, incomplete·refusal·reasoning 정책 및 safe UI projection도 검토해야 한다. 실제 AI와 Synthetic 표시는 명확히 분리해야 한다.

이번 Draft PR은 Ready 전환/병합하지 않고 사용자 리뷰를 기다린다. P2.2b Issue/Branch·live 호출을 만들지 않는다.
