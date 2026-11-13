# E5b 반복 지연 측정 · Repeated Latency Benchmark

[Issue #27](https://github.com/Kim-Ji-Won/AETHER/issues/27), 기준 main `5dd1ae9f8323d96ddac842374706c7fe9edda220`.
[E5a 계약](e5a-measurement-foundation.md)을 재사용합니다. Runtime 선정·E5c/E5d·P1 구현·병합은 범위 밖입니다. RFC는 PROPOSED입니다.

## 소스·화면 격리

관리형 `aether-e5b` 독립 Git Worktree가 AETHER-E5B 역할을 합니다. 기존 main 작업 폴더·P1 Worktree를 수정하지 않습니다. `target/e5b-issue-27`만 빌드·결과 경로로 사용합니다. E5a의 SMOKE_ONLY 데이터는 읽어 통계에 넣지 않습니다. 기존 Rust 계측, React/CSS, Swift bridge, Node provider/pin/LICENSE, capability/CSP/lockfile은 바꾸지 않습니다. 새 의존성 없이 Python 표준 라이브러리만 사용합니다.

모든 시도는 같은 commit SHA와 같은 sealed packaged app을 사용합니다. 계획 이후 source/artifact가 바뀌면 실행을 거부합니다. 측정 중 fetch/rebase/새 main 반영을 하지 않습니다. 최종 결과 문서만 추가 커밋한다면 측정 당시 SHA와 최종 HEAD를 구별하고 앱 입력 hash의 동등성을 확인합니다.

## 고정 일정과 제외 규칙

| phase | normal A/B | pause25 A/B | 통계 포함 |
|---|---|---|---|
| warmup | 후보별3회 | 후보별3회 | 본 측정에서 제외 |
| measurement | 후보별30회 | 후보별30회 | 유효 표본만 latency, 모든 시도는 분모에 포함 |
| overhead | 후보별 ON3/OFF3회 | 수행하지 않음 | 별도 공통 외부 지표 대조 |

총144회 계획이며 각 시도는 새 process입니다. phase 안의 round마다 A→B / B→A를 교대합니다. normal과 pause25는 서로 분리합니다. overhead ON/OFF 순서도 round마다 교대합니다. 작업은 고정 100ms Fake Provider와 기존 UI 버튼으로 시작합니다. pause25는 25% 상태를 실제 확인한 뒤 취소합니다. 취소까지 걸린 GUI 자동화 대기는 Host cancel 처리시간에 포함하지 않습니다. accept→terminal 취소 값에는 operator 대기가 들어가므로 정상 완료시간과 비교하지 않습니다.

실패·TIMEOUT·환경 제외를 기록하며 임의 보충하지 않습니다. 유효30개 미달은 PARTIAL입니다. warmup 실패도 따로 보고하며 본 측정으로 이동시키지 않습니다. 중단된 block은 이미 생성된 trial directory를 덮어쓰지 않습니다. 최대4개씩 처리하는 block을 이어갈 수 있지만 계획에 없는 trial은 생성하지 않습니다.

## 환경과 P1 간섭 방지

실측 전에 사용자가 P1 build/3D/GUI 보류와 전원·밝기 유지 여부를 확인해야 합니다. `--quiet-window-confirmed`는 그 확인을 기록하는 실행 gate이며 자체 사용자 승인을 만들지 않습니다. 자동 process 점검은 보조 수단이고 일반 3D 작업 전체를 탐지할 수는 없습니다.

각 시도 전/후 macOS build, architecture, AC/BATTERY, battery 상태·%, thermal 경고 수집 가능 여부, load average, logical CPU 수를 저장합니다. 화면은 foreground, 기본 900×760입니다. 화면 밝기는 수치 자동 수집을 하지 않고 사용자 유지 확인을 기록합니다. 밝기·전원·시스템 설정을 도구가 바꾸지 않습니다. 같은 환경 key(build/architecture/power/battery state/thermal)만 같은 cohort로 집계합니다. 시작/끝 key 변화나 P1/다른 AETHER 실행·build 감지, 1분 load가 logical CPU 수를 초과한 표본은 EXCLUDED입니다. load 자체도 보존하며 낮은 변화까지 완전히 통제했다고 주장하지 않습니다.

다른 프로세스의 전체 이름·인자·경로는 공개하거나 결과에 저장하지 않습니다. 읽기 전용 process 목록에서 build/다른 AETHER의 존재만 검사하고, 사용자 앱은 종료하지 않습니다. 이미 실행 중이면 새 trial을 시작하지 않습니다. 측정 중 감지하면 표본을 제외하고 block을 멈춥니다.

## 여섯 지표와 clock domain

| measurement | 정의 / ms | 한계 |
|---|---|---|
| accept_to_first_progress | Host accept → 25% 수락, Host Instant | 100ms timer+ACK, B 준비 비용 포함 |
| accept_to_terminal | Host accept → A terminal / B reap 뒤 run_final | normal/cancel 분리; B final 경계는 reap 포함 |
| host_cancel_processing | Host cancel 진입 → 취소 상태 확정 | UI→Host IPC 및 GUI 대기 제외 |
| node_spawn_to_ready | Host spawn 관찰 → 검증된 ready 수신 | B만, loader·handshake·pipe 포함 |
| cancel_confirmed_to_reap | Host cancel 확인 → child wait 완료 | B만; SIGKILL return code는 null일 수 있으나 reap 확인 필요 |
| process_spawn_to_session_ready_observed | Harness Popen 직전 → session_ready JSON 수신 | Python monotonic 한 clock. loader·pipe/reader 지연 포함, kernel process 생성의 정확한 시점이나 cold launch/first paint 아님 |

Host 내부 이벤트는 E5a schema_version=1과 ID/seq/terminal 검사를 그대로 사용합니다. 다른 프로세스의 clock timestamp를 직접 차감하지 않습니다. 새 process 실행은 cold-cache 검증이 없으므로 cold launch로 표현하지 않습니다. E5a event 처리시간과 E5b trial latency는 합치지 않습니다.

## ON/OFF 대조

같은 app의 `--benchmark-smoke` 유무만 바꿉니다. OFF는 새 구조화 계측 이벤트를 내보내지 않아야 합니다. 두 조건에 원래 존재하는 고정 `AETHER_FAKE` start/terminal 로그의 **Harness 수신 시각 차이**를 공통 외부 지표 `external_start_to_terminal_observed`로 사용합니다. OFF에서 내부 Host latency를 관측한 것으로 표현하지 않습니다. 파이프 buffering·reader scheduling·100ms fixture·기존 진단 로그 영향을 모두 포함합니다. 계측 on/off는 기존 logging/polling 전체를 켜고 끄는 실험이 아닙니다.

각 후보 ON/OFF3회는 큰 효과나 수집 결함을 찾는 작은 대조입니다. 오버헤드가 0임을 증명하거나 작은 차이를 인과관계로 확정하지 않습니다. 순서/환경 key가 맞지 않는 표본은 별도 집계합니다. GUI 자동화의 전체 wall time으로 Runtime latency를 대체하지 않습니다.

## 실행과 결과

```sh
# E5b Worktree root, existing tools only
export PATH="$HOME/.cargo/bin:$PATH"
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -v
CARGO_TARGET_DIR="$PWD/target/e5b-issue-27" cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --offline
# apps/desktop
npm run build
node --experimental-strip-types --test tests/fake-provider.test.mjs tests/sidecar-protocol.test.mjs
CARGO_TARGET_DIR="$PWD/../../target/e5b-issue-27" CARGO_NET_OFFLINE=true npm run tauri -- build --bundles app --no-sign -- --locked --offline
# Worktree root; commit source before preparing a frozen plan
python3 scripts/latency_benchmark.py prepare
python3 scripts/latency_benchmark.py check
# Only after the user confirms no concurrent P1 execution and stable screen/power
python3 scripts/latency_benchmark.py run-block --quiet-window-confirmed
python3 scripts/latency_benchmark.py analyze
```

Harness는 GUI를 임의 제어하지 않습니다. 기존 `cua_repl` 또는 사람이 계획에 맞는 A/B 실행, 고정 pause25 선택, 취소와 Quit만 조작합니다. 별도의 `gui.json`에는 실제 관찰한 instance/state/seq/terminal_count만 기록하고 ledger와 일치해야 유효합니다. GUI 기록이 없으면 `gui_pending`이며 PASS 표본으로 집계하지 않습니다. screenshot 생성·자동 클릭은 사용자 시각 승인이 아닙니다.

`target/e5b-issue-27/plan.json`은 source/input/전체 bundle hash와 고정 schedule입니다. `trials/NNN/result.json`은 이벤트·환경·status·metrics, `gui.json`은 별도 실제 관찰값입니다. `summary.json`은 cohort별 planned/attempted/success/failures/timeouts/excluded/gui_pending과 지표별 n/min/max/p50/p95/ms를 담습니다. p50/p95는 정렬된 표본의 nearest-rank ceil(p×n)이며 n=0은 null입니다. raw 결과는 ignored 로컬 디렉터리에만 보존하고 공개 기록에는 비식별 요약을 사용합니다.

자기 Popen의 PID/start time/executable과 Host가 만든 child의 ppid/start/executable만 확인해 정리합니다. 임의 PID·shell·다운로드 인자가 없습니다. 불명확한 PID를 추측해 종료하지 않습니다. Operator90초 timeout과 Runtime 내부 timeout을 구별합니다.

## 미검증과 사용자 시각 승인

WebKit CPU/RSS 귀속 UNAVAILABLE, 실제 renderer crash NOT_RUN, E5c 자원·energy 장시간 측정과 E5d 판단 NOT_RUN. 30개 표본의 p95도 기기/환경/fixture에 한정된 기술통계이며 독립성·일반화·신뢰구간 보장이 아닙니다. Runtime 승자를 선정하지 않습니다.

UI 소스 변경은 원칙적으로 없으며 최종 packaged app을 사용자가 직접 열어 확인할 경로를 제공합니다. 실제 화면 확인과 명시적 시각 승인, 정확한 PR HEAD의 별도 병합 승인이 모두 필요합니다.

VISUAL_DIRECTION_APPROVED: YES
MERGE_ALLOWED: NO
ARCHITECTURE_ACCEPTED: NO
RUNTIME_SELECTED: NO
MAIN_MERGED: NO
NEXT_PHASE_STARTED: NO


## 2026-11-12 실측 결과 · PARTIAL

총144회 계획을 모두 시도했습니다. warm-up12회와 ON/OFF12회는 모두 PASS, 본 측정120회는 PASS118 / TIMEOUT2입니다. 실패·환경 제외는 0이며 누락 표본을 보충하지 않았습니다. B normal·pause25의 유효 n은 각각29이므로 30개 완료 기준을 충족하지 못했습니다.

### 고정 기준과 환경

- 측정 기준 main: `5dd1ae9f8323d96ddac842374706c7fe9edda220`.
- 실제 측정 source: `4b021229f8f9b77115da09d71e6963c7112bef62`. 이후 변경은 Harness 동시 실행 방어·회귀 테스트·결과 문서이며, 측정을 새 HEAD에서 했다고 표현하지 않습니다.
- 전체 bundle manifest SHA-256: `9aead5e26afaa248b009604b9b202f58972d66e65ade8d4f28c0a112cfc3a8b9`.
- 실행 파일 SHA-256: `7d7cfa83d2cbb88d9b019319120c4b895c45427c7b1ae9c0778ce92891a99d64`.
- 앱 입력34개 및 bundle hash는 전후 동일. arm64 packaged app과 pinned Node를 사용했습니다. 실제 cold-cache는 검증하지 않았습니다.
- Apple Silicon arm64, logical CPU10, macOS26.6 / build25G5043d. 결과 파일 저장 시각 범위는 KST16:46–17:58이며 실행 시간 통계가 아닙니다.
- warm-up: BATTERY/discharging, battery28–36%, 1분 load2.577–4.877. 본 측정: AC/charging, battery28–44%, load3.881–7.785. ON/OFF: AC/charging, battery44–45%, load4.238–5.104. 모든 전후 thermal 기록은 NO_WARNING_REPORTED.
- **warm-up 이후 전원 조건이 바뀌었습니다.** 본 측정은 AC로 일관되지만 동일 전원 조건의 warm-up3회는 확보하지 않았습니다. 재실행·보충 없이 한계로 보존합니다. WARMUP_ENVIRONMENT_MATCH: NO.
- 기존900×760 foreground 창, 실제 native accessibility 클릭. 사용자는 본 측정 중 같은 밝기를 유지했다고 확인했습니다. 수치 밝기는 NOT_COLLECTED. 백그라운드 부하를 완전히 통제했다고 주장하지 않습니다.
- P1 잔존 build/다른 AETHER 감지0에서 시작했습니다. 새 main을 fetch/rebase하지 않았습니다. 사후 확인한 GitHub main은 P1.2 설계 문서 커밋 `69f6ee1e6d203d253cd7e6c547a8f837d2703feb`이며 이 변경은 측정 소스에 없습니다.

### 시도 수와 유효 표본

| phase | 후보 | fixture | 전체 시도 | 성공/유효 n | 실패 | TIMEOUT | 환경 제외 |
|---|---|---|---:|---:|---:|---:|---:|
| warmup | webview-a | normal | 3 | 3 | 0 | 0 | 0 |
| warmup | webview-a | pause25 | 3 | 3 | 0 | 0 | 0 |
| warmup | node-b | normal | 3 | 3 | 0 | 0 | 0 |
| warmup | node-b | pause25 | 3 | 3 | 0 | 0 | 0 |
| measurement | webview-a | normal | 30 | 30 | 0 | 0 | 0 |
| measurement | webview-a | pause25 | 30 | 30 | 0 | 0 | 0 |
| measurement | node-b | normal | 30 | 29 | 0 | 1 | 0 |
| measurement | node-b | pause25 | 30 | 29 | 0 | 1 | 0 |

### 본 측정 지연 · ms

각 값은 정상/취소별 trial 단위이며 nearest-rank ceil(p×n)입니다. warm-up·overhead·E5a SMOKE_ONLY는 제외했습니다. 아래 표는 AC cohort 전체의 기술통계이며 후술한 GUI 운영 방식 변경 전후의 통합값입니다. 특히 cancel accept→terminal에는 조작 대기가 포함되므로 Runtime 취소 처리시간으로 해석하지 않습니다.

| 후보 | fixture | 지표 | n | min | max | p50 | p95 |
|---|---|---|---:|---:|---:|---:|---:|
| webview-a | normal | accept_to_first_progress | 30 | 101.822208 | 112.875625 | 103.209292 | 108.696708 |
| webview-a | normal | accept_to_terminal | 30 | 405.510333 | 423.497458 | 408.961125 | 416.787292 |
| webview-a | normal | process_spawn_to_session_ready_observed | 30 | 278.952541 | 847.260667 | 378.154958 | 469.768666 |
| webview-a | pause25 | accept_to_first_progress | 30 | 100.931250 | 126.321458 | 102.484958 | 114.109791 |
| webview-a | pause25 | accept_to_terminal | 30 | 748.258166 | 918.433417 | 782.523333 | 845.177208 |
| webview-a | pause25 | host_cancel_processing | 30 | 0.000583 | 0.003125 | 0.001042 | 0.002250 |
| webview-a | pause25 | process_spawn_to_session_ready_observed | 30 | 291.662750 | 703.595541 | 375.742792 | 433.494291 |
| node-b | normal | accept_to_first_progress | 29 | 125.026875 | 302.555542 | 138.515042 | 284.765541 |
| node-b | normal | accept_to_terminal | 29 | 431.554166 | 625.602750 | 450.504666 | 591.561250 |
| node-b | normal | node_spawn_to_ready | 29 | 22.049292 | 198.023125 | 35.033375 | 134.327000 |
| node-b | normal | process_spawn_to_session_ready_observed | 29 | 315.544625 | 578.841834 | 383.423083 | 501.785875 |
| node-b | pause25 | accept_to_first_progress | 29 | 123.886375 | 459.909667 | 131.608916 | 316.493333 |
| node-b | pause25 | accept_to_terminal | 29 | 601.140000 | 1053.976541 | 790.322167 | 902.831000 |
| node-b | pause25 | cancel_confirmed_to_reap | 29 | 1.254542 | 18.216500 | 9.086917 | 12.949500 |
| node-b | pause25 | host_cancel_processing | 29 | 0.000542 | 0.005750 | 0.001083 | 0.005667 |
| node-b | pause25 | node_spawn_to_ready | 29 | 21.663500 | 357.434250 | 28.269500 | 207.334334 |
| node-b | pause25 | process_spawn_to_session_ready_observed | 29 | 288.526041 | 762.404166 | 370.684875 | 606.633500 |

### 실패 및 GUI 운영 방식의 변경

0-based trial039(B pause25)는 Harness instance와 화면 instance가 달랐습니다. 종료 직후 getAXState가 별도 앱을 다시 연 것으로 추정하며, Harness에는 host_main/session_ready만 있고 host_accept는 없습니다. 일치하지 않은 GUI receipt는 gui-unmatched.json으로 보존하고 유효 n에서 제외했습니다. 이때 block 종료를 기다리지 않고 시작한 trial040(B normal)도 작업 클릭 없이 같은 두 이벤트만 남았습니다. 둘 다 Harness90초 TIMEOUT, 자기 Host SIGTERM 종료 코드-15입니다. Runtime 내부 TIMEOUT이나 Node 실행 실패로 분류하지 않습니다. 둘의 프로세스 생존이 잠시 겹친 사실도 보존합니다.

trial041부터 Quit 뒤 getAXState를 호출하지 않고 프로세스 종료와 다음 current.json을 확인한 뒤 다음 창을 조회했습니다. 이후 추가 TIMEOUT은 없었습니다. 원래 데이터·계획·SHA·artifact는 바꾸지 않았으며 실패 시도 대체도 없습니다.

실측 종료 후 Harness에 nonblocking flock을 추가해 같은 worktree의 두 collector가 겹치지 않도록 했고, launch 전에는 동일 E5b artifact의 기존 앱도 차단합니다. 실제 별도 Python process의 lock 획득/해제 테스트와 기존 앱 감지 테스트가 통과했습니다. 이 보완 버전으로 반복 실측을 다시 수행한 것은 아닙니다. GUI instance 대조는 계속 필요합니다.

운영 방식 변경 전후를 분리한 민감도 표입니다. 모든 값은 ms이고 유효 trial만 사용했습니다. 아래 cohort는 각각30개가 아니므로 통합 표를 대체하는 완료 증거가 아닙니다.

| GUI 방식 | 후보 | fixture | n | first progress p50/p95 | terminal p50/p95 | cancel 처리 p50/p95 |
|---|---|---|---:|---|---|---|
| initial | webview-a | normal | 7 | 103.182292 / 108.696708 | 407.749459 / 416.787292 | N/A |
| initial | webview-a | pause25 | 7 | 102.845375 / 104.256084 | 783.091500 / 918.433417 | 0.000875 / 0.001209 |
| initial | node-b | normal | 7 | 156.899000 / 284.765541 | 463.264709 / 591.561250 | N/A |
| initial | node-b | pause25 | 6 | 125.992791 / 165.592667 | 766.532250 / 794.368083 | 0.001000 / 0.002000 |
| quit-without-requery | webview-a | normal | 23 | 103.281750 / 107.575167 | 409.389708 / 416.178334 | N/A |
| quit-without-requery | webview-a | pause25 | 23 | 102.404666 / 114.109791 | 782.523333 / 831.924708 | 0.001042 / 0.002250 |
| quit-without-requery | node-b | normal | 22 | 135.892791 / 238.706709 | 445.286209 / 546.581709 | N/A |
| quit-without-requery | node-b | pause25 | 23 | 136.380458 / 316.493333 | 790.486958 / 902.831000 | 0.001084 / 0.005667 |

### ON/OFF 공통 외부 지표 · ms

external_start_to_terminal_observed의 작은 반복 대조입니다. OFF의 구조화 계측 이벤트는0개이며 OFF 내부 Host latency는 수집하지 않았습니다. 아래 각 조건의 실패/TIMEOUT/환경 제외는 모두0입니다.

| 후보 | 계측 | 시도/유효 n | min | max | p50 | p95 |
|---|---|---:|---:|---:|---:|---:|
| webview-a | ON | 3/3 | 405.658375 | 407.517417 | 406.498667 | 407.517417 |
| webview-a | OFF | 3/3 | 405.489625 | 407.800041 | 406.088541 | 407.800041 |
| node-b | ON | 3/3 | 405.780167 | 463.693750 | 408.020125 | 463.693750 |
| node-b | OFF | 3/3 | 407.507458 | 411.459542 | 407.633458 | 411.459542 |

ON−OFF p50 차이는 A +0.410126ms, B +0.386667ms입니다. 각 조건 n=3이고 B ON 최대463.694ms 등 변동이 있으므로 차이를 계측의 인과적 비용으로 확정하거나 오버헤드0이라고 주장하지 않습니다.

### 검증과 화면 승인

| 검사 | 실제 결과 |
|---|---|
| Python regression | PASS: 실측 전후11개, 실측 후 collector 방어 보완 포함13개 |
| Rust locked/offline | PASS: 19개, 기존 승인/replay/cancel/lifecycle/실제 Node 회귀 포함 |
| Node/provider | PASS: 8개 |
| TypeScript/Vite/sidecar production build | PASS: 타입 검사, Vite8.3.6, pinned Node/LICENSE |
| macOS .app packaging | PASS: 실측 전에 local Tauri CLI --bundles app --no-sign -- --locked --offline로 빌드. 고정 artifact를 유지하여 실측 후 bundle 재빌드는 하지 않음 |
| executable | PASS: Info.plist의 aether-desktop, Mach-O arm64 |
| native CLI | PASS: AETHER_NATIVE_MACOS_MAJOR=26, sw_vers26.6과 일치, exit0 |
| 실제 packaged GUI | PASS: 유효142회 화면 instance/상태/seq/terminal 대조. B 성공·취소에서 reap 확인. 두 TIMEOUT GUI는 미검증 |
| 기존 기타 버튼 | 초기 코어/카운터/버전/Accessibility UI 존재 확인. 별도 클릭 회귀 NOT_RUN: 이번 자동화는 고정 A/B normal·pause25/cancel로 제한 |
| governance/whitespace | PASS: 21 artifacts 및 diff 검사; 최신 PR CI는 PR에 별도 기록 |
| 기존 산출물 보존 | PASS: 보존 manifest의 artifact14개 size/mtime와 E5a 결과29개 hash 동일 |
| 사용자 시각 승인 | YES: 사용자가 실제 측정 앱에서 기존 UI 유지 확인. P1.1/P1.2 디자인 승인·병합 승인에 해당하지 않음 |

공개 기록에는 개인 절대 경로·호스트명·환경 전체·원본 로그·사용자 입력을 포함하지 않습니다. 원본 구조화 결과와 GUI receipt는 ignored target 디렉터리에 보존합니다. summary.json SHA-256은 `301980e161e4dc9aebe96769be56181632e34b5e7cc88de705872eb57802fd91`입니다. UI/Rust/Swift/Node 구현, dependencies/lockfile, capability/CSP는 변경하지 않았습니다.

검토용 실행 명령(E5b Worktree root):

```sh
open -n "$PWD/target/e5b-issue-27/release/bundle/macos/AETHER.app"
```

측정 source와 최종 보고 HEAD가 달라졌으므로 기존 sealed plan으로 run-block/check를 실행하면 SOURCE_CHANGED로 거부되는 것이 정상입니다. analyze는 보존된 결과를 재집계할 수 있습니다. 새 계획은 별도 승인·새 결과 경로로 마련해야 하며 원래 plan/trials를 지우거나 덮어쓰지 않습니다.

B 표본 부족과 warm-up 전원 불일치, GUI 운영 방식 변경 때문에 `P0_B_E5B_LATENCY_PARTIAL`입니다. 동일 환경의 warm-up과 충분한 유효 표본을 얻는 새 측정 계획의 검토가 남았습니다. 새 실험은 이번 작업에서 자동 시작하지 않습니다. WebKit 귀속 UNAVAILABLE, renderer crash/E5c/E5d NOT_RUN을 유지합니다.

VISUAL_DIRECTION_APPROVED: YES
MERGE_ALLOWED: NO
ARCHITECTURE_ACCEPTED: NO
RUNTIME_SELECTED: NO
MAIN_MERGED: NO
NEXT_PHASE_STARTED: NO
