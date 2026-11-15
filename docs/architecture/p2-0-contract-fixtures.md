# P2.0 결정적 Fixture 설계 · Deterministic Contract Fixtures

**PROPOSED · NOT_RUN · 2026-11-15** · [Issue #46](https://github.com/Kim-Ji-Won/AETHER/issues/46) · [계약 명세](p2-0-agent-runtime-contract.md)

이 문서는 테스트 입력·가상 실행 순서·기대 oracle의 설계다. 테스트 runner, 앱 코드, 실제 Provider adapter를 구현하거나 실행하지 않았다. 표의 기대 결과는 PASS 보고가 아니다. E4/E5 기존 FakeRun trace를 새 text 계약의 테스트 결과로 재분류하지 않는다.

## 1. Fixture 실행 모델 제안

- 독립 메모리 ledger, 가상 monotonic clock, 명시적 FIFO host scheduler, 수동 release barrier를 사용한다. `sleep`, 실제 시간·랜덤 scheduler·네트워크·GUI·child process를 쓰지 않는다.
- 매 case마다 host `host-fixture-1`, UI epoch 1, session generation 1, run/request/attempt 1, correlation `corr-fixture-1`에서 초기화한다. restart case만 instance를 2로 바꾼다. 출처는 `synthetic`이다.
- host input은 Start/Cancel/Lifecycle/ClockAdvance, adapter input은 TextDelta/Completed/Failed, control은 AbortResult/RuntimeCancelAck/ResourceReleased로 분리한다. input enqueue와 host commit을 별도 step으로 기록한다.
- 입력 fixture는 각 step의 identity/clock/바리어 순서를 명시한다. canonical seq는 host가 생성하고 source_seq는 adapter emitter가 생성한다. canonical RunCreated/RunStarted가 source_seq를 소비하지 않는다.
- `before_dispatch`, `after_started`, `after_delta_1`, `before_terminal_commit`, `before_abort_confirm`, `before_runtime_ack`를 barrier로 둔다. cancel/completion 양쪽 순서를 별도 case로 고정한다.
- adapter spy는 dispatch count, abort 요청 count, local stream open/closed, cleanup ack, accepted/rejected ingress, external_call_count를 기록한다. 모든 case의 external_call_count=0을 기대한다. 여기서 dispatch count는 합성 adapter 호출 수다.
- identity나 policy가 거부되면 resource 생성/dispatch/output append=0을 기대한다. host lock 안에서 외부 wait를 수행하지 않는지도 후속 구현 구조 리뷰 대상이다.

### Fixture 전용 policy

| 값 | 고정 fixture 값 | oracle |
|---|---:|---|
| active run | 1 | 두 번째 Start BUSY, 새 run/dispatch 0 |
| input UTF-8 | 32 bytes | 32 허용, 33 거부; 공백만 거부 |
| delta UTF-8 | 8 bytes | 8 허용, 9 거부 |
| total output UTF-8 | 32 bytes | 32 허용, 33 실패/abort |
| pending ingress queue | 2 frames / 16 text bytes | 어느 cap이든 초과 시 BUFFER_LIMIT |
| accepted text deltas | 8 | 9번째 delta OUTPUT_LIMIT, terminal 공간 별도 확보 |
| run deadline | admission + 1000ms | now=1000부터 RUN_TIMEOUT |
| idle deadline | start/마지막 유효 delta + 100ms | now=100부터 IDLE_TIMEOUT |
| cleanup deadline | stop 요청 + 50ms | 50ms 경계에 미확인이면 CLEANUP_UNCONFIRMED |
| rate admission | 1000ms fixed window당 최대 2 accepted Start | 세 번째 RATE_LIMITED, 1000ms 경계 reset |
| cost policy | 실제 화폐가 아닌 가상 budget 2 units, Start당 reserve 1 | reserve 부족이면 COST_LIMIT, dispatch 0 |

이 수치는 작은 경계 테스트를 위한 값이며 제품 timeout·rate·가격·성능 예산으로 채택하지 않는다. rate/cost case 외에는 quota가 충분한 새 policy를 사용한다. 가상 cost는 과금 측정이 아니며 실제 취소 환불 oracle도 없다. 정상 trace에서는 clock=0에 명시적 frame을 순차 투입해 시간 경합을 제거한다.

## 2. 완전한 정상 trace 예제

모든 step은 위 고정 identity를 사용한다. 표의 event payload는 명세의 공통 envelope와 결합한다. Adapter ingress의 source_seq=1은 canonical seq=3에 대응한다.

| step | 입력 / barrier | 기대 state | 기대 canonical event | 부가 oracle |
|---:|---|---|---|---|
| 1 | Start(request=1, text=`질문`) | queued | 1 RunCreated `{}` | dispatch=0, output 빈 문자열 |
| 2 | before_dispatch release | running | 2 RunStarted `{}` | dispatch=1, terminal_count=0 |
| 3 | TextDelta(source_seq=1, `안녕`) | running | 3 ResponseDelta `{text: 안녕}` | output=`안녕`, 6 bytes |
| 4 | TextDelta(source_seq=2, `!`) | running | 4 ResponseDelta `{text: !}` | output=`안녕!`, 7 bytes |
| 5 | Completed(source_seq=3), ResourceReleased | succeeded | 5 RunSucceeded `{finish_reason: completed}` | terminal_count=1, cleanup=confirmed |
| 6 | 같은 Completed 재전송 | succeeded | 없음 | DUPLICATE_EVENT, seq=5, output 불변 |
| 7 | Cancel | succeeded | 없음 | ALREADY_TERMINAL, terminal_count=1 |

Completed 뒤 cleanup이 지연되면 즉시 succeeded로 표시하지 않는다. 명시적 완료를 pending으로 보유하되 run deadline과 완료 시점+50ms cleanup deadline을 유지한다. stream idle deadline은 종료한다. 그 사이 cancel은 정상적으로 먼저 commit될 수 있으나 cleanup deadline을 연장하지 않는다. 빈 응답 case는 step 3/4 없이 Completed(source_seq=1)와 cleanup 확인 후 canonical seq=3으로 완료한다.

## 3. 취소 trace와 race oracle

| case | 고정 입력 순서 | 기대 oracle |
|---|---|---|
| C01 queued 취소 | Start → Cancel (host가 미생성 자원 확인, ACK 없음) | Created seq1 → CancelRequested seq2 → Cancelled seq3; dispatch=0, cleanup=not_required, remote_outcome=not_dispatched |
| C02 delta 후 취소 | 정상 step1~3 → Cancel → Abort confirmed → RuntimeAck | seq4 CancelRequested, seq5 Cancelled; output=`안녕`을 partial로 표시, remote_outcome=unknown |
| C03 취소가 완료보다 먼저 | before_terminal_commit 대기 → Cancel commit → Completed → 양쪽 cleanup 확인 | Completed 거부, RunSucceeded 0, RunCancelled 1 |
| C04 완료가 취소보다 먼저 | Completed+cleanup commit → Cancel | RunSucceeded 1, Cancel ALREADY_TERMINAL, state 불변 |
| C05 중복 취소 | C02의 Cancel을 3회 → 양쪽 cleanup 확인 → Cancel | CancelRequested/RunCancelled 각 1, 중복은 snapshot만, abort effect 중복 0 |
| C06 한쪽 ACK만 도착 | Cancel → Abort confirmed → clock=50, RuntimeAck 없음 | failed(CLEANUP_UNCONFIRMED), admission 차단; 취소 완료 표시 0 |
| C07 늦은/위조 ACK | Cancel → 다른 attempt/generation ACK → clock=50 → 올바른 late ACK | 잘못된 ACK 무효; terminal failed 유지, 확인된 cleanup만 snapshot_revision 증가 가능 |
| C08 abort 미확인 | Cancel → RuntimeAck만 → Abort unconfirmed → clock=50 | C06과 동일; RuntimeAck만으로 cancelled 0 |
| C09 cancel 이후 delta/error | Cancel → delta → 일반 ProviderFailed → 양쪽 cleanup 확인 | output append 0, 일반 RunFailed 0, Cancelled 1 |
| C10 cleanup deadline 경계 | Cancel at0 → 양쪽 확인 at49 / at50 | 49는 cancelled; 50은 deadline 우선 failed, 뒤 cleanup 진단만 갱신 |
| C11 완료 후 cleanup 대기 | Completed → cleanup pending → Cancel → cleanup 확인 | 성공 commit 전 취소 승리, Cancelled 1 |
| C12 완료 후 cleanup 미확인 | Completed at0 → Cancel at49 → clock=50 | cleanup deadline 연장 없이 failed(CLEANUP_UNCONFIRMED); pending completion 뒤 delta도 수락 0 |

가상 ResourceReleased는 OS wait/reap 증거가 아니다. B child teardown 및 A context 정리는 선택 Runtime의 후속 integration test가 따로 필요하다.

## 4. Identity·순서·입력 방어 Matrix

| ID | 입력 변형 | 기대 oracle |
|---|---|---|
| I01 | 같은 request/text 재전송, dispatch 전/후 각각 | DUPLICATE_REQUEST, 추가 run/dispatch 0; 최신 snapshot 조회 가능 |
| I02 | 같은 request, 다른 text | REQUEST_CONFLICT, 기존 run 불변 |
| I03 | 이전 request, 새 run이 이미 존재 | STALE_REQUEST, high-water/현재 output 불변 |
| I04 | instance/run/request/attempt/generation 각 1개씩 변조 | INVALID_IDENTITY 또는 STALE_SESSION, 정상 run 실패/event 추가 0 |
| I05 | 같은 source_seq+payload / 다른 payload | duplicate는 무변경; 현재 bound stream의 conflict는 별도 PROTOCOL_ERROR terminal 1 |
| I06 | source_seq gap, 오래된 seq, 역순 | gap은 stream protocol failure; 보존 상세 이전 seq는 STALE_EVENT, output 중복 append 0 |
| I07 | unknown namespace/type/key, missing key, nested 대신 scalar, null, bool, numeric string | parser 거부; 현재 bound malformed stream만 별도 실패, 미인증 입력은 현재 run 불변 |
| I08 | 0/-1/1.5/2^32 identity/seq, ID 길이65, 비ASCII opaque ID | 거부, coercion/overflow wrap 0 |
| I09 | terminal 이후 새 seq의 delta/완료/실패 | ALREADY_TERMINAL, canonical seq/state/output/terminal_count 불변 |
| I10 | 두 번째 Start, 기존 active/cleanup unconfirmed | BUSY, 추가 dispatch 0; 첫 run cleanup 확인 전 해제 금지 |
| I11 | synthetic run에 provider source 주장, 다른 channel | identity/source binding 거부; 실제 Provider/승인 권한 생성 0 |
| I12 | 새로운 request/token 공간 소진 | admission 거부, wrap 0; 자동 restart 0 |

## 5. Deadline·limit·오류 Matrix

| ID | 입력·조작 | 기대 oracle |
|---|---|---|
| E01 | queued에서 clock=999/1000 | 999 queued 유지; 1000 RUN_TIMEOUT, dispatch 0 |
| E02 | running, delta 없음 at99/100 | 99 유지; 100 IDLE_TIMEOUT, abort/cleanup 시작 |
| E03 | 유효 delta at90, duplicate at180, clock=190 | duplicate가 idle clock 갱신하지 않음, 190 timeout |
| E04 | 전체 deadline 직전까지 90ms마다 delta; case 전용 total output=128 bytes, delta cap=16 | idle 갱신과 무관하게 1000 RUN_TIMEOUT; run deadline 연장 0 |
| E05 | Cancel commit 뒤 원래 run deadline 경과 | cancel의 cleanup deadline만 적용; RUN_TIMEOUT으로 취소 이유 덮어쓰기 0 |
| E06 | 정확히 deadline에 Completed/Delta 도착 | timeout 우선; 새 text/success commit 0 |
| E07 | input/chunk/total bytes cap-1/cap/cap+1, 한글/emoji 포함 | byte 기준 경계. UI maxLength/문자 개수로 통과시키지 않음 |
| E08 | UTF-8 code point 분할 / 잘못된 UTF-8 / buffer cap 초과 | bounded decoder 재조립 또는 PROTOCOL_ERROR/BUFFER_LIMIT; 깨진 text 출력 0 |
| E09 | delta 개수 8/9, terminal 직전 cap | 9번째 거부/OUTPUT_LIMIT; 실패 terminal 공간 확보, terminal_count=1 |
| E10 | queue frame 수 또는 bytes만 독립 초과 | backpressure 불가 시 BUFFER_LIMIT; 무한 queue/임의 drop 후 성공 0 |
| E11 | 401/403/429/quota/offline/5xx/미분류 합성 오류 | AUTH_REQUIRED/PERMISSION_DENIED/PROVIDER_RATE_LIMIT/QUOTA_EXCEEDED/OFFLINE/PROVIDER_UNAVAILABLE/PROVIDER_ERROR; raw body projection 0 |
| E12 | EOF만 도착 / unsupported tool/audio content | PROTOCOL_ERROR / UNSUPPORTED_CONTENT, 도구 실행 0 |
| E13 | policy 누락/0/overflow, rate 초과, cost 부족 | 각각 admission 거부, dispatch=0. 실제 비용 수치 생성 0 |
| E14 | failed(partial 있음) → 사용자가 새 request | 새 run/attempt, output 빈 상태로 시작; 자동 retry=0, 이전 output append 0 |
| E15 | 완료 후 늦은 timeout/오류, terminal 후 cleanup 진단 | terminal 불변, cleanup 진단만 revision 증가; 두 번째 terminal 0 |

E11의 HTTP 숫자는 가상 분류 입력이다. 실제 HTTP 요청·SDK 동작·Provider 지원을 검증한 것이 아니다. rate/cost는 별도 host policy 검증으로 token 추정과 실제 청구의 차이를 유지한다.

## 6. Lifecycle·UI·보안 Matrix

| ID | 시나리오 | 기대 oracle / 후속 실환경 검증 |
|---|---|---|
| L01 | A profile: reload → context loss | UI epoch 증가, 이전 command 거부, active run RUNTIME_LOST, abort, 자동 재실행 0 |
| L02 | B profile: UI reload, Runtime 유지 | epoch만 증가, run/session_generation 유지, snapshot 재부착. 기존 세대 producer를 단순 UI reload 때문에 거부하지 않음 |
| L03 | B profile: Runtime 연결도 끊김 | generation 폐기, failed/cleanup; old ACK/result 무효, child reap 실제 검증은 NOT_RUN |
| L04 | Quit/마지막 창 close, active/idle 각각 | intake 먼저 차단, app_shutdown 취소, cleanup 여부 분리; 새 Start 0 |
| L05 | host restart와 old callback | 새 instance, 이전 상태 복원/자동 retry 0. 이전 원격 처리·청구 unknown, 가짜 terminal 생성 0 |
| L06 | hidden/minimized 토글 | visibility로 상태·dispatch/seq 변경 0; 실제 timing·CPU/RSS 결론 없음 |
| U01 | UI seq gap/duplicate, 오래된 snapshot | 중복 text 0, gap에서 projection 정지·재조회, revision 역행 0 |
| U02 | subscribe→event seq3→snapshot last_seq3→event4 | seq3 중복 append 0, snapshot 전체 text+event4만 적용 |
| U03 | terminal snapshot과 이전 running snapshot 순서 역전 | terminal projection을 running으로 되돌리지 않음 |
| U04 | Preview/synthetic/provider 각각 및 provider disabled | Preview event 0, synthetic label 유지, 미승인 provider dispatch 0 |
| U05 | admission 통신 끊김, run 존재 여부 미확인 | 초안 보존·상태 확인, 자동 새 request 0; 원장에 없는 failed 표시 0 |
| S01 | 오류에 `CANARY_SECRET_DO_NOT_LOG`, 가상 URL/query/header/path 삽입 | UI/error/diagnostics/stdout/stderr/telemetry에 canary 0; ErrorProjection allowlist만 허용 |
| S02 | output에 `<script>`, `approved=true`, tool 지시 삽입 | text로만 표시, 코드·링크 자동 실행/승인/권한 생성 0 |
| S03 | Renderer가 URL/model/key reference/extra metadata 제시 | closed schema 거부, secret resolver 접근/전송 0 |
| S04 | 새 run 수락 또는 host 종료 뒤 과거 text 조회 | 이전 bounded text·중복 상세 폐기, 메모리 영속화·로그 저장 0; 현재 terminal run은 다음 run 전까지 snapshot에 유지 |

L01/L02의 profile은 논리 dependency double이며 A/B 구현·Runtime 선정·실제 reload/reap PASS를 뜻하지 않는다. OS crash/Renderer crash, 실제 endpoint·Keychain·청구·서명·다른 Mac·UI 접근성/렌더링은 별도 승인된 integration 검증에 남긴다.

## 7. 공통 assertion과 후속 검증 보고 형식

각 case는 state/output/identity/seq/terminal_count 외에 dispatch·abort·pending resource·cleanup_status·snapshot_revision·external_call_count·redacted diagnostic만 검사한다. 모든 prefix에서 아래 invariant를 검사한다.

1. 허용된 전이만 존재하고 terminal_count는 0 또는 1이다. terminal 뒤 state/seq/output/error가 바뀌지 않는다.
2. 거부된 ingress는 canonical event를 추가하지 않는다. 별도 host failure는 그 자체의 한 번의 terminal commit으로 구분한다.
3. cancel commit 이후 delta 수락·dispatch gate 소비·성공 commit이 없다.
4. cleanup 미확인 상태에서 다음 run을 시작하지 않는다. Runtime ACK만으로 Provider 정리를 가장하지 않는다.
5. 출처·세대·identity가 다른 데이터는 현재 output에 섞이지 않는다. UI projection은 host보다 앞서 terminal을 만들지 않는다.
6. 실제 external call, credential 접근, tool/audio effect는 0이다. 합성 문자열은 로그 정책 검사 외에 실제 개인정보를 포함하지 않는다.

후속 구현 보고에는 `case ID / contract version / implementation SHA / policy / ordered steps / expected / observed / PASS|FAIL|BLOCKED|NOT_RUN`을 남긴다. fixture 정책, source SHA, 실제 호출 여부와 integration 범위를 분리한다. 현재는 전 case **NOT_RUN — 설계만 작성**이다.

## P2.1a 구현 추적 · 2026-11-15

위 NOT_RUN은 P2.0 설계 시점의 기록이다. 사용자 개발 기준선 수용 후 [Issue #48 구현/검증 기록](p2-1a-agent-run-contract.md)에서 C01–C12/I01–I12/E01–E15의 Host 검증과 후속 NOT_RUN 범위를 분리한다. C01의 cleanup 확인은 자원이 없다는 host 판정이며 Provider/Runtime의 가짜 ACK가 아니라는 점을 명확히 했다. 54개 전체가 실행됐다는 의미가 아니다.
