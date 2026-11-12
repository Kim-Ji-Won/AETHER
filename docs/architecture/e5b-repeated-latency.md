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

VISUAL_DIRECTION_APPROVED: NO
MERGE_ALLOWED: NO
ARCHITECTURE_ACCEPTED: NO
RUNTIME_SELECTED: NO
MAIN_MERGED: NO
NEXT_PHASE_STARTED: NO
