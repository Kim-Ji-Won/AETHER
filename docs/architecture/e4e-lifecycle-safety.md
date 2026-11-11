# E4e 창·프로세스 수명주기와 복구

[Issue #23](https://github.com/Kim-Ji-Won/AETHER/issues/23) · 기준 `14c6d6029fb51d40b528627fb5d34cbe443d9204`.
RFC는 PROPOSED이며 Runtime 선정·E5 성능 측정은 NOT_RUN입니다. 아래 정책과 실제 검증 결과를 구분하며 실행 증거는 Issue 후속 기록에 둡니다.

## 구현 전 관찰과 정책

기존 E4d app의 새로운 격리 복사본에서 마지막 창 close 뒤 해당 Host PID가 종료되는 것을 확인했습니다. 이 기본 close 정책을 유지하며 hide로 대체하거나 백그라운드 상주를 추가하지 않습니다. B가 별도 프로세스여도 마지막 창 close 뒤 Host와 함께 종료하는 정책입니다.

| 사건 | Host / WebView | A 원장 | B 원장·child | snapshot / 이전 메시지·승인 |
|---|---|---|---|---|
| hide / minimize | 생존 / context 유지 | 진행, timer 지연 가능 | 고정 read-only fixture 진행 | 동일 세대, 승인 자동 실행 없음 |
| reload / context 재생성 | 생존 / 교체 | active → failed 한 건 | 기존 run·generation 유지, 계속 진행 | 새 context epoch로 host snapshot attach; 이전 epoch 쓰기 거부, 승인 무효화 |
| 마지막 창 close | 종료 / 소멸 | context 소멸 → failed | cancel_requested → reap 확인 → cancelled | 새 실행은 새 instance; sheet/pending 정리 |
| renderer crash | 실제 주입은 안전한 개별 PID 식별 시만 | 종료 신호를 관찰하지 못하면 성공/취소 추정 금지 | Host가 생존하면 fixture 진행 가능 | 실제 crash와 reload 모의 결과를 혼동하지 않음 |
| 정상 Quit | 종료 / 소멸 | active → cancelled | kill/wait 확인 후 cancelled | 신규 intake 차단, sheet/pending 무효화 |
| Host 강제 종료 | 소멸 / 소멸 | 메모리 기록 소실 | stdin EOF 시 Node 즉시 exit 1 | Drop 보장 아님; 실제 OS orphan 관찰 필요 |
| child 비정상 종료 | 생존 / 생존 | 해당 없음 | EOF 또는 barrier try_wait 감지 → reap → failed | snapshot 일치, 사용자의 새 요청만 시작 |
| 승인 부모 close | 기본 close 정책 | 카운터 증가 없음 | 해당 없음 | Rust pending 소비, native sheet/observer/monitor 정리, late callback 거부 |

## IPC와 상태 소유권

기존 7-field Provider frame과 최신 run 하나의 Rust ledger는 유지합니다. WebView의 create/start/cancel/submit만 `{instance, epoch, payload}` envelope를 요구합니다. instance는 macOS `arc4random_buf`의 128-bit 난수이고 프로세스마다 새로 생성합니다. epoch는 main page-start/close 때 증가합니다. outer serialized JSON은 768 bytes, inner frame 511 bytes, start/cancel 80 bytes의 기존 한계를 유지합니다. 이 검사는 Tauri JSON parsing 이후이며 transport allocation 전체의 상한은 아닙니다.

lifecycle gate를 잡은 상태에서 envelope 검증과 ledger 전이를 함께 수행합니다. 잠금 순서는 gate → ledger이며 pipe, native sheet, child wait 중에는 잡지 않습니다. 잘못된 instance/epoch, closing 상태는 payload를 실행하기 전에 거부합니다. 조회 전용 get_runtime_session은 local main capability에만 추가합니다. 기존 CSP·origin·window 범위를 확대하지 않습니다. instance는 freshness 식별자이며 사용자 인증이나 비밀 토큰이 아닙니다.

새 context는 session과 host snapshot을 읽고, 100ms 단일 in-flight 조회로 B 진행 상태를 다시 표시합니다. 이전 응답이 더 최신 run/seq를 덮지 않습니다. B의 run generation은 생성 시 고정되고 transport는 Host가 만든 전용 pipe이므로 reload 뒤에도 계속 수락됩니다. 새 context의 취소는 현재 epoch envelope와 정확한 run/variant를 요구합니다. A는 context 소멸 시 failed terminal 한 건으로 닫습니다. frame/promise 소실을 succeeded로 표시하지 않습니다.

마지막 창 close는 Quit으로 이어지므로 close/reopen에서 이전 run snapshot의 영속 복구를 주장하지 않습니다. UI에 `앱 재실행은 새 세션`과 instance/context를 표시합니다. 메모리 카운터와 ledger는 재시작 시 초기화됩니다.

## 종료와 승인 정리

ExitRequested에서 intake를 닫고 active synthetic run을 취소한 뒤 B worker가 자기 child handle을 kill/wait하도록 합니다. 최대 5초 동안 cleanup을 확인하고 성공할 때만 최종 exit를 허용합니다. Tauri ExitRequested 경로의 확인 실패 시 Host를 유지하고 오류를 기록합니다. macOS 기본 Quit 메뉴의 AppKit terminate:는 ExitRequested를 우회하므로 willTerminate observer에서도 intake 차단·sheet 정리와 잠금 없는 최대 5초 child wait를 실행합니다. 이 native 경로는 종료를 취소할 수 없으며 cleanup false를 실패로 기록합니다. Node는 stdin EOF를 ACK 대기 중뿐 아니라 timer 중에도 즉시 처리합니다. Host SIGKILL 뒤 cleanup 보장은 실제 시험 결과에 한정합니다.

승인 parent close/reload/Quit/60초 lifecycle deadline은 Rust pending을 먼저 무효화한 뒤 main thread에서 Swift sheet를 끝냅니다. Swift finish는 한 번만 callback하고 observer·event monitor·등록 closure를 제거합니다. callback id가 현재 pending과 같을 때만 명시적 response 1이 카운터를 증가시킵니다. 대기 중 상태 잠금을 잡지 않습니다. 60초는 실험 자원 정리 상한이며 작업별 TTL·보안 인증·실제 사람의 동의 검증은 [P4.2](../engineering/development-roadmap.md)에 남깁니다.

macOS 기본 Quit 항목은 sheet 중 실행되지 않는 관찰 결과에 따라 고정 native Quit 항목으로 교체하고 Tauri ExitRequested 정리 경로로 연결합니다. AppKit willTerminate 보완도 유지합니다. Lifecycle native 메뉴는 고정 main WebView reload와 창 close를 제공합니다. 임의 script/PID/path 실행 command는 노출하지 않습니다. GUI 자동화의 승인 클릭은 실제 사람의 동의를 증명하지 않습니다.

## 고정 fixture와 검증

normal은 100ms, long60은 start 이후 15초 간격의 25/50/75/FAKE_OK 네 단계입니다. 실제 wall time에는 ACK와 macOS scheduling 지연이 추가됩니다. pause25의 300초 상한은 유지합니다. B long60의 frame timeout은 20초이며 나머지는 기존 5초입니다. 외부 AI·네트워크·파일 작업은 없고 Node pin/LICENSE 및 의존성은 변경하지 않습니다.

```sh
# repository root
export PATH="$HOME/.cargo/bin:$PATH"
python3 scripts/prepare_node_sidecar.py --verify-only
CARGO_TARGET_DIR="$PWD/target/e4e-issue-23" cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --offline -- --nocapture
cd apps/desktop
npm run build
node --experimental-strip-types --test tests/fake-provider.test.mjs tests/sidecar-protocol.test.mjs
CARGO_TARGET_DIR="$PWD/../../target/e4e-issue-23" CARGO_NET_OFFLINE=true npm run tauri -- build --bundles app --no-sign -- --locked --offline
open -n "$PWD/../../target/e4e-issue-23/release/bundle/macos/AETHER.app"
```

파괴적 실험은 위 원본 대신 별도 복사본에서만 수행합니다. 이번에 생성한 PID·OS 시작시각·bundle path·instance·parent 관계를 대조합니다. 공유 WebKit PID에는 신호를 보내지 않습니다. 실제 renderer crash를 안전하게 특정할 수 없으면 NOT_RUN이며 reload 테스트로 대체 PASS하지 않습니다. 기존 app/DMG는 보존합니다.

기존 Rust·Provider·Node 회귀, epoch/instance/closing 거부, A context loss/B snapshot 유지, approval late callback, 실제 Node pipe EOF 테스트와 packaged GUI/OS 실험을 분리합니다. 실패·orphan이 관찰되면 원래 결과를 보존하고 정리 후 PASS로 바꾸지 않습니다. source SHA, app/Node hash, 횟수·exit·UI 및 로그 대조는 Issue에 기록합니다. E5에는 반복 표본, CPU/GPU/RSS/배터리, p50/p95, cold launch와 창 독립성 요구를 남깁니다.

API 근거: [Tauri RunEvent](https://docs.rs/tauri/latest/x86_64-apple-darwin/tauri/enum.RunEvent.html), [PageLoadEvent](https://docs.rs/tauri/latest/tauri/webview/enum.PageLoadEvent.html), [Node child pipes](https://nodejs.org/api/child_process.html). 이 문서는 서명·공증·다른 Mac 검증이나 production 안전성 보장이 아닙니다.

## 로컬 관찰 요약 · 2026-11-12

| 구분 | 실제 관찰 | 판정 |
|---|---|---|
| A 60초 foreground / hidden / minimized | 각각 1회, 60.015 / 60.405 / 60.239초, seq 1~5, FAKE_OK·terminal 1 | PASS |
| B 60초 foreground / hidden / minimized | 각각 1회, 60.036 / 60.028 / 60.013초, seq 1~5, exit 0 / reaped | PASS |
| A reload | run 7, generation 1, 25% → failed 한 건, context 2로 snapshot 재조회 | PASS |
| B reload + child abnormal exit | run 8/generation 2 유지, context 3 attach; 자기 child SIGKILL 후 failed 한 건; 새 run 9 성공 | PASS |
| 마지막 창 close | A 및 승인 대기 close: 카운터 1 유지·sheet/pending 제거. B close: reap·Host exit 0 | PASS |
| Host SIGKILL / restart | 직접 만든 Host exit -9; 250ms 간격 21회 해당 Host/child 잔존 0; 새 instance·counter 0·B 정상 완료 | PASS |
| 최종 Quit 회귀 | A+승인 대기, B active, idle 각각 Host exit 0; counter 0 유지·observer/monitor 제거·child 회수 | PASS |
| Rust / Provider·Node | 18 / 8 tests, 실패 0; 기존 E3b~E4d 회귀 포함 | PASS |
| renderer 실제 crash | 안전하게 전용 renderer PID를 식별하는 주입 경로를 확보하지 않음; 공유 WebKit 종료하지 않음 | NOT_RUN |

숨김과 최소화가 겹친 A 추가 표본 1회도 성공했지만 단독 minimized 표본으로 계산하지 않았습니다. 최소화 창 복원은 GUI 도구 한계로 사용자 도움을 받았으며 복원 후 화면을 다시 관찰했습니다. 승인 버튼은 GUI 자동화가 눌렀습니다. 실제 사람의 승인 안전성, 다른 Mac, 서명·공증, E5 성능 반복 표본은 미검증입니다.

개발 중 Quit 결함 두 건을 숨기지 않습니다. 처음에는 AppKit 기본 Quit이 ExitRequested 정리를 우회했고, 이를 보완한 뒤에도 sheet 중 기본 Cmd+Q가 종료되지 않는 실패 1회를 관찰했습니다. 앞의 실패는 willTerminate 정리로, 뒤의 실패는 native Quit 메뉴 교체로 수정했습니다. 최종 A+승인/B/idle Quit 세 표본을 다시 통과했습니다.

60초·reload·SIGKILL·parent-close matrix는 실행 파일 SHA-256 `b60c697700c97663a182870ecee1c2ec103e20771ce76ee7a8e19e94c7e8a0df`에서 수행했습니다. 최종 실행 파일은 `cbe44bc3848d63095acd6bff56c8cec623b2e1399f01b08f5e00593191cc0f7c`입니다. 두 빌드 사이 앱 소스 차이는 `lifecycle.rs`의 native Quit 메뉴 연결뿐입니다. 마지막 변경 후 Rust 18 tests, TypeScript/Vite·패키징, A/B 정상 경로, 코어·macOS 26·AX false·카운터 승인, A+sheet/B/idle Quit을 재검증했습니다. 60초 matrix와 SIGKILL을 최종 hash에서 다시 실행한 것으로 표현하지 않습니다.

빌드 시 Git HEAD는 기준 main이며 수정된 작업 트리에서 빌드했습니다. 최종 커밋과 실제 빌드 입력 파일의 SHA-256 일치를 별도로 확인하고 Issue/PR에 전체 source SHA를 기록합니다. 최종 Node 및 LICENSE hash는 pin과 동일합니다. 먼저 분리 실행한 일부 Host는 launcher가 종료되어 numeric exit code를 수집하지 못했으며, PID 소멸과 cleanup trace만 관찰했습니다. 이후 supervised Host의 -9/0, Node의 wait/reap 결과와 혼동하지 않습니다.
