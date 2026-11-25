# Preflight / E5c Gate 분리 · GATE_SPLIT_V2

2026-11-15 · [Issue #44](https://github.com/Kim-Ji-Won/AETHER/issues/44). 사용자가 승인한 새 정책이다. [이전 Preflight 결과](e5-independent-preflight-results.md)의 실패 Baseline, plan/seal/latch와 원본 #27/#34, #44 30/144는 변경하지 않는다.

## 적용 범위와 새 실행

`scripts/independent_e5c.py`의 새 출력은 `target/gate-split-44/`이다. clean commit에서 새로운 Preflight/E5c UUID와 plan seal을 만든다. 같은 source SHA·기존 고정 앱 bundle hash를 사용한다. 원본289개, recovery64개, 이전 Preflight14개 증거를 해시로 확인한다. 기존 출력 경로로 돌아가 재실행하거나 이전 latch를 삭제하지 않는다.

앱·Runtime·Swift·UI·IPC·CSP·의존성은 수정하지 않는다. ChatGPT와 Codex 창은 유지하며 사용자 화면 조건을 고정한다. 화면 밝기 수치를 읽지 못하면 사용자 유지 조건과 NOT_COLLECTED를 구분한다.

## HARNESS_PREFLIGHT · 기능·안전성

Source/Bundle/Plan Seal, Host/Node PID·PPID·executable·start identity, 외부 Build/Test/Server 없음, 동시 Collector·잔존 AETHER 없음, AC, 기존 과부하 경보 및180초 deadline을 필수로 유지한다. Process 소멸 관찰은 Host exit0·Rust terminal1·Child Reap·GUI receipt와 함께 확인해야 성공이다.

60초 정착 +60초/5초 간격12표본 환경 관찰을 한 번 수행한다. GPU/WindowServer drift는 `STABLE_OBSERVED` / `DRIFT_WARNING` / `UNAVAILABLE`로 별도 기록한다. GPU drift5pp·WindowServer drift10pp 초과만으로 기능 시험을 차단하지 않는다. 필수 safety 조회 실패나 환경 간섭은 중단한다. `functional_status`와 `environment_observation`을 별도 필드로 제출한다.

Preflight에서는 PostgreSQL/Docker를 정상 실행 상태로 둘 수 있다. PostgreSQL native identity·예상 local 데이터 디렉터리·다른 client/active transaction0, Docker local socket context·running container0을 읽기 전용으로 확인한다. 활성 연결/작업, 원격 또는 불명확한 endpoint, 조회 실패는 중단한다. case 시작 전과 실행 중 약1초 gate마다 확인하며 관측 사이의 모든 활동 부재를 보장한다고 주장하지 않는다. 서비스 데이터·설정은 변경하지 않는다.

고정6회: A normal, A pause25, B normal, B pause25, B deferred registration, B child/Host exit boundary. 이전 [독립 계획](e5-independent-preflight-plan.md)의 GUI·Ledger·special interleaving 판정은 유지한다. 실제 등록 지연/종료 snapshot을 관찰하지 못하면 PASS가 아니다. 실패·환경 간섭·deadline 초과 시 latch와 분모를 남기고 종료하며 보충하지 않는다. E5b latency 통계에 포함하지 않는다.

## 독립 E5c · MINIMUM_SCOPE

같은 source/artifact의 Preflight `functional_status: PASS`, 완료6/6, 증거 해시, 프로세스 정리가 필수이다. Preflight drift warning 자체를 E5c 승인으로 쓰지 않고 E5c 고유 gate를 다시 검사한다.

PostgreSQL 다른 작업과 Docker 실행 컨테이너0을 재확인한 뒤 승인된 정상 stop/start 정책을 적용한다. 60초 정착 +60초/5초 간격12표본의 Baseline을 한 번 수행하고 GPU drift≤5pp, WindowServer drift≤10pp, AC·고정 화면 조건을 요구한다. 실패하면 E5c만 BLOCKED이며 이미 완료한 Preflight는 변경하지 않는다. 실제 앱 Active 부하에 Idle drift 기준을 적용하지 않는다.

기존 `recovery-resource-plan.json`의 A/B 각3session, AB/BA/AB, Idle30초 / Foreground long60 관찰65초 / Recovery15초 / Hidden long60 관찰65초 / Recovery15초 / Pause25 Cancel≤30초 / Recovery15초 / Pause25 Reload≤30초 / Recovery15초 / Quit·Reap≤10초를 유지한다. session deadline400초, 귀속 가능한 CPU time/RSS는1초 간격이다. 각 phase 실제시간·GUI·Rust trace·Identity·sample 누락·회수를 검증하며 Collector 종료만으로 PASS 처리하지 않는다.

WebKit 전체 귀속 UNAVAILABLE, GPU Energy·Battery·장시간 leak 부재·renderer crash NOT_RUN을 유지한다. 비교 불가능하거나 미완료인 session은 분모에 남기고 대체하지 않는다. 원본 E5b와 통계를 합치지 않는다.

## 종료

직접 변경한 서비스 상태는 성공/중단 모두 복원하고 readiness를 확인한다. 다른 사용자·시스템 프로세스는 종료하지 않는다. 원본과 신규 manifest를 재확인하고 실제 결과만 ADR001 PROPOSED와 Draft PR45에 반영한다. Runtime 선정, ADR ACCEPTED, Provider 요청, P2, Ready 전환·병합은 하지 않는다.
