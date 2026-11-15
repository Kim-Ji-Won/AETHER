# P2.1a Rust Host 계약과 Synthetic Adapter

[Issue #48](https://github.com/Kim-Ji-Won/AETHER/issues/48) · **HOST_LIBRARY_IMPLEMENTED / INTEGRATION_NOT_RUN** · 2026-11-15

## 기준과 승인

[P2.0 PR #47](https://github.com/Kim-Ji-Won/AETHER/pull/47)의 승인 HEAD `6b44193ef5c00cbf48ba38274a0230868a48556e`와 예상 base `f44a6c07cec2940633c63e67613dd1259549b1f7`를 원격에서 확인했다. 승인 문서 4개, 충돌 없음, 리뷰 차단 없음, Ready 전환 후 Governance 성공을 확인하고 Squash Merge했다.

- 병합 main / 이번 구현 base: `0e182bfb46f7c036c0c81d5ecb8c5970d289c2ae`
- 승인 HEAD와 main의 전체 tree: 모두 `acc1191c49945045bf49e85da2e50fdbee66e3c7`, `git diff --exit-code` 동일
- [main push Governance](https://github.com/Kim-Ji-Won/AETHER/actions/runs/43718541078): completed / success
- 로컬 main fast-forward 동기화 후 별도 `feat/48-agent-run-contract` Worktree에서 구현

사용자는 P2.0을 **개발 기준선**으로 수용했다. [원 계약](p2-0-agent-runtime-contract.md)의 설계 당시 PROPOSED/NOT_RUN 기록을 실제 Runtime 검증으로 바꾸지 않는다. 이번 P2.1a 구현은 별도 Draft 리뷰 대상이며 병합 승인이 없다. Runtime A/B 선정, 실제 Provider, P2.1b/c는 시작하지 않았다.

## 구현 구조

| 파일 | 책임 |
|---|---|
| `apps/desktop/src-tauri/src/lib.rs` | 앱 binary와 분리된 Rust library 진입점. `agent_text`만 export |
| `apps/desktop/src-tauri/src/agent_text/mod.rs` | Host 원장, admission, identity, 단일 dispatch, ordered events, snapshot, cancel/cleanup/deadline |
| `apps/desktop/src-tauri/src/agent_text/types.rs` | `agent.text.v1`, closed enum/typed payload, ErrorProjection, 명시적 Policy |
| `apps/desktop/src-tauri/src/agent_text/wire.rs` | Start/Cancel/Snapshot/Frame의 closed post-JSON schema 검사, bounded UTF-8 decoder |
| `apps/desktop/src-tauri/src/agent_text/synthetic.rs` | bounded text queue, 명시적 `step` barrier, local abort/close |
| `apps/desktop/src-tauri/src/agent_text/tests.rs` | 가상 clock과 제어 가능한 순서로 실행하는 46개 테스트 |

Host의 배타적 `&mut Host` 호출이 직렬화 지점이다. 내부에는 외부 I/O·스레드·wall-clock·sleep이 없다. 이후 동시 호출 driver는 기존 `lifecycle Gate → ledger` 순서와 동기화 책임을 유지해야 한다. library가 새로운 global Runtime이나 Tauri command를 등록하지 않는다. 기존 `main.rs`, FakeRun, lifecycle, Approval, Swift, Sidecar, UI, capability/CSP, Cargo 의존성·lockfile은 변경하지 않았다.

Rust `Event`는 namespace·identity·source_kind·seq·typed payload를 보유한다. 원 계약 JSON envelope의 실제 IPC serialization/broadcast 및 UI projection은 P2.1b/c에 남는다. wire parser는 Tauri가 JSON을 파싱한 뒤의 검사이며 transport 사전 allocation 제한이 아니다. 현재 등록된 IPC 경로가 없으므로 새 Renderer 권한이 생기지 않는다.

## 구현 전 정리한 Bootstrap / Epoch

기존 `lifecycle.rs::Gate`의 최초 epoch 0과 검사 규칙은 그대로다. 새 library의 `Host::new`는 unbootstrapped 상태로 시작하며, 이때 wire epoch 0 또는 추측한 1을 넣어도 Start/Snapshot/Cancel은 거부한다.

1. 신뢰 host는 기존 lifecycle에서 얻은 instance를 Host 생성에 사용한다. 이 값은 UI 입력이 아니다.
2. **기존 Security Gate 검증 아래에서만 호출해야 하는** 내부 `attach(instance, legacy_epoch, lifecycle_live)`가 instance·live·epoch 비역행을 검사하고 `wire_epoch = legacy_epoch + 1`을 checked 계산한다. 최초 legacy 0은 Agent wire 1에만 매핑하며 기존 Gate 값을 수정하지 않는다.
3. 동일 legacy epoch 재부착은 idempotent다. request high-water, run, revision을 초기화하지 않는다. epoch overflow·과거 epoch·다른 instance·closing은 거부한다.
4. reload로 legacy epoch가 증가하면 wire epoch도 증가한다. 이전 UI command/snapshot 요청을 거부하고 새 session으로 현재 bounded snapshot을 읽는다. UI 교체만으로 Runtime generation을 교체하지 않는다.
5. 실제 Runtime loss는 별도 `runtime_lost` 입력으로 generation을 폐기한다. old ACK/result는 거부하고, 신뢰 host의 소유 자원 확인만 cleanup을 갱신할 수 있다.

`attach`는 신뢰 경계의 내부 API이며 호출 인자 자체가 기존 Gate 검증을 증명하지 않는다. 실제 앱 연결은 이번 범위 밖이다. 후속 bridge에서 기존 `with_request/with_live` 경로를 우회하거나 UI가 lifecycle_live/legacy_epoch를 자체 승인하도록 구현하면 안 된다. 이 단계의 bootstrap/reload 테스트는 Host 논리 검증이며 실제 WebView reload 증거가 아니다.

## State / Cancellation / Timeout

`queued → running → succeeded/failed`, `queued/running → cancel_requested → cancelled`를 구현했다. 모든 terminal은 count 1로 한 번만 확정한다. terminal 뒤 state·seq·output·error는 불변이고, 늦은 cleanup 진단만 snapshot revision을 올릴 수 있다. 정상 완료에도 explicit Completed와 local adapter/runtime release가 필요하다.

**C01 queued 취소:** dispatch 이전에는 자원이 없다. CancelRequested와 RunCancelled를 같은 host 호출에서 순서대로 기록하고 `cleanup=not_required`, `remote_outcome=not_dispatched`로 끝낸다. adapter 생성·Abort·가짜 Runtime ACK·cleanup timer가 필요 없다. 이후 dispatch/late result는 상태를 되돌리지 못하며 새 요청은 허용한다.

dispatch 이후 취소는 adapter release와 Runtime release를 분리한다. 하나만 확인되면 cancel_requested를 유지하고 cleanup deadline에서 CLEANUP_UNCONFIRMED로 실패한다. 다음 run은 정리 확인 전 BUSY로 거부한다. completion과 cancel은 호출 순서로 승자가 결정되며, completion-pending 중 취소는 기존 cleanup deadline을 연장하지 않는다. remote_outcome=unknown은 원격 전송·청구 취소를 보장하지 않는 보수적 계약 필드다. Synthetic 실행에 실제 청구가 있다는 뜻은 아니다.

Clock은 `advance_to(monotonic_ms)`로 주입한다. driver는 같은 tick에서 **clock 갱신/만료 처리 → 입력 전달** 순서를 지켜야 한다. 경계 `now >= deadline`에서 만료가 우선하며 역행/산술 overflow는 거부한다. 새 deadline은 queued admission, RunStarted/유효 delta, stop/completion 시점에서 생성하고 duplicate/late event로 늘리지 않는다. production timer 연결은 미구현이다.

## Bounded Adapter / Error / 정책

SyntheticAdapter는 고정 progress가 아닌 UTF-8 text delta를 queue에 넣고 `step` 한 번당 frame 한 개만 전달한다. `step`을 호출하지 않으면 text가 원장에 반영되지 않아 barrier 순서를 재현할 수 있다. dispatch는 한 번만 소비되며 queue의 frame/byte cap, chunk/누적 output byte cap, delta count cap을 검사한다. terminal을 기록할 공간은 별도로 예약한다.

Policy는 신뢰 host가 명시적으로 넣는 값이며 제품 default를 만들지 않았다. 누락에 대응하는 typed 값 부재는 생성 불가, 0/과도한 값은 PolicyUnconfigured다. allocation/time 상한은 코드 안전 한계이며 P0 측정에서 승인된 예산이 아니다. rate는 고정 window의 accepted Start 수, cost는 **합성 budget unit**을 admission에서 예약하는 결정적 모델이며 취소 시 반환하지 않는다. 실제 가격·token 추정·Provider quota를 구현한 것이 아니다.

ErrorProjection은 code/phase/retry_action/partial_output/remote_outcome만 가진다. Provider status의 분류는 합성 정수 입력에 대한 순수 함수이며 HTTP 요청이 없다. 원본 error body/message/header/path/key를 보관하는 필드가 없다. 문자열 code의 미분류 값은 ProviderError로 정규화한다. 기본 자동 retry는 0이고 수동 새 요청은 새 run/attempt와 빈 output에서 시작한다.

## Fixture 범위 · 실제 결과

[54개 설계](p2-0-contract-fixtures.md)의 ID를 유지했다. **P2.1a 범위 39개 Fixture의 Host unit/contract 검증 PASS**, 별도 보완 테스트 7개를 합해 Rust library **46 tests PASS**다. 각 `cNN_`, `iNN_`, `eNN_` 테스트 이름이 아래 ID에 대응한다. 54/54 PASS가 아니다.

| Fixture | 결과 | 실제 검증 범위 |
|---|---|---|
| C01, C02, C03, C04, C05, C06 | PASS | queued 무자원 취소, delta 후 두 ACK, 양쪽 race 순서, 중복 abort, 미확인 cleanup |
| C07, C08, C09, C10, C11, C12 | PASS | 잘못된/늦은 ACK, 한쪽 ACK, late result/error, deadline 경계, completion-pending 취소 |
| I01, I02, I03, I04, I05, I06 | PASS | 중복/conflict/high-water, 전체 identity, duplicate/gap/stale source_seq |
| I07, I08, I09, I10, I11, I12 | PASS | closed schema/type/range, terminal 불변, BUSY, source binding, counter 소진 |
| E01, E02, E03, E04, E05, E06 | PASS | queued/idle/total/cancel deadline, duplicate 비연장, 동일 tick 및 역행 |
| E07, E08, E09, E10, E11 | PASS | UTF-8 byte 경계/decoder, delta/queue cap, 안전 오류 분류 |
| E12, E13, E14, E15 | PASS | EOF/unsupported 실패, policy/rate/합성 cost, 수동 retry, 늦은 timeout/cleanup |
| L01–L06 | NOT_RUN | 실제 A/B reload/child/visibility/Quit/crash/restart 통합. 논리 shutdown/generation 보완 테스트는 별개 |
| U01–U05 | NOT_RUN | 실제 UI snapshot/event 복구·Preview·통신 단절. Host snapshot은 unit으로만 확인 |
| S01–S04 | NOT_RUN (전체 integration) | UI/log/telemetry/credential/rendering/영속성 통합. S01/S03/S04의 Host sanitizer·closed schema·현재 run 교체 부분만 보완 unit PASS |

7개 보완 테스트: 초기 bootstrap 경계, reload snapshot, 정상 Synthetic text trace, 빈 응답, S01/S03/S04 Host subset, 논리 shutdown/generation loss, Cancel/Snapshot wire 검사. 실제 Renderer/Node lifecycle PASS로 집계하지 않는다.

## 실행한 검증과 한계

저장소 root 기준:

```sh
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --offline --lib
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --offline
python3 scripts/prepare_node_sidecar.py --verify-only
python3 scripts/validate_governance.py
git diff --check
git diff --cached --check
git show --format= --check HEAD
```

- Rust library 46 / 기존 binary regression 19 PASS, doc tests 0. 기존 회귀에는 FakeRun 10, Native Approval 3, Lifecycle Gate 1, Sidecar 4, measurement 1이 포함된다. Native Approval 원장 unit PASS는 사람이 실제 승인 UI를 눌렀다는 증거가 아니다.
- build.rs가 요구하는 pinned Node 산출물은 main의 기존 ignored 산출물을 독립 Worktree에 복사하고 pin hash와 LICENSE를 검증했다. 다운로드·의존성 추가 없이 offline Cargo를 사용했다. P0 bundle/target을 빌드 경로로 사용하지 않았다.
- 새 코드에 네트워크/파일/프로세스/credential 실행 API가 없고 실제 Provider adapter가 없다. source_kind는 synthetic만 생성한다. 또한 빌드된 **library test executable**을 `sandbox-exec -p '(version 1)(allow default)(deny network*)' <library-test-executable>`로 실행해 **46/46 PASS**를 확인했다. HTTP 호출 counter를 임의로 0으로 설정한 테스트가 아니다. 이 증거는 P2.1a의 외부 Provider 호출 0회이며 production 전체 앱 sandbox를 증명하지 않는다.
- rustfmt는 설치된 stable toolchain에 없어 실행 불가였고 새 component를 설치하지 않았다. 저장소에서 formatter를 필수화하지 않으며 whitespace/governance와 컴파일·테스트를 검증했다.
- GUI·packaging·UI/Node TypeScript 변경 검증·실제 Provider/Keychain·실제 A/B lifecycle는 NOT_RUN. 기존 Rust Sidecar regression이 실행한 합성 child는 P0 실측이나 새 Runtime 선정이 아니다.
- 정확한 최종 source SHA와 GitHub governance CI는 Draft PR에 기록한다. governance CI는 Rust 테스트를 실행하지 않으므로 로컬 결과와 구분한다.

## P0 보호 및 P2.1b 선행 조건

Build/Test 전 P0 채팅이 idle이고 측정 종료 보고 이후 재개가 없음을 확인했다. 작업 중 재개 통지 시 무거운 Build/Test/GUI를 중단한다. 기존 #44/#45, fixed app, 원본 표본, recovery Worktree는 수정하지 않았다. E5c PARTIAL은 그대로다.

다음 단계에는 Runtime/reload 지속성 결정, 신뢰 lifecycle→Agent bootstrap bridge, 실제 transport sender binding·serialization·byte limit, monotonic timer driver, 소유 자원 cleanup 증거, 선택 Runtime의 lifecycle parity 검증이 필요하다. 이 중 Runtime A/B 선정과 P2.1b/c 착수는 별도 승인이다. 실제 Provider는 credential·전송 동의·모델·비용/한도 승인과 새 adapter 검증 이후에만 가능하다.

```text
P2_0_MAIN_MERGED: YES
P2_1A_HOST_FIXTURES_PASS: 39/39 scoped cases
P2_1A_LIBRARY_TESTS_PASS: 46
EXISTING_RUST_REGRESSION_PASS: 19
FULL_54_FIXTURES_PASS: NO
EXTERNAL_PROVIDER_CALLS: 0
RUNTIME_SELECTED: NO
P2_1B_STARTED: NO
P2_1C_STARTED: NO
P2_1A_MAIN_MERGED: NO
```
