# E5a 측정 기반 · Performance Measurement Foundation

[Issue #25](https://github.com/Kim-Ji-Won/AETHER/issues/25). 기준 main `4188661bbdb95fd7f119028c9df114c27aa43a24`.
[E4 계획](e4-runtime-comparison-plan.md)의 E5 계측을 위한 기반입니다. RFC는 **PROPOSED**, Runtime 미선정입니다. Smoke는 수집 경로 시험이며 성능 비교 결과·예산 통과·전체 benchmark 완료가 아닙니다.

## 목적과 비교 조건

가설은 동일 Host 경계에서 A/B 수락·처리 시각을 관찰하고 B의 child 준비 비용을 분리할 수 있다는 것입니다. WebView A와 bundled Node B는 같은 app/source, `fake-provider.ts`의 고정 합성 입력, ACK 이후 100ms 간격, 이벤트 계약, UI와 원장을 사용합니다. 일반 smoke의 순서는 A normal → B normal → B pause25 cancel → A pause25 cancel입니다. 실행마다 새 app instance와 trial이며 run ID는 같을 필요가 없습니다. 취소 표본에는 사람/GUI 도구의 대기 시간이 포함되므로 정상 표본과 섞지 않습니다.

같은 Mac/macOS build, foreground 화면, 전원·부하 조건을 기록합니다. Harness가 환경을 변경하거나 사용자 앱을 종료하지 않습니다. 환경은 순간 관찰이며 지속 통제 보장이 아닙니다. 실제 배터리 상태와 AC 연결 표시는 독립 필드입니다. 화면 밝기/주사율은 자동 수집하지 않습니다. GUI 조작·snapshot·다른 사용자 앱·빌드 부하가 편향을 만듭니다.

## 측정 정의와 현재 수집 경계

모든 유효 latency는 같은 clock domain의 끝−시작입니다. `ms`는 ns/1e6입니다. 값이 없으면 `null`과 이유를 사용하며 0으로 대체하지 않습니다. 아래 미구현 지표는 정의만 있으며 E5a 수집 성공을 주장하지 않습니다.

| 지표 | 단위 / 시작 → 끝 | 도구·수집 위치 | 한계 / 유효·무효 기준 |
|---|---|---|---|
| process launch 관찰 | ms / Harness Popen 직전 → 첫 진단 수신 | Python monotonic / Harness | spawn+loader+pipe+reader scheduling 포함. kernel process 생성·cold launch 아님. 같은 Popen PID·trial 필수 |
| Host main → session ready | ms / Rust init → 첫 get_runtime_session | Rust Instant / stderr | WebView가 IPC 가능해진 proxy. 첫 paint·실제 창 표시 아님; 같은 instance 필요 |
| 첫 창 표시 | ms / process 생성 → 첫 compositor frame | 향후 Instruments/검증된 frame hook | E5a NOT_RUN. setup callback·open exit로 대체 금지 |
| Runtime 준비 | ms / Host accept → Node ready 수신, spawn → ready 별도 | Rust Instant / Host | B의 loader·handshake·IPC 포함. A는 이미 UI를 실행한 WebView이므로 대칭 cold-ready 비용은 NOT_RUN |
| 첫 작업 가능 | ms / launch → 버튼 사용 가능 | GUI 관찰, 향후 계측 | E5a 숫자 NOT_RUN; session_ready만 가능성 proxy |
| UI start → Host 수락 | ms / UI invoke 직전 → Host accept | 향후 같은 clock RTT/분리 단계 계약 | process clock 직접 차감 금지. E5a NOT_RUN |
| Host accept 처리 | ms / begin 진입 → ledger 수락 후 계측 | Rust Instant / Host | envelope 검증·Node resource 경로 해석은 이전에 끝남. 전체 IPC latency 아님 |
| accept → 첫 progress | ms / ledger 수락 → 25% 수락 | Rust Instant / Host | 100ms fixture, ACK, B 준비 비용 포함. 정상 seq/ID일 때만 유효 |
| progress 처리 | ms / submit_from 진입 → 상태 전이·lock 해제 | Rust Instant / Host | JSON parsing/pipe transport 이전 단계 제외. event당 값; 평균 UI 지연 아님 |
| Provider → Host 전달 | ms / Provider emit → Host receive | 향후 clock-domain-safe 계약 | E5a NOT_RUN, 서로 다른 clock 차감 금지 |
| cancel 처리 | ms / Host cancel 진입 → 취소 상태 수락 | Rust Instant / Host | A cancelled, B cancel_requested. UI→Host 구간 제외 |
| cancel → Node reap | ms / Host cancel 확인 → child wait 완료 | Rust Instant / Host | B만 해당, 자기 child의 reaped 이벤트·PID 필요 |
| 작업 → terminal | ms / Host accept → A terminal 또는 B run_final | Rust Instant / Host | B는 reap 후 final. 정상/취소 별도 분석, terminal_count=1 필수 |
| Host/Node CPU | percent / 첫 sample → 마지막 sample의 cumulative CPU 차이 | ps time + Python monotonic / Harness | user+system CPU Δ/elapsed×100, 100%=1 core; 전체 GUI 대기 구간 포함. ps %cpu의 약 1분 감쇠 평균 사용 안 함 |
| Host/Node RSS | bytes / 각 순간값, 관찰된 max | ps rss KiB×1024 / Harness | 목표 250ms, ps 호출·스케줄 지연. transient child 누락 가능, 진짜 peak 아님 |
| 관련 프로세스 합계 RSS | bytes / 같은 interval의 process별 합 | 향후 귀속 검증 | E5a 합산 안 함. 공유 memory 이중 계산·WebKit 별도 process 때문에 전체 app memory 아님 |
| WebView renderer/helper CPU·RSS | percent / bytes | ps 관찰·Instruments 후보 | WebKit PID 귀속 검증 미완료, UNAVAILABLE. Host 값으로 전체 사용량 주장 금지 |
| GPU | utilization / interval | Metal System Trace 후보 | E5a NOT_RUN, template 설치만 확인; process 귀속·권한 미검증 |
| energy / idle wakeups | tool-specific / interval | powermetrics, Instruments 후보 | powermetrics superuser 요구 → BLOCKED. sudo 없음. 배터리 %로 정밀 에너지 추정 금지 |
| app / bundled runtime 크기 | bytes / sealed artifact | Python stat/hash / manifest | logical bytes, 압축·APFS 실제 점유량 아님. 같은 app에 A/B 모두 포함되어 variant별 배포 크기 비교 아님 |
| foreground/hidden/minimized | 조건별 동일 측정 | GUI+Harness manifest | E5a foreground만. 나머지는 E5c NOT_RUN |
| idle / active long60 | 조건별 CPU/RSS·진행 | 향후 E5c | E5a long60 실행 옵션 없음; 기존 기능은 유지 |
| 정상 Quit / 잔존 | exit code, count / 사용자 Quit → Popen wait 및 child 재확인 | Harness process handle+OS identity | 이번에 생성한 Host/child만. timeout 강제 정리는 정상 Quit PASS 아님 |

[Instant](https://doc.rust-lang.org/std/time/struct.Instant.html)는 wall clock과 별개입니다. Rust init 이후 elapsed ns와 Python monotonic ns는 서로 빼지 않습니다. 기존 E4e wall-clock 진단은 수집 대상에서 제외합니다. `operation_ns`에는 clock 읽기·계측 lock 획득 비용 일부가 포함됩니다. sleeping/clock anomaly/순서 역전은 유효 비교 근거가 아닙니다.

## 구현·데이터 계약

- `src-tauri/src/measurement.rs`: 기본 off. 실행 옵션 `--benchmark-smoke`, 32자리 hex `AETHER_BENCH_TRIAL`, 고정 `AETHER_BENCH_VARIANT`가 모두 있어야 활성화합니다. 환경을 전체 출력하지 않습니다. IPC·권한이나 승인 경계가 아니며 원장 상태에 영향을 주지 않습니다.
- `fake_run.rs`: 기존 ledger 전이 후 lock을 해제하고 수락/이벤트/취소를 관찰합니다. ledger의 ID/seq/terminal 정책은 유지합니다.
- `sidecar.rs`: 기존 spawn/검증된 ready/reap/final 지점을 관찰합니다. Node script·pin·protocol은 변경하지 않습니다.
- `main.rs`: init과 기존 session 조회에만 관찰점을 둡니다. 새로운 IPC/capability/CSP/Swift 변경 없음.
- `scripts/benchmark.py`: 표준 Python만 사용. 고정된 E5a packaged app만 실행하며 CLI에 임의 command/path/PID/download/repeat 옵션이 없습니다. GUI 버튼은 실제 사람이거나 기존 GUI 자동화가 누릅니다. 자동화 클릭은 사람 동의 증명이 아닙니다.

이벤트는 `AETHER_MEASURE ` 뒤 closed JSON, 최대 128개입니다. 모든 이벤트는 schema_version=1, trial_id, variant, component=host, app_instance, process_pid, run_id/request_id/generation/seq, event_type, monotonic_ns, nullable operation_ns/exit_code/child_pid, outcome, progress, terminal_count, result, fixture를 가집니다. 실제 child 관찰도 Host clock과 Host PID가 발행하고 child_pid는 관찰 대상입니다. 작업 전 이벤트의 run IDs=0은 미할당이며 결측 성능값 0을 의미하지 않습니다.

A/B 정상 이벤트는 host_accept → [B spawn/ready] → start → 25 → 50 → 75 → FAKE_OK → [B reap/final]입니다. B의 terminal event와 final은 같은 terminal 상태의 서로 다른 관찰점이며 terminal 두 건을 의미하지 않습니다. parser는 정확히 하나의 수락, 일치하는 ID·variant·fixture·instance, 단조 시간, 연속 seq, 정상 progress 배열과 terminal 1, B child lifecycle를 검사합니다. 취소는 pause25 고정 fixture를 사용합니다. 실패한 표본·timeout은 성공 latency에서 제외하되 분모와 이유를 보존합니다.

## Harness와 재현

기존 E4 산출물을 덮어쓰지 않습니다. E5a 전용 `target/e5a-issue-25`를 사용합니다.

```sh
# repository root
export PATH="$HOME/.cargo/bin:$PATH"
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -v
CARGO_TARGET_DIR="$PWD/target/e5a-issue-25" cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --offline
# apps/desktop
npm run build
node --experimental-strip-types --test tests/fake-provider.test.mjs tests/sidecar-protocol.test.mjs
CARGO_TARGET_DIR="$PWD/../../target/e5a-issue-25" CARGO_NET_OFFLINE=true npm run tauri -- build --bundles app --no-sign -- --locked --offline
# back at repository root, immediately after build, with no intervening app edits
python3 scripts/benchmark.py --seal-build
python3 scripts/benchmark.py --dry-run --variant webview-a
python3 scripts/benchmark.py --smoke --variant webview-a --fixture normal
python3 scripts/benchmark.py --smoke --variant node-b --fixture normal
python3 scripts/benchmark.py --smoke --variant node-b --fixture pause25
python3 scripts/benchmark.py --smoke --variant webview-a --fixture pause25
```

각 smoke는 **한 번만** 실행합니다. 요청한 A/B 버튼을 누르고 trace/terminal을 확인한 뒤 `Cmd+Q`로 끝냅니다. pause25는 기존 `25%에서 대기`를 선택하고 실행 → 25% → 취소 → cancelled를 확인합니다. 90초 timeout은 operator 조작 대기도 포함하며 runtime 성능 timeout과 다릅니다. 임의 반복 옵션은 없습니다. dry-run은 앱을 실행하지 않고 source/전체 bundle hash/Node pin/arm64와 seal 일치를 확인합니다.

seal은 기록된 빌드 직후의 입력 content hash와 bundle 파일 hash를 묶습니다. 자체적으로 compiler provenance를 증명하지는 않습니다. smoke 시 현재 HEAD·dirty 상태·build 당시 HEAD를 함께 저장하고 입력 내용이 바뀌면 거부합니다. 문서만 추가 커밋한 경우 앱 입력 hash가 같다는 증거와 최종 HEAD를 별도로 대조해야 합니다. 변조 검증기는 신뢰된 빌드 절차를 대체하지 않습니다.

실행 경로는 Info.plist의 고정 CFBundleExecutable과 arm64를 확인합니다. 직접 만든 Popen Host와 child_spawn 이벤트의 자식만 OS ppid/start time/executable을 private 메모리에서 대조합니다. 사용자 PID는 받지 않습니다. 종료 후 EOF 정리를 5초 관찰하며 잔존 child를 정리하면 해당 표본은 FAIL입니다. 평소에는 사용자의 정상 Quit을 기다립니다. timeout에는 자기 Popen만 terminate/kill하고 정확히 식별한 자기 child만 정리합니다. OS identity를 확인하지 못한 process를 추측해 종료하지 않습니다.

## 결과 파일과 분석

`target/e5a-issue-25/results/<trial_id>/`는 Git ignore됩니다.

- `manifest.json`: source SHA/입력 hash/fixture hash, app 파일별 hash와 전체 hash, Node version/hash/LICENSE, 비식별 환경 ID, macOS/build/architecture/전원·배터리/load/thermal 수집 가능 여부, 계측 on, sample interval.
- `events.jsonl`: 허용 schema의 Host 이벤트만. 기존 stderr 원문은 버립니다.
- `resources.jsonl`: 본 시험 Host/Node PID, Harness monotonic ns, cumulative CPU seconds, RSS bytes. 개인 경로/OS start string은 저장하지 않습니다.
- `results.json`: trial_id, variant, source_sha, artifact_hash, fixture, environment_id, measurement, value, unit, status, failure_reason.
- `summary.json`: SMOKE_ONLY, 전체 상태/Host exit, 항목별 n/total/min/max/p50/p95/실패/timeout/미수집 수. GUI는 별도 관찰해야 하며 parser 성공으로 GUI PASS를 만들지 않습니다.

status는 PASS/FAIL/TIMEOUT/NOT_RUN/UNAVAILABLE/BLOCKED입니다. 누락·NaN·Infinity·음수·bool·extra key는 거부합니다. 성공값만 정렬해 nearest-rank `ceil(p×n)`번째를 선택합니다. n=0이면 통계도 null입니다. event당 처리시간 표본과 trial당 latency는 서로 다른 분모이며 같은 통계로 섞지 않습니다. n=1 smoke의 p95는 성능 추정이 아닙니다. 단위가 다른 값·variant·measurement는 그룹을 분리합니다.

raw artifact는 로컬 target에 보존하고 공개 Issue/PR에는 비식별 요약과 hash/명령/exit만 기록합니다. 화면 원본·사용자 입력·음성·키·개인 경로·호스트명·serial·환경 전체를 저장하지 않습니다. 정책 변경 없이 원본 자동 업로드 없음.

## 설치·권한 관찰 및 후속

2026-11-12 현재 macOS 26.6 build 25G5043d arm64. Xcode의 xctrace 설치와 Time Profiler/Activity Monitor/Metal System Trace template를 확인했습니다. sandbox에서 cache 접근이 막힌 첫 조회는 실패했고, 허용된 외부 실행으로 목록 조회만 성공했습니다. 실제 Instruments recording/권한 요구는 NOT_RUN입니다. `powermetrics -n 1 -i 100`은 superuser 요구로 차단됐으며 sudo를 사용하지 않았습니다. `pmset -g therm`은 현재 OS에서 오류를 반환해 thermal은 UNAVAILABLE입니다. `ps` cumulative time/RSS는 자기 process 표본으로 확인합니다. WebKit renderer/helper의 정확한 app 귀속과 GPU·energy·wakeups는 후속 도구 검증이 필요합니다.

E5b는 승인 후 warm-up 3회 제외와 A/B 각 30회 AB/BA 교차 latency를 실행합니다. E5c는 idle/foreground/hidden/minimized/active long60별 15분×3회 및 귀속 검증된 resource 계측입니다. E5d는 누락·실패·신뢰 한계와 동등 조건을 점검해 의사결정 자료를 만듭니다. 계측 on/off, 동기 stderr 및 기존 polling/log의 오버헤드도 같은 artifact에서 교차 측정해야 합니다. E5a는 오버헤드 수치를 추정하지 않습니다. 실제 renderer crash NOT_RUN은 그대로 남습니다. Runtime 선택·ADR ACCEPTED·다음 단계 실행·main 병합은 별도 승인입니다.
