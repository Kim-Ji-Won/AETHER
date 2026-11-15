# P2.0 Agent Runtime 계약 · Contract Preparation

**PROPOSED · DESIGN_ONLY · 2026-11-15** · [Issue #46](https://github.com/Kim-Ji-Won/AETHER/issues/46)

기준 main: `f44a6c07cec2940633c63e67613dd1259549b1f7`. 이 문서는 [RFC 001](rfc-001-aether-system.md)의 Rust 원장·권한 경계와 [P2.1/P2.2 로드맵](../engineering/development-roadmap.md)을 구체화한 리뷰 초안이다. Runtime A/B 선택, production 아키텍처 승인, 구현 착수 승인이 아니다. 아래 MUST에 해당하는 규칙도 **향후 구현의 제안 계약**이며 현재 앱이 구현했다는 뜻이 아니다.

## 1. 근거와 작업 경계

- 현재 소스: `apps/desktop/src-tauri/src/fake_run.rs`, `lifecycle.rs`, `apps/desktop/src/fake-provider.ts`, `Shell.tsx`, `ShellParts.tsx`를 읽기 전용으로 확인했다.
- [E4d](e4d-failure-safety.md)의 terminal 직렬화·중복 거부와 [E4e](e4e-lifecycle-safety.md)의 instance/context·A reload 실패/B 재부착 차이를 재사용한다.
- [P0 Issue #44](https://github.com/Kim-Ji-Won/AETHER/issues/44) / [Draft PR #45](https://github.com/Kim-Ji-Won/AETHER/pull/45), HEAD `691537c473ce4ef936dc49a7bfccd40aa02a8776`의 보고: Harness Preflight **6/6 PASS**; 독립 E5c **PARTIAL**, 유효 session **0/6**, 첫 A session의 전체 400초 deadline 초과. Runtime 내부 timeout이나 A/B 성능 판정으로 해석하지 않는다.
- PostgreSQL·Docker 복원과 측정 프로세스 정리는 사용자 및 P0 종료 보고에 따른다. 여기서 서비스·측정을 재실행하지 않았다. ADR 001은 PR #45 안의 **PROPOSED**이며 본 브랜치로 가져오거나 ACCEPTED로 바꾸지 않는다.
- 문서·계약 예제·Fixture 설계만 변경한다. 앱/Runtime/IPC/CSP/의존성/lockfile/Particle Orb 변경, 로컬 Build/Test/GUI, 실제 Provider·키·음성·도구·기억 접근은 제외한다. P0의 원본 표본·고정 앱·Worktree를 보존한다.
- E5c 재개 통지 또는 P0 측정 재개 확인 시 로컬 Build/Test/GUI를 중단한다. 이 단계에서는 애초에 실행하지 않으며 측정 중 정적 검사도 보류한다. Fixture PASS나 P0 전체 완료를 주장하지 않는다.

## 2. 소유권과 기존 계약 호환

Rust host만 admission, identity 발급, policy/limit, 상태 전이, event sequence, terminal을 확정한다. Runtime은 제한된 orchestration과 취소 ACK를 제공한다. Provider adapter는 host 또는 별도 승인된 backend의 신뢰 경계에서 text stream을 정규화한다. UI는 host snapshot/event의 projection이다. 구현 언어·프로세스·SDK·전송 방법은 선택하지 않는다.

| 기존 구현 / RFC | P2 제안 | 호환 경계 |
|---|---|---|
| FakeRun `created`, seq 0 | text run `queued`, `RunCreated` seq 1 | 기존 tuple·명령·수치 변경 없음; 새 namespace `agent.text.v1` 제안 |
| `start/progress/terminal`, `FAKE_OK` | `RunStarted/ResponseDelta/RunSucceeded` | progress 25/50/75는 text delta가 아니다. 기존 FakeRun을 실제 응답으로 변환하지 않음 |
| FakeRun `generation` + lifecycle `instance/epoch` | `host_instance_id`, `session_generation`, `ui_context_epoch` | Runtime 연결 세대와 UI 재부착 세대를 별개로 정의 |
| 중복 start/frame는 명시적 오류 | 같은 정책 유지 + 최신 snapshot 조회 | 재전송을 새 dispatch로 해석하지 않음 |
| RFC `awaiting_approval/executing/outcome_unknown` | 이번 text subset에서는 사용하지 않음 | tool·외부 변경은 dispatch 전 거부. Provider 청구 불확실성은 별도 projection |
| RFC `attempt_id` | run당 단일 attempt, ID 별도 발급 | 자동 retry 없음. 향후 재시도는 새 run/attempt |

실행 출처 `source_kind`는 host가 `synthetic` 또는 `provider`로 고정한다. P2.1은 `synthetic`만 허용하는 후속 제안이다. P2.2 승인 전 `provider` admission은 거부한다. `preview`는 UI 전용이며 원장 event가 아니다. 모델/Renderer가 보낸 `source_kind`, `approved`, 역할·상태 문자열은 신뢰하지 않는다.

## 3. Run State Machine

상태: `queued`, `running`, `cancel_requested`, `succeeded`, `failed`, `cancelled`. 마지막 세 개만 terminal이다. `cancel_requested`는 5개 기본 lifecycle 상태에 더한 관측 가능한 중간 상태이며 별도 boolean으로 숨기지 않는다. 동시에 활성 run 하나, 자동 queue scheduling 없음. `queued`는 admission 이후 dispatch 전의 짧은 상태다.

| 현재 | host가 검증한 입력 / 조건 | 다음 | 확정 event / 부수 조건 |
|---|---|---|---|
| 없음 | 유효 Start, session·policy·capacity 통과 | queued | RunCreated; ID 발급, terminal_count=0, dispatch=0 |
| queued | 단일 attempt 시작 및 adapter 준비 | running | RunStarted; dispatch 권한 1회 소비, 실제 원격 수신/청구 증명 아님 |
| queued | dispatch 전 실패·deadline | failed | RunFailed; 외부 dispatch 0 |
| running | 유효 text delta | running | ResponseDelta; bounded buffer에 append |
| running | 명시적 adapter completion + local resource release 확인 | succeeded | RunSucceeded; EOF만으로 성공 금지 |
| queued/running | 사용자 Cancel 또는 host shutdown 정책 | cancel_requested | CancelRequested; 신규 dispatch·delta·성공 즉시 차단 |
| cancel_requested | adapter local stream 종료 + Runtime 정리 확인 | cancelled | RunCancelled; terminal_count=1 |
| cancel_requested | cleanup deadline 만료 또는 cleanup 불가 | failed | RunFailed(CLEANUP_UNCONFIRMED); 새 run admission 차단 유지 |
| queued/running | adapter 오류·protocol 오류·run deadline·Runtime loss (idle deadline은 running만) | failed | RunFailed; stream 폐쇄 및 cleanup 시작, 미확인 resource는 admission 차단 |
| terminal | 어떤 Provider event / 늦은 ACK / 완료 / timeout | 동일 | 거부 또는 진단 결과만; terminal event 재발행 없음 |

거부된 Start는 run/event를 만들지 않는다. terminal 전이는 Rust의 한 직렬화 지점에서 `terminal_count: 0 → 1`로 한 번만 commit한다. 외부 I/O·child wait·abort는 원장 lock 밖에서 수행한다. 실패를 먼저 확정해도 local cleanup은 별도 gate이며 실패 terminal을 cleanup 성공으로 해석하지 않는다.

`cleanup_status = not_required | pending | confirmed | unconfirmed`는 host 운영 snapshot 필드다. 자원이 생성되지 않았음이 host에서 확인된 queued 취소는 not_required이며 가짜 ACK를 기다리지 않는다. terminal의 state·seq·output·error는 불변이며 이후 cleanup 진단만 `snapshot_revision`으로 갱신할 수 있다. revision은 host instance 안에서 run 교체를 포함한 모든 snapshot 변경마다 증가하고 wrap을 금지한다. 새 run은 활성 run이 없고 cleanup이 `not_required/confirmed`일 때만 허용한다. cleanup 확인 불가 상태에서 재시작을 성공한 정리로 자동 간주하지 않는다.

Completed를 수락했으나 local cleanup이 남으면 running 안의 내부 completion-pending 표식으로 보유한다. 이후 delta는 protocol 위반이며 성공을 확정하지 않는다. run deadline은 계속 유효하고 stream idle deadline은 종료하며, 완료 수신 시점부터 cleanup deadline을 적용한다. 이 단계에서 취소가 먼저 commit되면 성공 대신 취소 경로로 들어가되 기존 cleanup deadline을 연장하지 않는다. cleanup 미확인 timeout은 CLEANUP_UNCONFIRMED로 실패한다.

## 4. Identity, Admission, Ordering

| 필드 | 발급·수명 / 검증 |
|---|---|
| `host_instance_id` | host 시작마다 새 opaque ID; 이전 host의 모든 command/event 거부 |
| `ui_context_epoch` | UI attach/reload마다 host가 증가. Start/Cancel/Snapshot command envelope에 필수; event stream identity와 분리 |
| `session_generation` | host가 Runtime 연결마다 증가. run은 생성 당시 세대에 고정; 연결 교체 시 이전 emitter 폐기 |
| `run_id` | host 내 증가 u32, 1부터; instance와 함께 유일, 재사용·overflow wrap 금지 |
| `request_id` | UI가 snapshot의 high-water보다 큰 u32 token 제안, host admission이 확정. 동일 payload도 재전송은 DUPLICATE_REQUEST, 다르면 REQUEST_CONFLICT |
| `attempt_id` | host 내 증가 u32; run당 한 개; 재연결 시 동일 작업 자동 재실행 금지 |
| `correlation_id` | host 발급 opaque ID, 승인된 진단 연결만. 권한·dedupe 키 아님, 임의 사용자 metadata 금지 |
| `seq` | host가 확정 event마다 1씩 증가, RunCreated=1. 거부·ACK는 증가시키지 않음 |
| `source_seq` | adapter/Runtime ingress stream의 독립 순번. host `seq`와 같은 수라고 가정하지 않음 |

wire identity·epoch·generation·event 순번은 1부터 시작하는 정수 u32이며 0/음수/분수/overflow/문자열 coercion을 거부한다. snapshot의 초기 high-water, event 부재 시 last_seq, terminal_count·byte count는 0을 허용한다. opaque ID는 최대 64 ASCII bytes의 `[A-Za-z0-9_-]+`. fixture의 짧은 ID는 예시이며 실제 발급 방식·entropy는 구현 리뷰 대상이다. 각 run과 단일 attempt의 identity 전체를 결합해 검증한다.

Start envelope 제안은 `{namespace, host_instance_id, ui_context_epoch, request_id, text}`의 closed object이다. host가 현재 session과 실행 출처·limit policy를 붙인다. Cancel은 `{namespace, host_instance_id, ui_context_epoch, run_id, request_id, attempt_id, session_generation}`만 받는다. Snapshot 조회는 `{namespace, host_instance_id, ui_context_epoch}`만 받는다. 이 명세는 새 IPC를 등록하지 않는다.

Start 중복 기록은 최신 수락 request의 정규화 payload와 identity를 메모리에만 보관한다. 정규화는 closed schema 검사 후 text의 원래 Unicode 내용을 보존한 비교이며 trim/Unicode 변환으로 다른 입력을 합치지 않는다. high-water보다 작은 request는 STALE_REQUEST, 최신 동일 ID의 다른 text는 REQUEST_CONFLICT, 같은 text는 DUPLICATE_REQUEST + snapshot 조회 안내다. 새 요청이 BUSY/INVALID_INPUT으로 거부되면 high-water를 올리지 않는다. reload는 high-water를 초기화하지 않는다. 다음 run 수락 때 이전 text·delta·중복 상세를 폐기하되 high-water는 유지한다. host 종료를 가로지르는 dedupe·영속 복구는 보장하지 않는다.

Ingress 검증 순서: trusted channel/session binding → byte/schema/version → instance/run/request/attempt/generation/source binding → source_seq → state/payload/limit → 원자적 commit. 동일 source_seq와 동일 normalized payload는 DUPLICATE_EVENT, 다른 payload는 EVENT_CONFLICT. 순번 gap은 OUT_OF_ORDER이며 자동 buffering/reorder하지 않는다. 최근 ingress 한 건의 bounded payload만 비교하고 더 오래된 순번은 STALE_EVENT로 거부한다. 개별 거부는 원장을 변경하지 않는다. 현재 소유 stream의 conflict/gap/malformed는 별도 host failure 경로로 PROTOCOL_ERROR 한 번을 확정하고 닫는다. 다른 session/위조 channel의 입력은 현재 정상 run을 실패시키지 않는다.

UI는 `(host_instance_id, run_id, seq)` 기준으로 중복 표시를 제거한다. next seq만 적용하며 gap·역행·identity 변경이면 projection을 멈추고 snapshot을 조회한다. snapshot은 해당 run의 전체 identity, `last_seq`, `snapshot_revision`, `request_high_water`, 전체 bounded text, state/error/source/cleanup을 원자적으로 포함한다. run이 없으면 run 필드는 null, high-water와 revision은 host 값이다. subscribe 시작 뒤 도착하는 event를 bounded buffer에 임시 보관하고 snapshot last_seq 이하를 버린 후 연속분만 적용한다. buffer 초과·gap 재발 시 재조회하며 무한 replay/reconnect는 하지 않는다. 오래된 snapshot은 revision과 이미 적용한 run/last_seq로 거부한다. 새 run은 이전 run 출력에 append하지 않는다. event 이력 전체 보존 대신 최신 bounded output과 snapshot을 복구 기준으로 사용한다.

## 5. Event Contract 제안

Canonical event는 host → UI의 closed JSON object이다. Provider/Runtime 입력을 그대로 broadcast하지 않는다. 공통 필드는 아래 예제와 같고 `payload`는 type별 exact keys만 허용한다. `source_seq`, 원본 Provider ID·HTTP body·key·내부 경로는 canonical event에 없다. `source_kind`는 run 동안 고정이다.

```json
{
  "namespace": "agent.text.v1",
  "host_instance_id": "host-fixture-1",
  "run_id": 1,
  "request_id": 1,
  "attempt_id": 1,
  "session_generation": 1,
  "correlation_id": "corr-fixture-1",
  "source_kind": "synthetic",
  "seq": 3,
  "type": "ResponseDelta",
  "payload": {"text": "안녕"}
}
```

| type | payload exact keys | 의미 |
|---|---|---|
| RunCreated | `{}` | queued, 빈 output |
| RunStarted | `{}` | running, 단일 dispatch 경로 시작 |
| ResponseDelta | `{text: string}` | 비어 있지 않은 UTF-8 text append. 완전한 code point 단위, HTML/script/tool 아님 |
| CancelRequested | `{reason: user_cancel \| app_shutdown}` | 출력·dispatch 차단이 commit된 시점 |
| RunSucceeded | `{finish_reason: completed}` | adapter의 명시적 완료와 local cleanup 확인; 제한 도달·truncation을 성공으로 숨기지 않음 |
| RunFailed | `{error: ErrorProjection}` | 아래 closed error object; partial text는 불완전 표시 |
| RunCancelled | `{reason: user_cancel \| app_shutdown, remote_outcome: not_dispatched \| unknown}` | local 종료 확인. 청구 취소·원격 미처리 보장 아님 |

새 CancelRequested는 관측 가능한 중간 상태를 위해 6개 후보 event에 추가한다. Provider Abort/Runtime CancelAck는 host 내부 control 결과이며 canonical 성공/취소 event가 아니다. `finish_reason=completed`일 때 text delta 0개인 빈 응답도 유효하되 UI가 빈 응답임을 표시한다. Provider가 tool call·audio·다른 content를 반환하면 UNSUPPORTED_CONTENT로 실패하고 실행하지 않는다.

## 6. Cancellation, Deadline, Lifecycle

1. host가 현재 identity·UI epoch를 검사하고 CancelRequested를 commit한다. 같은 run의 중복 Cancel은 현재 snapshot만 반환한다. succeeded/failed 뒤 Cancel은 ALREADY_TERMINAL, cancelled 뒤 반복 Cancel은 동일 snapshot이며 event 추가 0이다.
2. commit 이후 input dispatch gate와 delta acceptance를 닫는다. adapter에 idempotent Abort, Runtime에 cancel 신호를 보낸다. 이미 dispatch됐으면 원격 처리/비용은 `unknown`이다.
3. Provider Abort는 **로컬** reader/stream/timer 종료 확인이며 원격 청구 영수증이 아니다. Runtime CancelAck는 동일 instance/attempt/generation에 대한 자기 자원 해제를 뜻한다. ACK만으로 adapter가 멈췄다고 추정하지 않는다.
4. 둘 다 확인하거나 host가 해당 로컬 자원의 종료를 직접 확인한 경우에만 RunCancelled를 확정한다. child가 있는 후보는 소유 handle의 wait/reap까지 필요하다. 확인 불가면 cleanup deadline에서 CLEANUP_UNCONFIRMED로 실패하고 admission을 막는다.
5. 취소 뒤 도착한 delta/completion/오래된 ACK는 폐기한다. late event가 deadline을 연장하지 않는다. 먼저 succeeded/failed가 commit됐다면 취소가 이를 뒤집지 않는다. 먼저 CancelRequested가 commit됐다면 뒤의 완료/일반 Provider 오류가 이를 성공/일반 실패로 바꾸지 않는다; cleanup 결과만 terminal을 결정한다.

deadline은 host monotonic clock으로 계산한다. run 전체 deadline은 admission부터(queued 포함), idle deadline은 RunStarted부터 유효 delta까지와 유효 delta 사이, cleanup deadline은 중단 요청부터다. ingress를 commit할 때도 `now >= deadline`이면 timeout을 먼저 적용한다. 같은 tick의 cancel/completion 순서는 명시된 host dispatch order를 따른다. timeout과 CancelRequested가 겹치면 이미 수락한 취소의 cleanup deadline만 유지한다. wall clock·UI timer·Provider timestamp로 연장하지 않는다. 구체 제품 값은 미결정이며 Fixture 값은 제품 예산이 아니다.

| 상황 | 공통 계약 | 후보별 남는 차이 / 미보장 |
|---|---|---|
| UI reload | UI epoch 교체, 이전 command/projection 거부, snapshot 재조회 | A Runtime 연결 손실이면 RUNTIME_LOST로 실패/abort; B 연결이 실제로 살아 있으면 run 유지 및 새 UI attach. reload 지속성 요구·선택은 ADR에서 결정 |
| Runtime 연결 교체/비정상 종료 | 기존 generation 폐기, 활성 run 실패 및 local cleanup, 새 run 자동 생성 0 | B child 정리는 후보 adapter의 책임, A context loss는 재실행 금지 |
| hidden/minimized | visibility만으로 취소/새 run 생성하지 않음. UI rendering과 execution 분리 | A scheduling·B 자원 사용은 미검증 사항이며 E5c PARTIAL로 성능 보장 불가 |
| Quit/마지막 창 close | intake 먼저 폐쇄, 활성 run cancel(app_shutdown), 정리 상한 적용, 승인 pending 무효 | 정리 불가를 숨기지 않음. 기존 E4e 경로를 후속 구현 때 대조 |
| host crash/restart | 새 instance, 이전 callback 거부, 과거 run 자동 retry 0 | 메모리 상태/terminal 전달은 유실 가능. crash 뒤 cancelled/failed 기록 보장 없음, 원격 결과·비용 unknown |

이 계약의 terminal 1건 보장은 살아 있는 host 원장의 논리 commit에 한한다. crash를 가로지르는 exactly-once 전송·외부 효과·event 전달을 주장하지 않는다.

## 7. Provider Boundary와 제한

아래는 언어 중립 의사 인터페이스이며 실행 코드·SDK signature가 아니다.

```text
prepare(TextRequest, HostPolicy) -> PreparedAttempt | AdmissionError
start(PreparedAttempt, HostCancelToken) -> BoundedStream<AdapterFrame>
abort(AttemptIdentity) -> LocalAbortResult
close(AttemptIdentity) -> LocalCleanupResult

TextRequest = identity + source_kind + text + host-selected policy_ref
AdapterFrame = bound identity + source_seq + one of:
  TextDelta(text) | Completed(completed) | Failed(classification)
LocalAbortResult / LocalCleanupResult = confirmed | unconfirmed
```

`prepare`는 외부 전송 없이 schema·한도·승인 정책을 확인한다. `start`의 dispatch gate는 취소와 같은 host 직렬화 경계를 통과해야 하며 RunStarted와 함께 한 번만 소비한다. gate 소비 뒤 실제 I/O 전에도 cancel token을 확인하지만, 이미 소비된 dispatch의 원격 전송 방지·청구 취소는 보장하지 않는다. 그 경합은 remote_outcome=unknown이다. Runtime/Renderer는 URL, HTTP headers, model name, credential reference, tool 목록, 임의 script를 지정하지 못한다. host allowlist의 policy_ref로만 선택한다. Provider별 frame→AdapterFrame mapping은 실제 Provider 승인 뒤 별도 명세가 필요하다. credential resolver·실제 transport 구현은 없다.

| 경계 | 제안 규칙 / 실패 시 |
|---|---|
| 자격 증명 | 신뢰 host adapter 또는 별도 승인 backend만 보유. React/WebView·Node sidecar·bundle·canonical event·로그에 key 전달 금지. Keychain 저장/읽기도 후속 승인 전 금지 |
| 입력 | whitespace-only 거부, text 원문은 변형하지 않음. UTF-8 bytes 기준 host 상한 + UI 표시 상한을 각각 검사. 현재 Dock의 maxLength=4000은 UTF-8 byte 제한을 대체하지 못함 |
| 출력 | chunk byte 상한, 누적 text byte 상한, ingress/canonical event 개수 상한. delta cap과 별도로 Created/Started/CancelRequested/terminal 최대 4개 공간을 예약해 한도 실패도 종료 가능해야 함. UTF-8 불완전 code point는 bounded decoder에서만 보류, 초과·잘못된 UTF-8은 실패/abort |
| buffering | byte와 event 수로 bounded queue, backpressure 수용 불가 시 BUFFER_LIMIT 실패 및 stream 폐쇄. output truncate 후 성공 금지 |
| rate/concurrency | 활성 run 1개, admission rate 정책은 host에서 검사, 실패는 dispatch 0. UI disable만으로 enforce하지 않음 |
| 비용 | host가 모델·최대 출력·허용 비용 정책을 고정. 설정/가격 근거·quota 승인 없으면 provider admission 차단. token 추정이나 local abort를 청구 hard cap으로 표현하지 않음 |
| timeout | run/idle/cleanup을 독립 제한. 서버 Retry-After는 검증된 bounded hint만 보존하며 자동 retry를 만들지 않음 |
| retry | 기본 0. 401/403·잘못된 입력은 설정/입력 수정 후 새 요청, 429/offline/5xx도 사용자가 명시적으로 새 run 생성. 이전 partial output에 append 금지 |
| 저장·로그 | 본문은 현재 run bounded 메모리와 UI에만. 진단에는 allowlist code·수치·opaque correlation만. request/response·헤더·URL·key·개인 경로·raw SDK error 미기록 |

모든 cap/deadline은 시작 전에 유효한 HostPolicy로 고정하고 run 도중 느슨하게 바꾸지 않는다. 필수 값 누락/0/overflow는 POLICY_UNCONFIGURED로 dispatch 전 거부한다. 자동 context 수집·파일 첨부·대화 저장·원격 embedding은 없다. UI에는 전송 대상·데이터 범위와 출처를 설명하는 후속 P2.2 gate가 필요하다.

## 8. Error Model과 안전한 Projection

ErrorProjection exact keys: `{code, phase, retry_action, partial_output, remote_outcome}`. `phase`는 `admission | dispatch | stream | cleanup | lifecycle`, `retry_action`은 `none | new_request | fix_input | configure`, `partial_output`은 boolean, `remote_outcome`은 `not_dispatched | unknown`. UI 메시지는 code별 고정 한국어 사전에서 생성한다. 임의 message/stack/cause/raw body 필드를 받지 않는다. unknown classification은 PROVIDER_ERROR로 닫는다.

| code / 분류 | 처리 / retry_action |
|---|---|
| INVALID_INPUT, INPUT_LIMIT | admission 거부 / fix_input |
| POLICY_UNCONFIGURED, PROVIDER_DISABLED | dispatch 0 / configure |
| BUSY, RATE_LIMITED, COST_LIMIT | admission 거부 / new_request (policy 충족 뒤 수동) |
| DUPLICATE_REQUEST, REQUEST_CONFLICT, STALE_REQUEST, STALE_SESSION, INVALID_IDENTITY | command 거부, 정상 활성 run 불변 / none |
| DUPLICATE_EVENT, EVENT_CONFLICT, STALE_EVENT, OUT_OF_ORDER, ALREADY_TERMINAL | ingress/command 결과, UI terminal event가 아님 / none |
| AUTH_REQUIRED, PERMISSION_DENIED | adapter 인증/인가 실패 → RunFailed / configure |
| PROVIDER_RATE_LIMIT | RunFailed, 자동 재시도 없음 / new_request |
| QUOTA_EXCEEDED | RunFailed, quota 정책 확인 필요 / configure |
| OFFLINE, PROVIDER_UNAVAILABLE, PROVIDER_ERROR | RunFailed / new_request |
| RUN_TIMEOUT, IDLE_TIMEOUT | RunFailed, abort/cleanup / new_request (cleanup 확인 뒤) |
| OUTPUT_LIMIT, BUFFER_LIMIT, PROTOCOL_ERROR, UNSUPPORTED_CONTENT | RunFailed, abort / none |
| RUNTIME_LOST | RunFailed, 이전 generation 폐기 / new_request (새 session/cleanup 확인 뒤) |
| CLEANUP_UNCONFIRMED | RunFailed, admission 계속 차단 / none |

같은 code라도 admission 거부와 run 실패는 서로 다른 결과 타입이다. run 생성 전 오류에 RunFailed/가짜 run_id를 만들지 않는다. 부분 응답이 있으면 화면에 “중단된 응답”으로 남길 수 있으나 완료로 취급하지 않는다. 민감정보 검증은 합성 canary로 raw provider error가 UI·stderr·telemetry에 새지 않는지 후속 Fixture에서 검사한다.

## 9. Spatial Presence UI Integration Contract

현재 main의 Command Dock은 로컬 초안만 보유하며 send/mic/attach가 disabled이고 Particle는 Preview다. 이번 변경은 이 상태를 유지한다. 후속 연결은 Dock 입력→host admission→snapshot/event→text projection이라는 최소 경로이며 Orb 디자인·shader·animation 교체를 포함하지 않는다.

| host/connection 상태 | 후속 UI 표시·허용 동작 |
|---|---|
| provider 미설정 / Preview | 전송 준비 중, Preview 고정 표시. Preview가 실제 run 생성/연결 상태를 바꾸지 않음 |
| synthetic 준비 | “합성 응답 / 네트워크 전송 없음”; 기존 Fake Runtime Diagnostics와 text Fixture를 구분 |
| admission 응답 대기 | 초안 보존, 중복 send 잠금; UI 자체로 queued 확정 금지 |
| queued/running | 대기/응답 중, Cancel 제공. delta는 동일 run에만 append; 가짜 완료율 없음 |
| cancel_requested | 취소 중, send 차단; Abort 응답만으로 “취소 완료” 금지 |
| succeeded | 완료 또는 빈 응답 안내. host cleanup 확인 후 새 전송 가능 |
| cancelled/failed | 취소/고정 오류 문구, partial 표시, 허용된 수동 새 요청. 원격 취소·청구 중단 보장 없음 |
| 연결 손실 / snapshot gap | “상태 확인 중”, 마지막 확인 상태와 구분. 임의 failed/succeeded 생성 금지 |

로컬 연결 상태 `unconfigured | attached | reconnecting | disconnected`와 run 상태는 별개다. attached는 Provider 인증·실제 호출 가능을 증명하지 않는다. text 응답을 `speaking`으로 매핑하지 않고 listening/mic/voice 상태도 생성하지 않는다. output은 text node로 표시하고 HTML 실행·자동 링크 열기·모델 승인 적용은 금지한다. 저장 기능이 없는 현재 범위에서 영속 대화 이력처럼 표시하지 않는다.

## 10. Fixture·후속 최소 단위와 미결정

[결정적 Fixture 설계](p2-0-contract-fixtures.md)에 입력·순서·oracle·미실행 상태를 정의한다. 실행 가능한 테스트나 Runtime 모듈은 추가하지 않았다.

| 후속 최소 단위 | 완료 조건 / gate |
|---|---|
| P2.1a host contract + synthetic adapter | 사용자 계약 승인과 새 구현 Issue; state/ordering/cancel/error Fixture 구현·검증. 실제 Provider dispatch 0 |
| P2.1b 선택된 Runtime 연결 | Runtime ADR과 reload 지속성 요구 승인; identity/abort/cleanup parity 및 선택 후보의 lifecycle 검증 |
| P2.1c Dock text projection | 앞 두 단위와 UI 연결 승인; source 표시·partial·snapshot 복구·재시도 동작, 기존 Orb와 diagnostics 회귀 |
| P2.2 실제 Provider | 공급자·모델·비용/한도·credential·전송 동의·보존 정책 승인 및 sanitize/timeout 검증 후 별도 범위 |

이 표는 한 P2.1 구현 Issue 안의 검토 가능한 작업 순서 제안이며 지금 새 Issue를 시작하지 않는다. Runtime-independent host 부분과 선택 후보 통합 gate를 구분한다. A/B 자원 우열, reload 중 지속 여부의 제품 요구, production limit/deadline, key 저장/배포 경로, Provider mapping, crash 후 durable recovery는 미결정이다.

```text
CONTRACT_STATUS: PROPOSED
FIXTURES_EXECUTED: NO
RUNTIME_IMPLEMENTATION_ALLOWED: NO
RUNTIME_SELECTED: NO
PROVIDER_CALL_ALLOWED: NO
REAL_AUDIO_INTEGRATION_ALLOWED: NO
MAIN_MERGED: NO
NEXT_PHASE_STARTED: NO
```
