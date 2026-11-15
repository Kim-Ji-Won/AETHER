# E5 Recovery 실행 결과 · Issue 44

2026-11-15 · [Issue #44](https://github.com/Kim-Ji-Won/AETHER/issues/44) · **PARTIAL / 실행 중단**.
[사전 등록 계획](e5-recovery-plan.md)과 [Runtime ADR PROPOSED](../decisions/001-foreground-runtime-proposal.md)를 함께 읽는다.

## Provenance와 원본 보존

| 대상 | 값 |
|---|---|
| 기준 main | `f44a6c07cec2940633c63e67613dd1259549b1f7` |
| 실제 측정 Harness source | `cd25da37be683a48895dfa7c57c464c56537b757` |
| 독립 Run ID | `29069f377e614aaba48b5f7cdda88fd6` |
| plan SHA-256 | `73e2ca3022160c4f15d15d39207583b7140a7c1ac6213850f0430ef24f8d881e` |
| 고정 앱 build source | `4b021229f8f9b77115da09d71e6963c7112bef62` |
| bundle manifest SHA-256 | `9aead5e26afaa248b009604b9b202f58972d66e65ade8d4f28c0a112cfc3a8b9` |
| 실행 파일 SHA-256 | `7d7cfa83d2cbb88d9b019319120c4b895c45427c7b1ae9c0778ce92891a99d64` |
| 원본 #34 최종 HEAD | `b70fe0bd36538c6df7f44bfe07e7cbce3b30ceff` |

원본 #27/#34의 PARTIAL, plan/trials/summary 289개와 34개 app source 입력을 해시로 재확인했다. 원본 summary SHA-256 `301980e161e4dc9aebe96769be56181632e34b5e7cc88de705872eb57802fd91`과 bundle이 일치한다. 원본의 B normal/cancel n29 및 TIMEOUT 2건을 수정·통합하지 않았다.

이번 실행 증거 manifest는 64개 파일을 기록하며 manifest SHA-256은 `5c8b67daff6ea298c635d3f3445ca450c71435ae1627ff7489a9c0053529536b`이다.

이번 출력은 독립 worktree의 `target/recovery-44/`에만 있다. plan, baseline, result, GUI receipt, summary, stop latch와 evidence manifest를 보존한다. 공개 문서에는 개인 절대 경로·전체 환경·원본 로그를 넣지 않는다. 최종 PR의 child discovery 보완은 **측정 후 수정**이며 위 source에서 수행한 실행 결과를 수정 후 재검증으로 표현하지 않는다.

## 환경과 실제 Baseline

Apple Silicon arm64, macOS 26.6 build 25G5043d, logical CPU 10, AC, 배터리 100%. `pmset` thermal 경고 미보고이며 온도 수치 측정이 아니다. 사용자 승인에 따라 기존 밝기·화면 배치를 유지했고 밝기 수치는 NOT_COLLECTED이다. 고정 앱의 기본 900×760 창과 기존 E5a 화면을 사용했다. 이번 실행에서 OS window geometry를 별도로 계측하지 않아 픽셀 크기 독립 확인은 NOT_RUN이다. GUI는 기존 `cua_repl` native AX 클릭이며 사람 직접 조작·시각 승인과 구분한다.

PostgreSQL 다른 client/active transaction 0, Docker 실행 컨테이너 0, booted Simulator 0, 외부 build/server 및 다른 AETHER 0을 확인했다. Codex helper는 실제 개발 서버와 구분했다. PostgreSQL/Docker 정상 중지 후 60초 정착 + 60초/5초 간격 12표본을 **한 번** 관찰했다.

| 지표 | 앞 30초 중앙값 | 뒤 30초 중앙값 | 차이 | 사전 기준 | 결과 |
|---|---:|---:|---:|---:|---|
| GPU Device Utilization | 16.0% | 15.0% | 1.0pp | ≤5pp | PASS |
| WindowServer CPU | 25.1% | 26.45% | 1.35pp | ≤10pp | PASS |

GPU 절대 cutoff는 적용하지 않았다. `ioreg`/`ps`는 간접 관찰값이며 GPU energy나 앱별 귀속이 아니다. 실제 앱 활동 구간에 idle drift 기준을 적용하지 않았다. 동일 AX 절차, source, artifact를 유지했지만 GUI·Renderer와 Harness 진단 자체의 관측 비용은 제거하지 못했다. ON/OFF 대조 전에 중단했으므로 그 비용의 크기는 UNKNOWN이다.

## 144회 계획과 실제 분모

| phase / 조건 | 계획 | 시도 | 성공·유효 n | FAIL | TIMEOUT | EXCLUDED | NOT_RUN |
|---|---:|---:|---:|---:|---:|---:|---:|
| warm-up A normal | 3 | 3 | 3 | 0 | 0 | 0 | 0 |
| warm-up A pause25 | 3 | 3 | 3 | 0 | 0 | 0 | 0 |
| warm-up B normal | 3 | 3 | 3 | 0 | 0 | 0 | 0 |
| warm-up B pause25 | 3 | 3 | 3 | 0 | 0 | 0 | 0 |
| measurement A normal | 30 | 5 | 5 | 0 | 0 | 0 | 25 |
| measurement A pause25 | 30 | 4 | 4 | 0 | 0 | 0 | 26 |
| measurement B normal | 30 | 5 | 4 | 0 | 0 | 1 | 25 |
| measurement B pause25 | 30 | 4 | 4 | 0 | 0 | 0 | 26 |
| ON/OFF 합계 | 12 | 0 | 0 | 0 | 0 | 0 | 12 |
| 전체 | 144 | 30 | 29 | 0 | 0 | 1 | 114 |

warm-up 12개는 아래 통계에서 제외했다. 본 측정의 유효 표본은 17개이고 어떤 조건도 n30을 충족하지 못했다. B normal 029는 GUI terminal receipt가 없고 metric도 성공 통계에 포함하지 않는다. 실패를 성공으로 채우기 위한 보충·교체·Baseline 재시도는 하지 않았다.

## 중단 원인과 수정 범위

index 029 / B normal에서 Harness가 `ENVIRONMENT_CONFLICT`를 기록하고 identity가 확인된 자기 Host에 SIGTERM을 보냈다. Host exit는 -15이며 측정 trace는 `host_main → session_ready → host_accept → child_spawn`까지만 남았다. stop latch가 이후 trial을 막았고 collector/driver가 종료됐다. 실제 Node Runtime 실패나 Runtime timeout으로 해석하지 않는다.

**확인한 코드 결함:** child stderr diagnostic을 읽어 `children`에 등록하기 전에 다음 process 검사 tick이 실행되면, 방금 spawn된 자기 Node를 외부 Node로 분류할 수 있었다. 같은 process snapshot을 사용하는 unit 재현에서 이전 분류는 외부 1, 수정한 분류는 외부 0으로 확인했다. 수정은 parent·실행 파일·시작 identity를 대조해 같은 snapshot에서 child를 찾아 등록한다. 다른 parent와 재사용된 Host identity는 계속 차단한다. 다음 실행의 원인 조사에 쓸 비식별 conflict counts도 추가했다.

**미확정 부분:** 당시 conflict PID/category를 저장하지 않아 실제 중단의 단독 원인이 이 race였다고 입증할 수 없다. 중단 직후 외부 build/server 0, 다른 AETHER 0, PostgreSQL/Docker 0이었다. trace의 child_spawn 직후 중단과 결함은 일치하지만 사후 관찰을 당시 snapshot으로 대체하지 않는다. 따라서 raw `EXCLUDED / ENVIRONMENT_CONFLICT`를 그대로 보존한다. 수정 후 새 Trial은 NOT_RUN이다.

## 관찰된 지연 · 제한된 기술 통계

단위는 모두 ms, nearest-rank `ceil(p*n)`. n4~5의 p95는 최대값이며 안정적인 tail 추정이나 후보 순위 근거가 아니다. normal과 pause25, trial과 event를 섞지 않았다. pause25 accept→terminal에는 GUI가 취소를 누르기까지의 시간이 포함된다. Host cancel 처리시간은 아주 짧은 코드 구간의 계측값이며 end-to-end 취소 UX가 아니다. process start→session ready는 외부 관찰이며 cold-cache launch/paint가 아니다.

| 조건 | 지표 | n | min | max | p50 | p95 |
|---|---|---:|---:|---:|---:|---:|
| webview-a normal | `accept_to_first_progress` | 5 | 102.411125 | 104.303208 | 103.238916 | 104.303208 |
| webview-a normal | `accept_to_terminal` | 5 | 406.451459 | 409.987042 | 408.161708 | 409.987042 |
| webview-a normal | `external_start_to_terminal_observed` | 5 | 406.009209 | 407.763000 | 407.573625 | 407.763000 |
| webview-a normal | `process_spawn_to_session_ready_observed` | 5 | 305.505125 | 485.612542 | 371.888250 | 485.612542 |
| webview-a pause25 | `accept_to_first_progress` | 4 | 102.627375 | 103.170459 | 102.757208 | 103.170459 |
| webview-a pause25 | `accept_to_terminal` | 4 | 777.302625 | 798.796375 | 782.276375 | 798.796375 |
| webview-a pause25 | `host_cancel_processing` | 4 | 0.000709 | 0.001208 | 0.000750 | 0.001208 |
| webview-a pause25 | `process_spawn_to_session_ready_observed` | 4 | 307.536958 | 390.068000 | 359.606833 | 390.068000 |
| node-b normal | `accept_to_first_progress` | 4 | 126.892042 | 162.548209 | 130.137667 | 162.548209 |
| node-b normal | `accept_to_terminal` | 4 | 433.441875 | 470.915375 | 436.824416 | 470.915375 |
| node-b normal | `external_start_to_terminal_observed` | 4 | 407.616208 | 409.709833 | 408.085375 | 409.709833 |
| node-b normal | `node_spawn_to_ready` | 4 | 23.407958 | 59.553375 | 25.241084 | 59.553375 |
| node-b normal | `process_spawn_to_session_ready_observed` | 4 | 316.237375 | 391.648083 | 324.557042 | 391.648083 |
| node-b pause25 | `accept_to_first_progress` | 4 | 127.717458 | 129.432584 | 128.956292 | 129.432584 |
| node-b pause25 | `accept_to_terminal` | 4 | 769.186042 | 788.383458 | 772.269167 | 788.383458 |
| node-b pause25 | `cancel_confirmed_to_reap` | 4 | 5.644500 | 8.197625 | 7.422667 | 8.197625 |
| node-b pause25 | `host_cancel_processing` | 4 | 0.000958 | 0.001667 | 0.001209 | 0.001667 |
| node-b pause25 | `node_spawn_to_ready` | 4 | 24.255333 | 25.695084 | 24.968750 | 25.695084 |
| node-b pause25 | `process_spawn_to_session_ready_observed` | 4 | 306.197875 | 374.301166 | 340.302875 | 374.301166 |

## E5c, E5d 및 복원

E5c MINIMUM_SCOPE는 후보별 3회, 1초 sampling과 idle/foreground long60/hidden/cancel/Reload/Quit 관찰 시간을 source commit 전에 고정했다. E5b stop latch 때문에 **BLOCKED / NOT_RUN**이다. Host/Node CPU·RSS, 반복 RSS 증가, resource recovery의 새 실측 증거는 없다. WebKit 귀속 UNAVAILABLE, GPU/energy·battery·장시간 상주·renderer crash NOT_RUN을 유지한다.

고정 combined 앱은 122,070,398 bytes, 그중 Node 실행 파일은 111,534,432 bytes이다. A-only/B-only 별도 빌드 크기나 실행 RSS 차이로 해석하지 않는다. 기존 E4e lifecycle 근거는 그 문서의 각 artifact와 재검증 범위에 한정한다.

PostgreSQL `brew services start postgresql@16` exit 0, 등록 직후 readiness 실패 뒤 해당 LaunchAgent 정상 `launchctl kickstart` exit 0, SQL `SELECT 1` exit 0으로 복원했다. `sh.brew.postgresql@16`의 RunAtLoad/KeepAlive는 true이다. Docker `desktop start` exit 0, 최종 `desktop status` running, 로컬 engine 29.9.3 응답·실행 컨테이너 0을 확인했다. 데이터·WAL·설정·Volume 삭제/수정, force kill/prune/reset은 하지 않았다.

중단 trial Host/Node PID는 모두 소멸했고 다른 AETHER 및 개발 서버 후보 0, collector/driver 종료를 확인했다. E5 실측은 끝났으므로 다른 프로젝트 개발을 재개해도 된다. 새 측정은 자동 시작하지 않는다.

## 검증과 남은 조건

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -v`: 22 tests PASS. 기존 parser 6개 + 반복/guard/child discovery 검사. 실제 GUI 검증과 별도이다.
- 보존된 원본의 instrumented PASS trace 136개를 parser로 읽기 전용 재검증: PASS. 원본 결과는 변경하지 않음.
- 실제 fixed app GUI: 완료한 29회에서 새 instance, run 1, 정상 seq5/FAKE_OK 또는 취소 seq3, terminal1, Node reap, 정상 Quit exit0 확인. 029 GUI는 NOT_COMPLETED.
- `python3 scripts/validate_governance.py`, `git diff --check`, staged diff: 제출 시 확인. 앱·Swift·IPC·의존성 변경 없음. 새 앱 build/Rust/Node 재실행은 NOT_RUN이며 고정 앱의 해시·arm64와 실제 실행을 검증했다.
- E5d는 [ADR PROPOSED](../decisions/001-foreground-runtime-proposal.md)까지만 작성한다. 새 자원 측정과 30회 지연 근거가 없어 Runtime 선택을 확정하지 않는다.
- 후속 실측은 별도 Run ID/output/source seal과 환경 예약이 필요하다. 현재 plan/stop latch를 지우거나 이번 표본을 30회까지 임의 보충하지 않는다.

E5B_RECOVERY: PARTIAL
E5C_MINIMUM_SCOPE: BLOCKED
E5D_ADR: PROPOSED
ARCHITECTURE_ACCEPTED: NO
RUNTIME_SELECTED: NO
MODEL_CALL_EXECUTED: NO
MAIN_MERGED: NO
