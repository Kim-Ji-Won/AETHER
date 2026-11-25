# 독립 Preflight / E5c 실행 결과 · Issue 44

2026-11-15 · **BLOCKED / 앱 Trial 시작 전 중단**. [사전 등록 계획](e5-independent-preflight-plan.md), [이전 30/144 결과](e5-recovery-results.md), [ADR 001 PROPOSED](../decisions/001-foreground-runtime-proposal.md)를 함께 읽는다.

## 기준점과 source / artifact

| 대상 | 값 |
|---|---|
| 기준 main | `f44a6c07cec2940633c63e67613dd1259549b1f7` |
| 작업 시작 PR #45 HEAD | `1f0c39efd373c0c47781fedf3d82ba3e4567793b` |
| 검증·봉인한 Harness source | `744ff159719ba92281975b801d6b07242a0ee7d1` |
| HARNESS_PREFLIGHT Run ID | `ecc330389f844ff6af08f2a73be18b85` |
| Preflight plan SHA-256 | `beadd5a4943fd82533f67e69e50356aaaaa7e950ceab1a19eb14376a3388376b` |
| 독립 E5c Run ID | `f0fe387e1389493983971bd2fbf91c8b` |
| E5c plan SHA-256 | `d2db9ff334e06e6dd4450c1271bf11e3fd7ba1eac4640fa6cee5e60b9d694c50` |
| fixed app build source | `4b021229f8f9b77115da09d71e6963c7112bef62` |
| bundle manifest SHA-256 | `9aead5e26afaa248b009604b9b202f58972d66e65ade8d4f28c0a112cfc3a8b9` |
| executable SHA-256 | `7d7cfa83d2cbb88d9b019319120c4b895c45427c7b1ae9c0778ce92891a99d64` |
| 새 증거 manifest SHA-256 | `8d6d3df7a7feaaaf7bb28289e20df4057195ae6abf7be4c3977291530e516143` |

출력은 독립 worktree의 `target/preflight-e5c-44/`이다. manifest는 새 plan/seal/baseline/stop/summary/services 기록 14개를 해시로 고정한다. 공개 문서에는 개인 절대 경로·전체 환경·원본 로그를 싣지 않는다. 최종 보고 commit은 위 실행 source 뒤의 문서 갱신이며 실측 source로 혼동하지 않는다.

## Harness 변경과 검증

- `recovery_process.py`: native process start microseconds, PID/PPID/executable 검증, snapshot 이후 정상 소멸과 PID 재사용·외부 Node 구분. 소멸 관찰만으로 PASS를 만들지 않는다.
- `recovery_latency.py`, `recovery_resource.py`: 공통 identity 검증을 사용한다. 기존 E5b plan/latch 및 legacy E5c gate는 유지한다.
- `independent_e5c.py`: 고정 Preflight 6회와 별도 E5c 6 session plan, 동일 source/artifact 및 Preflight PASS 증거 gate, collector lock, 개별 latch, phase/GUI/ledger/identity/sample 검증을 구현한다.
- 특별 Preflight는 Harness의 stderr 등록 지연과 snapshot 보관을 사용하도록 구현했다. **실제 앱에서 목표 interleaving을 관찰한 상태가 아니다.**
- 앱/Rust/Swift/UI/IPC/CSP/의존성/lockfile/CI 변경 및 앱 재빌드 없음.

`PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -v`: **38 tests PASS**. normal child/Host 소멸, 미등록 소멸, PID reuse, PPID/executable 변경, 외부 parent, 조회 오류, seal/latch/provenance, 누락 sample, incomplete phase, GUI receipt, Reload terminal 계약을 포함한다. 실제 자기 `/bin/sleep` 프로세스의 identity·정상 exit 후 소멸도 검사했다. 이는 packaged app 검증을 대체하지 않는다.

보존된 instrumented PASS trace **136개**를 읽기 전용으로 재검증했다. 초기 신규 테스트 import 실패는 source 봉인 전 수정했고 위 38개 최종 실행이 통과했다. 준비용 서비스 조회의 원본 worktree 환경변수 누락도 수정했으며 그 실패 시 앱 실행·서비스 중지는 없었다.

## 환경 Gate와 실제 중단

초기 준비 확인은 BATTERY / discharging이어서 실측을 보류했다. 사용자가 AC 연결과 밝기·화면 유지를 확인한 뒤 실제 AC를 재확인했다. macOS 26.6 build 25G5043d, arm64, logical CPU 10. 전원·부하·외부 process를 읽기 전용으로 확인했다. P2 채팅은 idle이며 중단 기록이 유지됐다.

PostgreSQL 지정 데이터 디렉터리 일치, 다른 client/active transaction 0, Docker running container 0, booted Simulator 0, 외부 Build/Server·다른 AETHER 0이었다. Docker CLI는 중지 전 status stopped를 표시했지만 engine 응답과 Desktop process가 존재하므로 실제 실행 상태로 취급하고 정상 stop/start했다. Codex helper와 CoreSimulator 서비스는 종료하지 않았다.

두 서비스 정상 종료 후 60초 정착 + 60초/5초 간격 **12표본, 한 번**의 Baseline을 실행했다.

