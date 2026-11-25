# E5 독립 후속 실행 계획 · Issue 44

상태: PROPOSED 결과 대기. [Issue #44](https://github.com/Kim-Ji-Won/AETHER/issues/44).
기준 main `f44a6c07cec2940633c63e67613dd1259549b1f7`.
2026-11-15 사용자가 승인한 운영 기준을 이번 독립 실행에만 적용한다. Runtime 결정과 ADR ACCEPTED는 승인하지 않았다.

## 불변 대상과 source 분리

- 기존 #27 / Draft #34는 PARTIAL 그대로 보존한다. 144회 원본 plan/trials/summary 289개를 수정하지 않는다.
- 원본 증거 상대 경로→SHA256 map의 canonical JSON digest는 `3494474dafe735a407a3504028dfd96a4a93f9a77e4fbd7a47ed2ce5879cc97a`이다.
- 측정 앱 build source `4b021229f8f9b77115da09d71e6963c7112bef62`, bundle manifest `9aead5e26afaa248b009604b9b202f58972d66e65ade8d4f28c0a112cfc3a8b9`를 고정한다. 최신 UI 앱으로 교체하거나 재빌드하지 않는다.
- 기존 최종 Harness/report HEAD는 `b70fe0bd36538c6df7f44bfe07e7cbce3b30ceff`이다. 당시 측정 source와 다르다.
- 새 Harness는 별도 worktree의 `scripts/recovery_latency.py`, 출력은 `target/recovery-44/`이다. `AETHER_E5B_ORIGINAL_ROOT`는 원본 worktree를 읽는 용도로만 쓴다.
- 커밋 후 `prepare`가 실제 Harness HEAD, 새 experiment/trial UUID, 원본 증거 map, 앱 manifest, E5c 계획을 `plan.json`에 기록하고 `plan.sha256`으로 seal한다. 기존 plan이 있으면 덮어쓰지 않는다.

## 고정 E5b 설계

A/B × normal/pause25 각 warm-up 3회(12), 본 측정 30회(120), A/B ON/OFF 각 3회(12): 총 144회. round별 AB/BA 교차, 보충·교체 없음. 실패·TIMEOUT·제외와 전체 시도 분모를 남긴다. E5a SMOKE_ONLY 및 기존 E5b와 합치지 않는다.

Rust monotonic clock 안에서 accept→first progress, accept→terminal, cancel processing, Node spawn→ready, cancel confirmed→reap를 계산한다. 새 프로세스 spawn→session ready는 Harness clock의 외부 수신 시각으로 계산하며 cold-cache launch가 아니다. ON/OFF는 동일 diagnostic start→terminal의 외부 수신 간격만 대조한다. GUI 대기와 Runtime 내부 시간은 구분한다. nearest-rank `ceil(p*n)`과 ms, min/max/p50/p95/n을 사용한다.

각 trial은 새 프로세스, 고정 900×760 foreground 창에서 같은 GUI 절차를 사용한다. 실제 instance/variant/fixture를 확인하고 실행 버튼, pause25는 25에서 취소, terminal을 확인해 GUI receipt를 기록한다. Cmd+Q 후 PID 종료·collector 완료를 확인한 다음 다음 trial을 시작한다. Quit 후 AX 재조회로 앱이 다시 실행되는 경로를 사용하지 않는다. 수집기 lock으로 동시 실행을 막는다. 90초는 operator를 포함한 전체 timeout이며 Runtime 내부 지연으로 해석하지 않는다.

## 환경 운영 기준 · 사전 등록

준비·테스트·커밋·seal 동안 PostgreSQL과 Docker를 유지한다. 실측 직전에 PostgreSQL identity와 다른 client/active transaction 0, Docker running container 0을 확인한다. `brew services stop postgresql@16`, `docker desktop stop --timeout 30`만 사용한다. 데이터·Volume은 보존한다. 시작 상태와 직접 중지한 범위를 별도 로컬 기록에 남긴다.

화면 배치를 고정하고 60초 정착, 이어 60초 동안 5초 간격 12표본을 수집한다. 앞 6개/뒤 6개의 GPU 중앙값 차이는 ≤5 percentage points, WindowServer CPU 중앙값 차이는 ≤10 percentage points이다. 절대 GPU cutoff는 없다. 이는 이번 실험의 drift 경보이며 보편적 표준이 아니다. `ioreg` Device Utilization과 `ps %cpu`는 간접·최근 관찰치이며 GPU energy나 정확한 원인 귀속이 아니다.

AC, 동일 밝기·창 크기를 유지한다. 외부 build/server/3D/heavy work는 허용하지 않는다. Codex의 지정 runtime helper는 개발 서버와 구분한다. CoreSimulator 서비스는 종료하지 않되 booted Simulator와 실제 개발 작업이 없는지 실행 전 점검한다. 자기 Host/Node는 PID·시작 시각·실행 파일 identity를 검증한 경우에만 외부 간섭에서 제외한다. 자기 AETHER 활동에 따른 GPU 증가는 외부 간섭으로 취급하지 않는다.

Baseline은 한 번만 예약·실행한다. 실패·중단한 결과를 지우고 유리해질 때까지 반복하지 않는다. trial 중 AC·외부 프로세스 및 환경 변화를 확인한다. 실제 Runtime active 구간에 idle GPU drift 기준을 적용하지 않는다. GUI 도구와 관측 자체의 부하는 양 후보에 공통이며 완전히 제거되지 않는다. 확인된 환경 변화·오류가 발생하면 stop latch를 기록하고 남은 trial을 자동 시작하지 않는다.

## E5c · MINIMUM_SCOPE 사전 등록

기계 판독 계획: `scripts/recovery-resource-plan.json`. E5b 종료 및 자기 프로세스 정리 후 같은 drift 기준의 별도 Baseline을 한 번 실행한다. E5b가 중단되면 E5c도 BLOCKED로 남긴다.

A/B 각각 3개 session, AB/BA/AB 순서. 각 session에서 idle 30초 → foreground long60 관찰 65초 → recovery 15초 → hidden long60 관찰 65초 → recovery 15초 → pause25 취소(최대 30초) → recovery 15초 → pause25 Reload(최대 30초) → recovery 15초 → 정상 Quit/reap(최대 10초). 총 session deadline 400초이며 GUI 전환 시간이 포함된다. 각 전환의 실제 시각과 UI/원장 결과를 별도 receipt에 기록한다. 1초 간격 Host/Node 누적 CPU time과 RSS, identity·child 생성/종료를 수집한다. CPU는 누적 time 차이/실제 경과 시간의 한 core 기준이며 RSS는 bytes이다.

A Reload는 진행 작업의 안전 중단, B Reload는 고정 fake 작업의 Host/child 유지·재연결 후 취소를 확인한다. 실제 renderer crash와 다르다. baseline/recovery RSS 차이와 세 반복의 경향만 제시하며 장기 leak 부재를 주장하지 않는다. 단일 combined 앱 bytes와 Node 기여 bytes를 기록하며 독립 A-only/B-only 번들 크기라고 표현하지 않는다.

전체 계획의 15분×조건당3회가 아니다. WebKit 전체 CPU/RSS 귀속 UNAVAILABLE, GPU 비용·energy·배터리 지속시간·장시간 상주·실제 renderer crash NOT_RUN이다.

## 중단·복원·산출물

자기 process는 identity 확인 후 정상 Quit가 기본이다. 오류 시 검증된 자기 Host에만 SIGTERM을 보내며 SIGKILL로 통과시키지 않는다. 잔존하면 BLOCKED와 identity를 기록한다. 타 프로젝트·시스템·Codex 프로세스는 종료하지 않는다.

완료/중단 후 직접 중지한 PostgreSQL과 Docker를 `brew services start postgresql@16`, `docker desktop start`로 복원하고 SQL readiness, LaunchAgent 등록, Docker status를 확인한다. 자동 등록은 되었지만 시작되지 않으면 확인된 해당 PostgreSQL LaunchAgent에만 정상 `launchctl kickstart`를 사용한다. force/prune/reset은 사용하지 않는다.

결과는 별도 보고와 Runtime ADR PROPOSED에 연결한다. 사용자에게 관찰·추정·NOT_RUN을 구분하고 Draft PR에서 멈춘다. Runtime 선정, 모델 호출, 보안 경계 변경, main 병합은 수행하지 않는다.
