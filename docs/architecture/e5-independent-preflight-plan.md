# 독립 Harness Preflight와 E5c · Issue 44

2026-11-15 사용자 실행 승인. 기존 [30/144 결과](e5-recovery-results.md), #27/#34 PARTIAL, 기존 plan/stop latch는 불변이다. 이번 실행은 E5b 통계 보충이 아니다. 앱·Runtime·UI·IPC·CSP는 변경하지 않는다.

## Provenance와 실행 게이트

`scripts/independent_e5c.py prepare`는 clean commit의 Harness source SHA, 기존 고정 앱 build source 및 bundle manifest hash, 원본 289개 증거 digest와 기존 #44 manifest hash를 확인한다. `target/preflight-e5c-44/`에 서로 다른 UUID의 Preflight/E5c plan과 SHA-256 seal을 배타적으로 생성한다. 같은 plan 재생성·case 대체·latch 삭제는 허용하지 않는다. 예전 E5b gate는 그대로 두고 새 진입점에서만 독립 E5c를 허용한다.

E5c는 같은 source/artifact의 HARNESS_PREFLIGHT 6/6 PASS와 증거 해시, 자기 Collector/Host/Node 잔존 0, 새 전용 baseline/latch를 요구한다. Collector 파일 lock과 실행 중 Python collector 점검으로 동시 수집을 막는다. GUI receipt는 실제 native AX 관찰에서 작성하며 collector 완료만으로 PASS가 되지 않는다.

기존 앱 bundle manifest는 `9aead5e26afaa248b009604b9b202f58972d66e65ade8d4f28c0a112cfc3a8b9`, build source는 `4b021229f8f9b77115da09d71e6963c7112bef62`이다. source SHA와 실제 Run ID/seal은 실행 결과 문서에 기록한다.

## Process Identity와 정상 소멸

PID·PPID·실행 파일과 `libproc`의 process start microseconds를 대조한다. ABI 출처는 Apple의 [proc_info.h](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/proc_info.h)이다. ps snapshot의 parent/path와 native identity가 다르거나 시작 identity가 바뀌면 중단한다. 조회 권한 오류는 소멸로 간주하지 않는다.

동일 Host가 생성한 검증된 Node는 stderr 등록 전 snapshot에서도 발견할 수 있다. snapshot 후 사라진 기존 자기 child/Host는 `CHILD_DISAPPEARED` / `HOST_DISAPPEARED` 관찰로 분류한다. 성공은 별도의 Rust terminal/reap, GUI receipt, Host exit 0과 잔존 0을 모두 요구한다. 미등록 child 소멸, 재사용된 PID, 외부 parent의 Node는 자기 프로세스로 면제하지 않는다.

## HARNESS_PREFLIGHT · 고정 6회

| index | Runtime / fixture | 필수 증거 |
|---|---|---|
| 0 | A normal | GUI/Rust 동일 run·seq5·FAKE_OK·terminal1, Quit exit0 |
| 1 | A pause25 | 25% 확인 후 취소, seq3·cancelled·terminal1, Quit exit0 |
| 2 | B normal | 위 정상 조건 및 child identity/reap |
| 3 | B pause25 | 위 취소 조건 및 child identity/reap |
| 4 | B pause25 / deferred_registration | stderr child_spawn은 수신하되 registry 삽입을 보류, snapshot이 실제 살아 있는 child를 발견한 audit를 확인한 뒤 취소/Quit |
| 5 | B pause25 / exit_boundary | 살아 있는 child snapshot을 보관 → 취소/reap → 같은 snapshot과 실제 소멸 대조. 살아 있는 Host snapshot 보관 → GUI Quit → 같은 snapshot과 exit0/소멸 대조 |

특수 case는 Harness의 관찰 순서만 제어한다. 앱·Runtime에 지연이나 위조 event를 넣지 않는다. 실제 interleaving 미관찰, identity 불명, 환경 간섭, GUI/ledger 불일치, 180초 case deadline 초과 시 latch를 기록하고 이후 case를 중단한다. 어떤 Preflight 값도 latency 통계에 포함하지 않는다.

## 독립 E5c · MINIMUM_SCOPE

기존 `scripts/recovery-resource-plan.json`의 6 session AB/BA/AB 및 phase 시간을 재사용한다. 새 sealed plan에서 gate만 독립 Preflight로 명시한다. 각 session은 새로운 앱 process이며 400초 deadline에 GUI 전환 시간이 포함된다.

Idle 30초 → Foreground long60 관찰 65초 → Recovery 15초 → Hidden long60 관찰 65초 → Recovery 15초 → Pause25 Cancel 최대30초 → Recovery 15초 → Pause25 Reload 최대30초 → Recovery 15초 → 정상 Quit/reap 최대10초.

1초 간격 Host/Node CPU time·RSS를 native identity 전후 확인으로 수집한다. phase 시작/종료 monotonic time, GUI receipt, run별 Rust trace, child PID/start identity, 누락·간격을 보존한다. CPU는 동일 프로세스 누적 time 차이/실제 경과 시간의 한 core 기준이다. RSS는 bytes이며 관측된 child 종료를 RSS 0으로 대체하지 않는다.

Hidden은 GUI로 숨긴 뒤 read-only LaunchServices hidden flag의 실제 첫 true부터 65초 창을 기록한다. setup/restore 시간을 hidden CPU/RSS 분석 창과 구분한다. 이후 창을 복원해 terminal UI를 읽는다. long60는 새 run이고 실제 fixture/완료가 일치해야 한다. A Reload는 context 증가와 failed terminal1, B Reload는 같은 작업 유지 후 취소/reap를 검증한다. recovery 중 child 잔존·샘플 누락·시간 부족은 실패한다. 유효 session/실패/누락 분모를 유지하며 추가 session을 만들지 않는다.

combined app 총 bytes와 Node 실행 파일 bytes만 비교한다. A-only/B-only 별도 번들 크기는 측정하지 않는다. WebKit 전체 CPU/RSS는 UNAVAILABLE, GPU Energy·Battery Efficiency·장시간 leak 부재·실제 renderer crash는 NOT_RUN이다.

## 환경과 중단·복원

AC·동일 밝기·창 크기/화면 배치, 다른 프로젝트 Build/Test/Server 및 P2 실행 없음이 필요하다. 실제 자기 앱의 활동 부하는 외부 간섭으로 분류하지 않는다. 서비스 정상 중지 전에 PostgreSQL active client/transaction 0과 Docker running container 0을 확인한다. 타 사용자 앱은 종료하지 않는다.

Preflight와 E5c 각각 60초 정착 후 60초 동안 5초 간격 baseline을 한 번 실행한다. 앞/뒤30초 GPU median drift ≤5pp, WindowServer CPU median drift ≤10pp이다. 절대 GPU cutoff는 없다. AC/부하/외부 프로세스는 case 중에도 확인한다. baseline 실패를 유리해질 때까지 반복하지 않는다.

정상 Quit가 기본이다. 실패 시 native identity가 일치하는 자기 Host에만 SIGTERM을 보내며 force kill로 통과시키지 않는다. 중단/완료 후 직접 중지한 PostgreSQL/Docker를 정상 복원하고 SQL readiness·자동 시작 등록·Docker engine을 확인한다. 데이터·Volume은 보존한다.

결과는 별도 실행 기록과 ADR 001 PROPOSED 재평가로 제출한다. E5b PARTIAL을 PASS로 바꾸지 않으며 Runtime 선정·ADR ACCEPTED·Provider/P2·PR 병합은 하지 않는다.
