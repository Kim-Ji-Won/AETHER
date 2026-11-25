# Gate 분리 실행 결과 · Issue 44

2026-11-15 · **HARNESS_PREFLIGHT PASS / INDEPENDENT_E5C PARTIAL**.
[승인된 새 계획](e5-gate-split-plan.md), [이전 Baseline BLOCKED](e5-independent-preflight-results.md), [ADR001 PROPOSED](../decisions/001-foreground-runtime-proposal.md).

## Source와 증거

| 대상 | 값 |
|---|---|
| base main | `f44a6c07cec2940633c63e67613dd1259549b1f7` |
| 검토 기준 PR HEAD | `f9e07c66c0dc4e141da309ea6096a1d5d44d437e` |
| 실제 Harness source | `ca4bcd90dc2b4d4428420bcb91c8e33c4aa70b96` |
| Preflight Run ID | `214a2eb9497d46288e467bbe69cdd452` |
| Preflight plan SHA-256 | `cf485340efac671352deec9de295b09a9239d8e33ae9fe00ee062afdae604c10` |
| E5c Run ID | `ddc87a1b09974585986c0ade5dd50a64` |
| E5c plan SHA-256 | `6f0d26e8dfe01dfac65ade233278f47d2447448c0334abb09fe8824348903e09` |
| frozen app build source | `4b021229f8f9b77115da09d71e6963c7112bef62` |
| bundle manifest SHA-256 | `9aead5e26afaa248b009604b9b202f58972d66e65ade8d4f28c0a112cfc3a8b9` |
| 새 실행 증거48개 manifest SHA-256 | `377cb7b12d75cff6aa536f1fbdbb6de6d099a60ca568834d2f91d8daaa95a677` |

출력은 독립 worktree의 `target/gate-split-44/`이며 이전 출력은 재사용하지 않았다. 최종 보고 commit은 위 실제 실행 source와 구분한다. 개인 경로·원본 로그·자격 증명은 공개하지 않는다.

## Gate 변경 및 회귀

`recovery_preflight_policy.py`가 PostgreSQL native identity/실제 executable/조회 전후 start identity와 local DB 다른 연결·트랜잭션0, Docker local socket context·running container0을 읽기 전용으로 확인한다. Homebrew symlink와 실제 executable 경로 차이는 source 봉인 전 정규화하고 회귀 테스트했다. 활성 사용자 작업이나 identity 불명은 허용하지 않는다.

`independent_e5c.py`는 Preflight의 GPU/WindowServer drift를 관찰값으로 기록하고 functional verdict와 분리한다. E5c는 별도 Baseline과 기존5pp/10pp Gate를 유지한다. source/artifact/seal·동시 Collector·외부 작업·AC·기존 deadline·terminal/reap 조건은 유지했다. 앱·Rust·Swift·UI·IPC·CSP·의존성·lockfile·CI 변경 및 재빌드 없음.

`PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -v`: **46 tests PASS**. 동일 drift의 Preflight 경고/E5c 차단, 외부 Node 차단, 활성 PG 연결·Docker container 차단, 원격 endpoint 거부, PG identity/경로 정규화, 기존 PID reuse/소멸·seal/receipt/sample·phase 검증을 포함한다. 이 unit 결과와 아래 실제 앱 검증은 별도이다.

## 환경과 Preflight 6회

macOS26.6 build25G5043d, arm64, AC. 사용자 ChatGPT·Codex 창을 유지하고 강제 종료·숨김을 요구하지 않았다. 밝기/화면 고정은 사용자 조건이며 밝기 수치·창 pixel geometry 독립 계측은 NOT_COLLECTED이다. P2 채팅은 inactive였고 실제 외부 Build/Server·다른 AETHER0을 확인했다. GUI는 Codex `cua_repl` native AX 자동화이며 사람 직접 조작과 구분한다.

Preflight 동안 PostgreSQL·Docker는 실행 상태를 유지했다. 다른 client/active transaction0 및 running container0이 확인된 상태이며 case 중에도 유휴 service gate를 확인했다. 관측 사이의 모든 외부 활동 부재를 보장하지 않는다.

| 환경 관찰 | 앞30초 median | 뒤30초 median | drift | 결과 |
|---|---:|---:|---:|---|
| GPU | 23.5% | 20.0% | 3.5pp | STABLE_OBSERVED |
| WindowServer CPU | 30.6% | 29.7% | 0.9pp | STABLE_OBSERVED |

60초 정착+60초/5초 간격12표본을 한 번 기록했다. safety_status PASS와 environment_observation을 분리했다. 실제 이번 drift가 작았다는 사실을 새 정책의 drift 초과 통합 실행 증거로 표현하지 않는다. drift 경고 경로는 unit test로 확인했다.

