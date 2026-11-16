# P2.1b Foreground WKWebView Integration

[Issue #50](https://github.com/Kim-Ji-Won/AETHER/issues/50) · 기준 main `2484e773f9d077a0cbc37a33768f892c8b6b866a`

## 승인된 범위와 남은 결정

사용자는 P2.1b의 첫 개발용 text 경로에 Runtime A(WKWebView)를 제한적으로 승인했다. Foreground 단일 synthetic run이며 Reload/Quit 중단을 허용한다. 자동 재전송/재시도, 영구 대화 복원, background 실행 보장은 없다. Rust Host가 admission/state/terminal/deadline의 소유자다.

이 승인은 Runtime A의 성능 우위나 최종 제품 Runtime 선정, P0 완료가 아니다. [P0 Issue #44](https://github.com/Kim-Ji-Won/AETHER/issues/44) / [Draft PR #45](https://github.com/Kim-Ji-Won/AETHER/pull/45)의 ADR 001 PROPOSED와 E5b/E5c PARTIAL은 그대로다. 기존 미병합 ADR을 변경하거나 새 최종 선정 ADR을 만들지 않는다. 이 문서는 제한적 개발 결정과 구현 증거를 기록한다.

[P2.0 계약](p2-0-agent-runtime-contract.md)과 [P2.1a Host](p2-1a-agent-run-contract.md)를 재사용한다. 실제 Provider/credential/음성/도구/메모리는 제외한다. P2.1c 제품 Conversation UI 및 Ready/merge는 별도 승인 대상이다.

## 실행 구조

`AgentDiagnostics → AgentRuntime(TypeScript, WKWebView) → Tauri local capability → lifecycle Gate → agent_runtime Runtime → agent_text Host / SyntheticAdapter`

- 기존 FakeRun은 별도 namespace, 명령, UI와 구현을 유지한다. 25/50/75 progress를 text로 변환하지 않는다.
- Diagnostics는 고정 문자열의 synthetic producer만 사용한다. 사용자 입력 질문, URL, model, credential reference를 받는 Provider 경로는 없다. 입력 128 / delta 64 / output 2048 UTF-8 bytes, 128 deltas, 기존 lifecycle envelope 768 bytes의 상한을 둔다.
- Start는 queued만 만든다. 별도 dispatch barrier에서 Rust `SyntheticAdapter`를 생성한다. queued cancel은 adapter 미생성/cleanup NotRequired로 즉시 끝난다.
- 실제 text frame은 WKWebView의 순차 async producer에서 생성한다. Rust adapter 소유 수명과 Host 검증을 거쳐 한 frame씩 반영하며 source_seq를 재발급하거나 덮어쓰지 않는다. 이 bridge는 무제한 ingress queue를 만들지 않는다. P2.1a adapter의 memory queue와 virtual barrier 테스트는 유지한다.
- UI는 이벤트를 이어 붙이지 않고 전체 Host snapshot을 교체한다. 같은 instance에서 wire epoch/revision이 역행하는 응답을 버린다. 불확실한 admission은 재전송하지 않는다. 출력은 React text node로만 표시한다.

## Lifecycle / Sender Boundary

기존 Gate의 instance와 legacy epoch를 읽는 native 경로만 `Host::attach`를 호출한다. UI 값으로 trusted lifecycle을 생성하지 않는다. legacy epoch 0은 그대로 두고 agent wire epoch는 legacy+1이다. 앱 최초 page Started가 이미 발생했다면 실제 첫 UI는 1→2일 수 있다. bootstrap은 기존 Host를 초기화하거나 request high-water를 지우지 않는다.

새 명령은 `agent_bootstrap`, `agent_start`, `agent_cancel`, `agent_snapshot`, `agent_runtime_input` 다섯 개다. 기존 local/main capability에 별도 최소 권한을 추가하고 CSP는 그대로 유지한다. Rust는 Tauri가 주입한 `WebviewWindow`의 label=main과 실제 URL origin을 검사한다. release 앱은 `tauri://localhost`, debug에서만 기존 고정 dev origin을 추가 허용한다. JSON의 sender 주장만으로 통과하지 않는다.

Host CSPRNG로 run마다 128-bit lease를 발급한다. 실제 호출 창/origin → lifecycle instance/epoch → lease → 전체 run/request/attempt/generation/correlation/source_kind → source_seq/closed schema 순서로 검증한다. lease는 로그·저장소에 기록하지 않으며 다른 run/context에서 재사용할 수 없다. 이는 같은 origin의 신뢰된 bundled producer를 묶는 경계이며, 같은 JS context 전체가 장악된 경우를 격리하는 별도 보안 principal은 아니다.

모든 lock 순서는 Gate → Runtime이다. Timer는 Runtime만 잡고 Gate로 역진입하지 않는다. 외부 wait/IPC는 lock 안에서 수행하지 않는다. 기존 FakeRun/Native Approval의 Gate 검사는 closing 거부를 유지한다. Agent shutdown 중에는 정확한 기존 envelope의 snapshot/이미 bound된 release만 별도 허용하며 start/dispatch/frame 권한을 열지 않는다.

Reload Started는 lease를 폐기하고 generation을 증가시켜 active run을 RuntimeLost로 확정한다. 소유 Rust adapter를 drop하고 native Finished가 이전 document 교체를 관찰한 뒤 cleanup을 확인한다. Finished가 없으면 cleanup deadline 뒤 Unconfirmed이며 다음 admission을 막는다. 새 UI는 bootstrap/snapshot으로 실패 상태를 재동기화하며 작업을 다시 시작하지 않는다. window Destroyed는 실제 context 파괴 확인 경로다. OS renderer crash 강제 주입은 별도 검증 범위다.

## Timer / Cancel / Cleanup

Rust의 process-local `Instant`를 시작점으로 10ms driver가 Host 가상 clock에 elapsed ms를 전달한다. 모든 command 앞뒤에도 같은 clock을 진행한다. run=12s, idle=3s, cleanup=2s는 짧은 진단용 설정이며 제품 성능 예산이 아니다. renderer timer/rendering/visibility가 Rust deadline을 소유하지 않는다. deadline 도달은 뒤따르는 frame/ACK보다 먼저 처리한다.

Cancel/Timeout은 Rust adapter queue를 실제로 비우고 abort를 확인한다. WK producer는 AbortController로 timer를 제거하고 마지막 in-flight IPC가 끝난 뒤 release를 보낸다. Host는 adapter와 runtime 양쪽 정리 전 성공/취소 완료를 만들지 않는다. ACK 누락은 Failed/CleanupUnconfirmed 또는 기존 timeout terminal + cleanup Unconfirmed다. 늦은 정리 확인은 cleanup만 바꾸며 terminal/error/output을 되돌리지 않는다.

Quit는 intake를 먼저 닫고 AppShutdown 취소를 요청한다. 기존 FakeRun child와 Agent cleanup이 모두 확인되어야 정상 종료한다. 5초 안에 확인되지 않으면 host를 유지하고 실패를 기록한다. intake는 닫힌 채로 유지하며, 늦은 cleanup 이후 사용자의 명시적 Quit 재요청을 허용한다. AppKit의 직접 native termination 경로는 main-thread ACK를 기다릴 수 없으므로 별도 `cleanup_confirmed` 관측값을 남기고 성공을 가장하지 않는다. 로그는 고정 상태만 기록하며 text/lease/raw transport 오류를 넣지 않는다.

## 검증 기록

검증 소스는 이 문서와 함께 제출되는 Draft PR HEAD다. 2026-11-16(KST), macOS 26.6 / arm64의 별도 Worktree에서 수행했다. 서명/공증/배포 또는 P0 측정 결과가 아니다.

| 검사 | 상태 / 범위 |
|---|---|
| Rust `cargo test --locked --offline` | PASS: Host 46, bridge 13, 기존 regression 19 (총 78) |
| `npm run typecheck` | PASS |
| Node `--experimental-strip-types --test` (agent-runtime/fake-provider/sidecar-protocol) | PASS: 13 |
| Governance / whitespace | PASS: 21 governance artifacts / `git diff --check` |
| Release packaged macOS app | PASS: 아래 관찰 범위, unsigned `--no-sign` app bundle |
| network-denied test binaries | PASS: Host 46 + bridge 13, `(deny network*)` 아래 재실행 |

실행 환경 경로 누락으로 최초 패키징 명령이 실패한 후 Cargo/npm 경로를 명시했다. 최초 복사 캐시에 Three.js가 없고 offline npm cache도 ENOTCACHED였으므로, 동일 lockfile의 기존 visual worktree 캐시를 별도 worktree에 복사했다. dependency/lockfile 변경과 registry 다운로드는 없다. TypeScript IPC argument 타입 오류는 수정 후 검사 통과했다.

실행 명령(저장소 루트 기준):

```sh
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --offline
cd apps/desktop
npm run typecheck
node --experimental-strip-types --test tests/agent-runtime.test.mjs tests/fake-provider.test.mjs tests/sidecar-protocol.test.mjs
CARGO_NET_OFFLINE=true npm run tauri -- build --bundles app --no-sign -- --locked --offline
```

아래 전체 matrix의 첫 packaged executable SHA-256은 `a0c62ce13aca1ee503083b215b234c51abc7d33924999248d3100377d8d87986`이다. 최종 검토에서 Quit 실패 후 재요청 flag를 해제하는 보완을 추가했고, 최종 executable SHA-256은 `a8af5e3b9d806b039f50dfa0a041437ed5ee87dbf322fd5f916a0dff09dc8838`이다. 최종 bundle에서는 정상 run, IPC 9개 oracle, Quit 미확인→늦은 ACK→새 Start 거부→명시적 Quit 재요청을 추가 확인했다. 나머지 첫 matrix를 최종 bundle에서 전부 반복했다고 주장하지 않는다. Rust 전체 78개는 최종 소스로 재실행했다.

UI는 bundle을 LaunchServices로 실행하고 native AX 상태와 실제 메뉴/버튼을 사용해 관찰했다. 최초 network-denied GUI 프로세스는 실행됐지만 앱 제어 도구가 GUI를 찾지 못해 종료했다. 그 시도는 앱 PASS로 세지 않으며 GUI 검증은 정상 LaunchServices 실행에서 수행했다. network-denied 증거는 위 test binaries에 한정한다.

| Packaged 관찰 | 실제 결과 |
|---|---|
| 정상 Synthetic Text | run 1: `Synthetic text 완료.`, Succeeded / seq6 / terminal1 / Confirmed |
| queued 취소 | run 5: Queued seq1 → Cancelled seq3 / terminal1 / NotRequired; 가짜 ACK 없음 |
| running 취소 | run 6: partial text 유지, Cancelled / terminal1 / Confirmed |
| IPC 계약 검사 | duplicate Start, wrong lease, wrong attempt, duplicate Event, cancel 후 late Delta/Completion, duplicate Cancel, source-seq gap, UnsupportedContent 총 9개 oracle PASS |
| idle deadline | run 7: Failed / IdleTimeout / terminal1 / Confirmed |
| cleanup ACK 누락/늦은 ACK | run 8: Failed / CleanupUnconfirmed / Unconfirmed, 새 Start disabled. 늦은 ACK 뒤 Confirmed로만 변경, Failed/seq6/terminal1/error 유지 |
| run deadline / Diagnostics 숨김 | run 9: Main으로 이동한 동안에도 11개 delta 후 Failed / RunTimeout / terminal1 / Confirmed |
| 진행 중 Reload | run 10: RuntimeLost / terminal1 / Confirmed, legacy epoch 1→2, wire 2→3, generation 2→3; snapshot 재연결, 자동 재실행 없음 |
| 진행 중 Quit | run 11 Running을 관찰 후 Cmd+Q. `stop-intake` → `AETHER_AGENT_QUIT cleanup=confirmed` → `child_idle=true pending=false`, 프로세스 없음 확인 |
| 재시작/이전 Session | 새 instance, Agent Run 없음과 counter0. 직전 instance의 snapshot 요청은 `Stale or closing lifecycle session`으로 거부됨 |
| 최소화/비활성 | 두 번째 실행의 run 1: Cmd+M 이후 문서 숨김 관찰, Failed / IdleTimeout / terminal1 / Confirmed 확인. background 지속 실행/성능 보장으로 해석하지 않음 |
| 마지막 창 Close | 두 번째 실행의 run 2 Running 관찰 후 native close. 앱 목록에서 제거 확인; 가짜 복구/재시작 없음 |
| 기존 FakeRun | WebView A와 bundled Node B 각각 FAKE_OK / terminal1; B exit0/reaped 확인. Agent text와 별도 표시 |
| Native Approval | native sheet 취소 후 counter0, 다시 1회 승인 후 counter1 |
| 최종 bundle Quit 실패/복구 | CleanupUnconfirmed / Unconfirmed인 run에서 Quit 후 5초 이상 host 유지. 늦은 ACK 뒤 Failed/terminal1/error/closing=true 유지, cleanup만 Confirmed. 새 Start 거부·revision 불변, 두 번째 Cmd+Q 후 앱 종료 |

화면 증거는 비민감 고정 synthetic 값만 포함하는 native AX 관측을 위 표로 전사했다. Main Particle/Dock/Navigation은 기존 상태를 유지했다. 무관한 화면이나 원본 시스템 로그를 커밋하지 않는다.

## Fixture 구분

- C01–C12 / I01–I12 / E01–E15: 기존 39개 결정적 Host fixture 재실행 PASS. 이것만으로 실제 WK lifecycle 검증을 주장하지 않는다.
- bridge 추가 13개: bootstrap 0/1, native sender predicate/lease, queued cancellation, normal two-party cleanup, cancel/completion race, idle/run/cleanup deadline, reload replacement/missing finish, quit, restart, bound malformed frame.
- frontend 5개: snapshot revision/terminal 역행 방지, admission 불확실 시 재전송 없음, queued cancel no ACK, Completed→release 순서, in-flight frame drain→release barrier.
- L02/L03 (Runtime B), U02 (event subscription race), 실제 Provider credential/HTTP/청구, OS crash, 서명/공증, 다른 Mac: NOT_RUN.
- 실제 앱 대응: C01/C02/C03/C05의 취소 경로, I01 및 I04/I05/I06의 lease·attempt·중복·gap 하위 경우, E02/E04/E15의 deadline/terminal 하위 경우, L01/L04/L05/L06의 위 표에 명시한 foreground 경로를 검증했다. fixture의 모든 입력 변형을 앱에서 반복한 것은 아니다.
- U01/U03/U05는 frontend 테스트의 projection/admission 하위 범위 PASS, U04는 실제 synthetic 표시를 관찰했다. S01/S03/S04는 기존 Host 테스트/코드 검토 범위이며 실제 credential/telemetry 검증은 NOT_RUN. S02의 React text-only 경계는 코드 검토했으나 독립적인 악성 markup 실환경 전수 검사는 NOT_RUN.
- 다른 sender label/origin은 Rust predicate 테스트와 Tauri capability 검토 PASS이며 별도 공격용 WebView의 실환경 주입은 NOT_RUN. Reload 후 stale command는 Gate/Host 테스트로 검증했고 실제 앱에서는 native context 폐기와 재연결을 관찰했다. 54/54 PASS라고 보고하지 않는다.

## 변경 파일과 보존

| 파일 | 역할 |
|---|---|
| `apps/desktop/src-tauri/src/agent_runtime.rs` | Gate 아래 bridge, sender/lease, monotonic driver, adapter/cleanup |
| `apps/desktop/src-tauri/src/agent_runtime/tests.rs` | bridge contract 13 tests |
| `apps/desktop/src-tauri/src/agent_text/mod.rs` | trusted-owner snapshot inspection 재사용 |
| `apps/desktop/src-tauri/src/lib.rs` | binary integration과 library 역할 설명 |
| `apps/desktop/src-tauri/src/lifecycle.rs` | trusted attach, reload/context replacement, shutdown cleanup |
| `apps/desktop/src-tauri/src/main.rs` | command / native lifecycle hook 연결 |
| `apps/desktop/src-tauri/build.rs` | 최소 command manifest |
| `apps/desktop/src-tauri/capabilities/main-agent-text.json` | local main synthetic IPC 권한 |
| `apps/desktop/src-tauri/tauri.conf.json` | capability 연결, 기존 CSP 보존 |
| `apps/desktop/src/agent-runtime.ts` | bounded synthetic producer, abort/drain, snapshot projection |
| `apps/desktop/src/AgentDiagnostics.tsx` | 제한된 개발 검증 UI |
| `apps/desktop/src/Shell.tsx` | Diagnostics panel 한 곳 추가 |
| `apps/desktop/tests/agent-runtime.test.mjs` | projection / admission / drain 검증 |
| `docs/architecture/p2-1b-foreground-runtime.md` | 범위·결정·증거·한계 |
| `docs/architecture/README.md` | 후속 승인 이력과 증거 링크 |

main과 P0 및 #38의 두 visual Worktree는 별도 보존했다. 해당 HEAD/status/diff를 비교하고 P0 채팅이 idle인 상태에서만 build/test/GUI를 진행했다. P0 evidence/고정 App Bundle은 수정하지 않았다. 새 bundle과 의존성/빌드 산출물은 이번 Worktree의 ignored 경로에만 둔다. 실측이 재개되면 같은 Mac의 무거운 작업을 중단하는 규칙을 유지한다.

## 보안 및 후속 조건

실제 External Provider Call=0: 새 경로에는 network client/SDK/URL/secret resolver/process 실행이 없고, 고정 synthetic frame만 Tauri local IPC로 전달한다. 기존 CSP는 외부 connect를 허용하지 않으며 새 capability는 shell/network/keychain 권한을 포함하지 않는다. 전체 앱의 기존 FakeRun Node fixture는 별도 회귀 경로다. Provider 사용 가능성이나 credential 안전성을 실제 API로 검증한 것은 아니다.

P2.1c 최소 선행 조건은 이 Draft의 lifecycle/cleanup 및 실제 앱 증거 리뷰, 제한적 foreground/reload 중단 정책의 제품 표시, snapshot/error/cancel 상태를 사용할 최소 UI 요구 승인이다. 실제 Provider 도입은 별도 endpoint/credential/native boundary·권한·비용·취소 정책 승인과 검증이 필요하다. P0 최종 ADR, background 지속성, Runtime A/B 비교는 이 PR로 완료되지 않는다.