| 항목 | 앞 30초 median | 뒤 30초 median | drift | 승인 기준 | 판정 |
|---|---:|---:|---:|---:|---|
| GPU Device Utilization | 30.5% | 21.5% | 9.0pp | ≤5pp | BLOCKED |
| WindowServer CPU | 41.1% | 32.7% | 8.4pp | ≤10pp | PASS |
| AC / 외부 Build·Server·AETHER·중지 서비스 | AC / 0 | AC / 0 | — | 유지 | PASS |

`GPU_PERCENT_DRIFT`에 따라 Preflight latch `BASELINE_FAILED`를 기록했다. GPU가 0%가 아니기 때문이 아니며 AETHER 앱은 아직 실행되지 않아 자기 앱 활동을 외부 간섭으로 잘못 분류한 경우도 아니다. 이 관찰만으로 GPU 변화의 특정 앱 원인을 확정하지 않는다. 유리한 Baseline을 얻기 위한 재시도는 하지 않았다.

## 실제 실행 분모

| 고정 Preflight case | 계획 | 시도 | 유효 PASS | 상태 |
|---|---:|---:|---:|---|
| A normal | 1 | 0 | 0 | NOT_RUN |
| A pause25 | 1 | 0 | 0 | NOT_RUN |
| B normal | 1 | 0 | 0 | NOT_RUN |
| B pause25 | 1 | 0 | 0 | NOT_RUN |
| B deferred registration | 1 | 0 | 0 | NOT_RUN |
| B child/Host exit boundary | 1 | 0 | 0 | NOT_RUN |

독립 E5c도 Preflight gate 미충족으로 **BLOCKED**, A 0/3·B 0/3, 총 **0/6 session NOT_RUN**이다. E5c에는 `PREFLIGHT_BASELINE_BLOCKED` latch를 남겼고 E5c Baseline도 실행하지 않았다. Collector summary의 `PARTIAL / completed: 0`은 분모 집계이며 운영 중단 원인은 baseline/latch의 BLOCKED이다. 앱 실패·Runtime timeout 표본으로 기록하지 않는다.

따라서 새 GUI/Rust Ledger/terminal/reap 실증, Host/Node CPU·RSS 비교, phase recovery, 반복 증가 경향은 **NOT_RUN**이다. 특별 race interleaving도 미관찰이다. WebKit 전체 귀속 UNAVAILABLE; GPU Energy, Battery Efficiency, 장시간 leak 부재, 실제 renderer crash NOT_RUN을 유지한다. E5b 새 144회, 추가 latency 표본, ON/OFF 재측정은 하지 않았다.

## 복원·보존·다음 검토

- PostgreSQL stop/start exit0. start 직후 SQL readiness exit2 뒤 확인된 LaunchAgent에 정상 `launchctl kickstart` exit0, SQL `SELECT 1` exit0. `RunAtLoad / KeepAlive: true` 보존.
- Docker stop/start exit0. 최종 Desktop `running`, engine `29.9.3` 응답, running container 0. 중간 starting 상태를 최종 복원 성공으로 사용하지 않았다.
- 측정용 Host/Node는 이번 실행에서 생성하지 않았다. 마지막 process 점검에서 Collector 후보0·다른 AETHER0·Build/Server0. 실측은 종료됐으며 다른 프로젝트 개발 재개 가능.
- #27/#34 원본289개 digest `3494474dafe735a407a3504028dfd96a4a93f9a77e4fbd7a47ed2ce5879cc97a`, 기존 #44 증거64개 manifest `5c8b67daff6ea298c635d3f3445ca450c71435ae1627ff7489a9c0053529536b` 일치. 기존 TIMEOUT, 30/144, stop latch와 고정 bundle/arm64 실행 파일 보존.
- 기존 E5a/E4 앱·DMG, #38/#35/#42 작업은 건드리지 않았다. 원본 E5b worktree clean. DB/WAL/config/Volume 삭제·변경, 다른 사용자 process 종료 없음.
- `python3 scripts/validate_governance.py`, `git diff --check`, staged diff와 `git show --format= --check HEAD`를 제출 시 검사한다. 새 앱 build/Rust/Node 재실행은 NOT_RUN이며 앱 소스 불변이다.

ADR은 새 resource 근거가 없으므로 성능 추천을 강화하지 않는다. A는 foreground-only·Reload 중단을 허용하는 경우 구조적 복잡도에 근거한 조건부 개발 후보일 뿐이다. B 지속성 필요 여부, 자원 비용, 미수집 energy와 안전 회수는 여전히 결정 제약이다.

실행 재개에는 실패한 plan/latch를 수정하는 대신 별도 승인·Run ID/출력과 환경 원인 검토가 필요하다. 이번 Draft PR은 코드·증거·한계 검토를 위해 갱신하며 Ready 전환이나 병합은 하지 않는다.

HARNESS_PREFLIGHT: BLOCKED
INDEPENDENT_E5C: BLOCKED
E5B_ORIGINAL_PARTIAL_PRESERVED: YES
E5D_ADR: PROPOSED
RUNTIME_SELECTED: NO
ARCHITECTURE_ACCEPTED: NO
MODEL_CALL_EXECUTED: NO
MAIN_MERGED: NO