| index / case | GUI·Rust 실제 결과 | Host Quit | Child | Functional |
|---|---|---|---|---|
| 0 A normal | run1, seq5, FAKE_OK, terminal1 | exit0 | 해당 없음 | PASS |
| 1 A pause25 | run1, seq2/25% 확인 → 취소 seq3, terminal1 | exit0 | 해당 없음 | PASS |
| 2 B normal | run1, seq5, FAKE_OK, terminal1 | exit0 | Node exit0, reaped | PASS |
| 3 B pause25 | run1, 25% → cancelled seq3, terminal1 | exit0 | 중단·reaped | PASS |
| 4 B 등록 지연 | stderr child_spawn 수신 후 registry 삽입 보류, 살아 있는 자기 Node를 snapshot에서 발견 → 취소 | exit0 | identity·reap 일치 | PASS |
| 5 B 종료 경계 | 실제 Child snapshot 보관 → 취소/reap → 같은 snapshot의 소멸 판별; Host snapshot 보관 → GUI Quit → 소멸/exit0 대조 | exit0 | CHILD_DISAPPEARED | PASS |

6개의 서로 다른 app instance를 확인했다. case4는 `registered_before:false`, `stderr_registration_used:false`와 CHILD_DISCOVERED audit가 남았고, case5는 CHILD_DISAPPEARED/HOST_DISAPPEARED 및 exit0 증거를 남겼다. 앱 바이너리·실제 Runtime 동작에 지연을 추가하지 않고 Harness 관찰 순서만 제어했다. 계획6/시도6/PASS6/실패0/보충0이다. E5b latency 통계에 포함하지 않는다.

## E5c · Baseline과 분모

Preflight 증거·같은 source/artifact·자기 PID 소멸 및 Collector0을 확인한 뒤 PostgreSQL/Docker를 정상 중지했다. 외부 Build/Server/AETHER/중지 서비스0, booted Simulator0이었다.

| 독립 E5c Baseline | 앞30초 median | 뒤30초 median | drift | 사전 기준 | 결과 |
|---|---:|---:|---:|---:|---|
| GPU | 20.0% | 20.0% | 0.0pp | ≤5pp | PASS |
| WindowServer CPU | 30.05% | 30.8% | 0.75pp | ≤10pp | PASS |

Baseline은 한 번만 실행했다. A/B 각3session, AB/BA/AB 계획 중 첫 A session만 시도했다.

| 조건 | 계획 | 시도 | 유효 전체 session | TIMEOUT | NOT_RUN |
|---|---:|---:|---:|---:|---:|
| A | 3 | 1 | 0 | 1 | 2 |
| B | 3 | 0 | 0 | 0 | 3 |
| 합계 | 6 | 1 | 0 | 1 | 5 |

**첫 A session의 전체400초 deadline 초과**로 TIMEOUT latch를 기록하고 중단했다. 이는 GUI 조작·도구 왕복·전환 대기를 포함한 Harness session timeout이며 Runtime 내부 timeout이 아니다. 사후 deadline 확대·재시도·session 교체는 하지 않았다. raw result는 BLOCKED/TIMEOUT, 집계는 PARTIAL/유효0이다. Preflight PASS는 유지한다.

완료한8phase 합계는374.686초였다. Idle42.322초, Foreground91.244초, Recovery32.177초, Hidden101.222초, Recovery31.640초, Cancel11.707초, Recovery40.032초, Reload24.342초다. 원래 최소 관찰시간 외 GUI/도구 운영 시간이 각 phase에 포함됐다. 마지막 Recovery 중 deadline에 도달해 정상 Quit phase는 NOT_RUN, 측정 Host exit는 Harness의 identity 확인 후 SIGTERM에 의한 **-15**이다.

## 실제 Resource / Lifecycle 증거의 제한된 범위

첫 A instance에서 run1 Foreground와 run2 Hidden은 FAKE_OK/seq5/terminal1, run3 취소는 cancelled/seq3/terminal1, run4 Reload는 context1→2와 failed/seq3/terminal1로 GUI와 Rust가 일치했다. Hidden flag true65표본을 실제 확인했다. 정상8phase의 sample/시간/GUI 검사는 통과했으나 마지막 Recovery와 정상 Quit이 없어 전체 session은 PASS가 아니다.

총 Host sample396개. 다음은 **중단 session의 기술 통계**이며 유효 A/B 비교 표본이 아니다. CPU는 같은 Host의 누적 CPU time 차이/실제 sample 경과시간, 한 core 기준이다. RSS는 MiB, WebKit 전체가 아니다. Foreground에는 운영 대기와 완료 후 시간이 포함되며 순수 active60초 CPU로 해석하지 않는다. Hidden은 실제 숨김 시작 후 고정65초 sample 창만 사용한다.

| A 구간 | sample n | 누락 Host | CPU % | RSS min / median / max MiB |
|---|---:|---:|---:|---|
| Idle 전체42.322초 | 42 | 0 | 1.569 | 100.906 / 100.922 / 100.969 |
| Foreground phase91.244초 | 90 | 0 | 1.744 | 100.922 / 101.406 / 101.578 |
| Recovery after normal | 32 | 0 | 1.591 | 101.406 / 101.406 / 101.406 |
| Hidden 고정65초 | 65 | 0 | 0.186 | 109.750 / 109.750 / 109.844 |
| Recovery after hidden | 32 | 0 | 0.414 | 109.750 / 109.750 / 109.750 |
| Pause25 Cancel | 11 | 0 | 1.085 | 109.750 / 109.766 / 109.797 |
| Recovery after cancel | 40 | 0 | 0.329 | 109.797 / 109.797 / 109.797 |
| Pause25 Reload | 24 | 0 | 0.946 | 109.797 / 109.797 / 110.625 |

미완료 Recovery after Reload는11sample로 전체 phase 성공 판정에서 제외한다. RSS 증가를 leak로 확정하거나 leak 부재로 해석하지 않는다. B CPU/RSS·세session 반복 경향·E5c Child Resource Recovery는 NOT_RUN이다. Preflight의 B reap 성공으로 Resource 검증을 대신하지 않는다. combined bundle122070398bytes/Node executable111534432bytes는 유지되며 별도 A-only/B-only bundle 비교가 아니다.

## 중단 뒤 observer 재실행과 정리

400초 timeout 직후 모델이 GUI 상태를 조회했을 때 CUA가 종료된 앱을 새 instance로 자동 재실행했다. 측정 instance와 달라진 것을 즉시 확인하고 receipt/새 sample로 사용하지 않았다. read-only로 새 PID·start identity·고정 executable을 확인한 뒤 이 작업에서 다시 열린 앱만 Cmd+Q로 정상 종료했다. 측정 Host와 observer 앱 모두 소멸, 다른 AETHER0을 확인했다. 이 observer 동작을 새로운 trial이나 정상 Quit 성공으로 섞지 않는다.

## 복원·보존·제출

PostgreSQL stop/start exit0. 초기 readiness exit2 이후 확인된 LaunchAgent 정상 kickstart exit0, SQL SELECT1 exit0, RunAtLoad/KeepAlive true 복원. Docker stop/start exit0, 최종 running 및 engine29.9.3·running container0 확인. 데이터/WAL/config/Volume 삭제·변경 및 다른 사용자 process 종료 없음. ChatGPT·Codex는 유지했다.

최종 Collector 후보0, 다른 AETHER0, 외부 Build/Server0. 실측은 종료됐으며 다른 프로젝트 개발을 재개해도 된다. 원본289개·기존 recovery64개·이전 Preflight14개 및 각 latch/seal, fixed bundle, app source 입력34개는 보존한다. 기존 #38/#35/#42와 다른 worktree·앱·DMG는 건드리지 않았다.

제출 검사는 Python46 tests, 보존된 trace136개, governance, diff/staged diff/git show whitespace이다. 새 Rust/Node 실행·앱 rebuild는 NOT_RUN이며 production 소스 불변이다. 후속 Resource 실행이 필요하면 운영 대기와 관찰 구간을 구분하는 자동화/사전 deadline 검토가 먼저 필요하다. 이번400초 실패를 바꾸지 않는다.

ADR001 PROPOSED 재평가: 지정된 Harness 안전성6사례는 새 실증이 확보됐다. A의 일부 Lifecycle 관찰은 추가됐지만 B Resource와 정상 E5c session이 없어 CPU/RSS 우열·전체 수명주기 완료·Runtime 선정의 근거로 충분하지 않다. Foreground 한정 조건부 개발 후보와 전체 Runtime 비교 완료를 구분한다.

HARNESS_PREFLIGHT_FUNCTIONAL: PASS
PREFLIGHT_ENVIRONMENT_OBSERVATION: STABLE_OBSERVED
INDEPENDENT_E5C: PARTIAL
E5C_VALID_SESSIONS: 0/6
E5B_ORIGINAL_PARTIAL_PRESERVED: YES
E5D_ADR: PROPOSED
RUNTIME_SELECTED: NO
ARCHITECTURE_ACCEPTED: NO
MODEL_CALL_EXECUTED: NO
MAIN_MERGED: NO
