# G3 · 오프라인 철회 상태 전이와 crash fixture

**오프라인 준비 단위 · G3 회귀 25/25 PASS.** [Issue #68](https://github.com/Kim-Ji-Won/AETHER/issues/68)의 범위는 I/O 없는 철회 protocol과 메모리 journal fixture다. 실제 파일의 영속성, macOS owner 확인 또는 Keychain 삭제를 구현·입증하는 단계가 아니다. G2 OS 시험은 **NOT_RUN**, 제품 G3는 **NOT_IMPLEMENTED**, 실제 Provider 요청은 **BLOCKED**다.

기준 `main`은 `daeadb5110dd78f6d6abd677887fbc52bf0e5671`이다. [Gate 0 §6·§11–§14](p2-2b-3-live-integration-preflight.md), [G2 서명·시험 계획](p2-2b-3-g2-credential-signing-preflight.md), [b-1 Credential·동의](p2-2b-1-native-credential-consent.md), [G1 단일 Host](p2-2b-3-g1-offline-single-host.md)의 경계를 따른다. Gate 0 §12는 G2 실제 접근을 기다리는 동안 G3 메모리 crash fixture를 준비할 수 있도록 한다.

## 범위와 소유권

| 이번 단위 | 후속 단위 |
|---|---|
| 명시적 입력·결과로 진행하는 순수 Rust 상태 전이 | 앱의 단일 native owner와 실제 비동기 저장소 연결 |
| 미확인 상태에서 admission을 닫는 protocol | 프로세스 간 owner lock, 파일 접근통제·손상·삭제·rollback 탐지 |
| commit 지연·실패·응답 유실을 제어하는 메모리 fixture | 실제 durable commit과 OS crash·전원 장애 검증 |
| 기존 Host·메모리 adapter의 취소·cleanup을 이용한 회귀 | 서명된 macOS 앱과 정확한 G2 Keychain 항목의 대조 |
| 비밀 없는 slot·generation·operation의 일치 검사 | 실제 등록·회전·reconciliation UI와 secure entry |

Protocol과 회귀 전체는 `#[cfg(test)]` 아래에 있다. 파일, Keychain, 네트워크 또는 renderer IPC를 직접 호출하지 않는다. 필요한 연산과 결과를 구분하고, fixture가 외부 효과와 commit의 순서를 관찰한다. 제품 resolver·dispatch에는 이 fixture를 연결하지 않는다. Production dispatch는 `ProviderDisabled`를 유지한다.

시험의 `RunFixture`는 기존 library `OfflineProvider`가 소유한 Host 하나와 그 안의 공용 Provider adapter를 재사용한다. 실제 b-1 승인 발행·소비·폐기 및 mock resolver의 resolve 횟수도 검사한다. 삭제 의도 장벽이 없는 기존 `revoke_credential()`은 호출하지 않는다. 제품 앱의 `Runtime.host` 연결을 변경하거나 두 번째 Host를 등록하지 않는다.

Host는 run의 admission·identity·deadline·terminal·cleanup을 계속 소유한다. Journal 처리 결과가 Host의 terminal을 새로 만들거나 두 cleanup owner의 확인을 대신할 수 없다. 기존 메모리 철회 상태의 성공은 durable journal의 성공 증거가 아니다.

## Journal 모델

기록 모델은 schema version, 불투명 credential slot, generation, operation 식별자, 철회 단계와 필요한 cleanup 상태로 한정한다. 이는 저장 형식이나 파일 위치를 확정하는 결정이 아니다.

- Credential 값·그 지문, 승인 토큰, 요청/응답 본문·digest, Authorization header는 기록하지 않는다.
- 메모리 fixture는 제출된 기록과 commit된 기록을 구분한다. 제출 또는 함수 반환만으로 commit 성공을 만들지 않는다.
- Crash fixture는 volatile 상태를 버리고 commit된 기록만 새 instance에 전달한다. 이 동작은 메모리 모델의 재시작이다.
- 결과는 현재 연산과 요청된 단계에 결속한다. 다른 instance·operation·slot·generation 또는 과거 단계의 결과가 잠금을 해제하거나 다음 효과를 발행하면 안 된다.
- 누락·손상·지원하지 않는 기록, 신뢰할 수 없는 세대 연속성은 신규 초기화의 근거가 아니다. 확인 불가 상태로 유지한다.

`fsync`, 원자적 교체, 저장 매체의 쓰기 순서, 실제 접근권한, rollback 방어와 crash 이후 OS 자원 소멸은 이 메모리 모델로 입증하지 않는다.

## 철회 순서

### 1. 즉시 차단과 취소

철회 요청을 받으면 같은 직렬화 경계에서 신규 admission을 잠그고 미소비 동의를 폐기한다. 활성 run의 취소·로컬 abort를 요청한 뒤 cleanup을 진행한다. **최초 journal 쓰기 완료를 기다리지 않는다.** 저장 중에도 Host clock·deadline·abort 처리는 계속되어야 한다.

영속 접수 성공은 철회 의도의 commit 확인 후에만 가능하다. 최초 쓰기가 실패해도 잠금을 유지하고 운영 오류를 보고하며, 이미 요청한 취소·abort·cleanup을 계속한다. 기록 실패를 이유로 이전 승인이나 활성 입력 경로를 복구하지 않는다.

### 2. Cleanup과 삭제 의도

삭제에는 다음 두 조건이 모두 필요하다.

1. Adapter와 Runtime 양쪽 owner의 자원 해제 확인 또는 실제 자원 미생성에 해당하는 `NotRequired`.
2. 해당 slot의 삭제 시도 의도에 대한 commit 확인.

한쪽 ACK, cleanup deadline 만료, terminal 확정, journal commit 중 어느 하나도 나머지 증거를 대신하지 않는다. Queued 상태에서 자원을 만들지 않은 사실과 dispatch 뒤 자원 종료 확인을 구분한다.

조건이 갖춰진 뒤 정확한 slot에 대해서만 삭제 효과를 발행한다. 같은 요청을 반복해도 추가 삭제를 자동 발행하지 않는다. 취소 전까지 수락한 partial output과 terminal count 1은 이후 journal 처리와 독립적으로 보존한다.

### 3. 삭제 결과와 확인 기록

삭제 실패, 응답 유실, 삭제 성공 뒤 확인 기록의 실패 또는 crash는 미확인 잠금으로 남긴다. 삭제 함수의 성공 응답만으로 재등록을 허용하지 않는다. 확인 상태의 commit까지 완료되어야 하며, 실제 OS 항목의 부재 증거는 G2 계약을 따른다.

자동 삭제 재시도, 자동 재등록·회전, 다른 credential 선택, 앱 재시작으로 실패 잠금을 해제하지 않는다. 사용자가 다시 요청했다는 사실만으로 불명확한 이전 삭제를 반복하지 않는다.

## 재시작과 명시적 reconciliation

새 instance는 항상 잠긴 상태에서 시작한다. 기록이 없거나 과거 확인 완료 기록이 있어도, 새 명시적 reconciliation 전에는 admission을 허용하지 않는다. 과거 consent·lease·ACK는 새 instance의 권한이 아니다.

| 복구 입력 | 허용 판단 |
|---|---|
| Journal 누락 또는 신뢰할 세대 연속성 없음 | 초기 정상 상태로 취급하지 않고 격리 |
| 미완료 cleanup 기록 | 이전 owner와 자원 상태의 확인이 필요. 재시작만으로 성공 ACK 생성 금지 |
| 삭제 의도만 남음·결과 불명 | 자동 삭제 재시도 없이 명시적 조정 대기 |
| 과거 확인 완료 기록 | 새 instance의 reconciliation과 현재 owner·slot 증거를 요구 |
| 잠김·권한 거부·다른 서명의 not-found | 원 owner의 항목 부재로 해석하지 않음 |
| 같은 owner·slot의 항목 잔존 | 잠금 유지. 실제 폐기는 별도 명시적 결정 필요 |
| 같은 owner·slot의 부재와 cleanup 확인 | 확인 기록 commit 이후에만 후속 명시적 등록의 전제 충족 |

이번 fixture의 `Continuity::Verified`와 `AbsenceEvidence::fixture`는 신뢰할 이력, 현재 원 owner의 정확한 slot 부재 관찰이라는 명시적 시험 전제다. 이를 발행할 production 경로, 서명 검증 또는 Keychain query는 없다. 잠김·권한 거부·다른 group의 not-found가 이 증거로 바뀌는 실제 OS 분류도 아직 구현하지 않았다. `CleanupProof::BothReleased`는 새 owner가 과거 자원 종료를 확인했다는 전제이고, 재시작 자체의 ACK가 아니다.

`registration_generation()`은 후속 명시적 등록의 후보 세대만 반환하며 admission을 열거나 실제 등록을 실행하지 않는다. operation·ticket sequence 고갈은 `Exhausted`로 유지해 cleanup만 계속하고 조정을 통한 해제를 막는다. 마지막 operation을 정상 완료한 경우에도 다음 operation을 발급할 수 없으면 등록 후보를 거부한다. 실제 등록·회전 intent와 새 generation 예약의 영속 연결은 후속 구현이다. 과거 generation이나 consent를 복구하는 경로를 만들지 않는다.

## 결정적 회귀

아래 계약을 25개 시험으로 확인했다. 중첩된 crash·오류·identity 사례는 개별 시험 개수와 별도로 합산하지 않는다.

| 사례 | 확인할 관찰 |
|---|---|
| 최초 쓰기 지연 | 쓰기를 기다리지 않는 취소·abort·cleanup, Host clock 진행, admission 차단 |
| 최초 쓰기 실패 | 위 동작 유지, 오류 보고, 영속 접수 성공·삭제 선언 없음 |
| 위 두 경우의 활성 run | 취소 뒤 수락 delta 0, 기존 partial output 보존, terminal count 1 |
| 두 owner 순서·단일 ACK·deadline | 두 해제 근거와 삭제 의도 commit 이전 삭제 0 |
| 각 commit 전후 crash | commit된 모델만 복원, 새 instance의 admission·resolve 0 |
| 삭제 성공 뒤 기록 실패·응답 유실 | 미확인 잠금, 자동 삭제 재시도·등록·회전 0 |
| 중복 명령·늦은 결과·다른 identity | 중복 효과 또는 과거 결과에 의한 잠금 해제 0 |
| 누락·손상·과거 확인 기록 | 새 명시적 reconciliation 전 admission 0 |
| 명시적 부재·잔존·불명 조정 | 동일 slot의 증거와 commit 순서, 잔존·불명 시 잠금 유지 |
| Operation·generation·ticket sequence 고갈 | cleanup 계속, 조정을 통한 고갈 잠금 해제 없음, 마지막 operation 이후 재시작해도 등록 후보 거부 |

## 검증 환경과 결과

2026-11-25 Linux Debian 13 x86_64, Rust/Cargo 1.100.0에서 실행했다. 원본 library를 그대로 컴파일한 결과이며 macOS 전체 Rust 검증이 아니다. 집중 회귀와 전체 회귀의 중복은 합산하지 않는다.

Linux의 임시 Cargo harness는 저장소의 원래 `src/lib.rs`를 직접 참조한다. 실제 Tauri 2.12.5를 `default-features=false`로 사용하며 Tauri shim이나 소스 복제·변형은 없다. 기존 `build.rs`가 macOS 전용 native build를 수행하므로 이 harness는 library protocol 회귀용이다. 제품 의존성·lockfile을 바꾸지 않았고, harness dependency 364개가 저장소와 같은 version/checksum임을 대조했다. 새 제품 dependency는 없다.

작업 전용 toolchain 및 GTK/WebKit sysroot를 설정한 wrapper로 아래 Cargo 명령을 실행했다. `G3_HARNESS`는 저장소 밖 임시 manifest 디렉터리다. `--offline`은 Cargo의 dependency 네트워크 비사용이며 OS network-denied 시험이 아니다.

```sh
cargo test --manifest-path "$G3_HARNESS/Cargo.toml" --locked --offline --lib -- --skip loopback_ --test-threads=1
cargo test --manifest-path "$G3_HARNESS/Cargo.toml" --locked --doc
cargo test --manifest-path "$G3_HARNESS/Cargo.toml" --locked --lib -- --test-threads=1
python3 -B scripts/validate_governance.py
git diff --cached --check
```

| 검사 | 현재 기록 |
|---|---|
| G3 protocol·메모리 crash·실제 Host/동의 회귀 | **25/25 PASS**, 아래 149개에 포함 |
| 소켓 시험 제외 원본 library | **149/149 PASS**, 기존 loopback 5개 제외 |
| 기존 compile-fail doctest | **2/2 PASS** |
| TLS 포함 원본 library | **153 PASS / 1 FAIL**, 총 154개. 전체 PASS가 아님 |
| Governance·whitespace·상대 링크·변경 범위·민감정보 | **PASS**, Governance 21개 항목·상대 링크 4개·변경 파일 4개 |
| macOS 전체 Rust·Runtime binary·network-denied·TLS loopback | NOT_RUN |
| 실제 journal 저장소·process lock·전원 장애·rollback 방어 | NOT_RUN |
| Keychain·signed fixture·secure entry·packaged GUI·signed release | NOT_RUN |
| 실제 Provider·API Key·billing | 미접근, 실행 횟수 0 |

TLS 실패는 기존 `loopback_wrong_hostname_rejected_before_http`다. Python TLS peer가 handshake 후 최초 `stream.readline(256)`에서 `BrokenPipeError`로 종료하여 Rust의 peer 종료 상태 검사가 실패했다. **변경 없는 기준 main `daeadb5110dd78f6d6abd677887fbc52bf0e5671`**을 별도 harness에서 같은 exact filter로 실행해 동일 오류를 재현했다. 나머지 Linux loopback 4개는 PASS다. 이 Linux 환경의 기존 fixture 실패를 이번 변경으로 해결했다고 주장하지 않으며, 기존 transport/loopback 파일은 수정하지 않았다. macOS TLS 결과로도 대체하지 않는다.

149개 통과 실행 전후 아래 소스 SHA-256이 같았다. PR에는 실제 commit SHA와 같은 소스의 검증 결과 및 GitHub CI를 연결한다.

| 소스 | SHA-256 |
|---|---|
| `provider/mod.rs` | `cf497d257dc612c52a5281e36d3f948452c1d5ca748eb70dc37ca94f0404927b` |
| `provider/revocation.rs` | `d54f21e8261f8736990d29820b1f2f9418c028df08d518479e0d869c05fc6ab0` |
| `provider/revocation/tests.rs` | `440c4be8841e7f753146fe08e1e76b90860b7818e6623b7a3de104c79cbb350b` |

## 보존과 후속 판정

기존 Linux 검토 checkout은 보존한다. 이 작업 환경에서는 사용자 Mac의 P0 고정 앱, 원본 289개·추가 64/14/48개 증거 및 기존 Mac worktree에 접근할 수 없다. 따라서 이 파일은 해당 항목의 새 해시 대조나 Mac `main` 동기화를 주장하지 않는다.

E5b/E5c는 `PARTIAL`, E5d ADR은 `PROPOSED`, 전역 Runtime은 미선정이다. 이 PR의 오프라인 회귀가 통과해도 G2 OS 시험은 `NOT_RUN`, 제품 G3는 `NOT_IMPLEMENTED`로 남는다. G3의 최종 통과에는 실제 저장소·단일 owner·G2 항목·서명 앱을 함께 검증한 증거가 필요하다.

Draft PR은 최신 HEAD의 별도 병합 승인까지 미병합으로 유지한다. 이번 변경의 복구는 해당 소스·fixture·계약 문서를 함께 되돌리는 리뷰된 PR로 수행할 수 있다. 실제 항목·파일 migration은 없으며, 후속 작업에서도 journal 삭제·재설치로 미확인 잠금을 초기화하지 않는다.

## 2026-11-26 · G3 병합본 macOS 회귀

[Issue #70](https://github.com/Kim-Ji-Won/AETHER/issues/70)에서 [PR #69](https://github.com/Kim-Ji-Won/AETHER/pull/69)의 squash `b15809841e7f3e9f6cc1f680bae920fe8fb8593a`를 검증했다. 유일한 parent는 `daeadb5110dd78f6d6abd677887fbc52bf0e5671`, 승인된 feature와 squash의 tree는 `27923f5d1fcb2ac28dc43d9e634ccb0c32df03f8`다. 테스트를 끝낸 뒤 이 증거 절을 추가했다.

- 환경은 macOS 26.6 (25G5043d) arm64, macOS SDK 26.2, rustc/cargo 1.99.1이다. 기존 macOS Cargo manifest·build.rs를 그대로 사용했다. Issue 전용 worktree에 기존 캐시와 sidecar를 APFS 복사하고 별도의 target·Swift 모듈 캐시를 사용했다. `python3 -B scripts/prepare_node_sidecar.py --verify-only`로 고정 Node와 LICENSE를 대조했다.
- 실행 전후 HEAD·Rust 소스 tree·추적 소스 diff와 이 문서의 세 소스 SHA-256을 대조했다. 소스 tree는 `27c5ffd1176b4bdf0af29f6732f5d15e27ce1a2b`, Cargo.lock SHA-256은 `9496fefeabe913a0fb282d12f648b0d1dbdef0ad0e169df1ee308340e9133f80`로 같았다.

| Mac에서 실제 실행한 검사 | 관찰 결과 |
|---|---|
| 전체 Rust | **198/198 PASS**. Library 154, 앱 binary 42, compile-fail doctest 2. 실패·ignored 0 |
| G3 집중 filter | **25/25 PASS**. 전체 library에도 같은 25개 포함 |
| 기존 회귀 | Host·Synthetic 46, Provider 49, Credential/Consent 19, Transport 15, 앱 Runtime 23(기존 14 + G1 9) 모두 PASS. 전체 Rust의 부분집합 |
| 기존 TLS loopback | **5/5 PASS**. 정상 스트림·peer close, redirect, 잘못된 hostname, 비신뢰 CA, cancel/close. Transport에 포함 |
| macOS OS network-denied | **172/172 PASS**. loopback 제외 library 149 + Runtime binary 23. 실행 프로세스에 sandbox-exec로 네트워크를 차단 |
| Frontend·Typecheck, packaged GUI | **NOT_RUN**. 앱/UI 소스 변경 없는 Rust 병합본 검증 |
| 실제 filesystem durability·process lock·rollback 탐지, Keychain 부재 확인 | **NOT_RUN**. 메모리 journal의 commit·crash 모델만 검증 |

저장소 루트에서 실행했다. 아래 두 변수는 이번 Cargo 출력에서 확인한 시험 실행 파일이다. Cargo offline은 dependency 다운로드를 막고, 별도의 sandbox-exec 실행이 OS 차단 증거를 제공한다.

```sh
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --offline --locked
LIB=apps/desktop/src-tauri/target/debug/deps/aether_desktop-8aa63b36155ccae4
BIN=apps/desktop/src-tauri/target/debug/deps/aether_desktop-f2494fff42e599fb
"$LIB" agent_text::provider::revocation::tests:: --quiet
sandbox-exec -p '(version 1)(allow default)(deny network*)' "$LIB" --skip loopback --quiet
sandbox-exec -p '(version 1)(allow default)(deny network*)' "$BIN" agent_runtime::tests --quiet
```

G3의 최초 쓰기 지연·실패 사례에서 admission 차단과 동의 폐기, cancel/abort·cleanup 진행, 취소 후 수락 delta 0, 기존 partial output과 terminal count 1을 확인했다. 양쪽 cleanup과 삭제 의도 commit 이전에는 삭제가 없었다. 메모리 crash·응답 유실·stale 결과·이전 instance의 ACK/승인이 잠금을 해제하지 못했고, operation·generation·ticket sequence 고갈 및 마지막 operation 후 등록 후보 거부도 통과했다. [회귀의 실제 assertions](../../apps/desktop/src-tauri/src/agent_text/provider/revocation/tests.rs)

Linux의 TLS 포함 **153 PASS / 1 FAIL** 기록을 유지한다. `loopback_wrong_hostname_rejected_before_http`는 이 Mac에서 PASS였고 Linux에서는 변경 전 parent에서도 Python peer의 BrokenPipeError가 재현됐다. 이번에 transport 소스·시험·timeout을 수정하지 않았다. 로컬 TLS 이외의 Provider 호출이나 과금은 없었다.

G1 앱 Host와 test-only G3 오프라인 기반은 main에 병합됐다. 제품은 `DisabledCredentials` 및 `ProviderDisabled`이며 G2 실제 OS 시험 **NOT_RUN**, 제품 G3 영속 journal 통합 **NOT_IMPLEMENTED**, 첫 실제 Provider 요청 **BLOCKED**다. P0 고정 앱·289개 및 64/14/48개 증거를 기존 기준과 대조했다. E5b/E5c PARTIAL, E5d ADR PROPOSED, 전역 Runtime 미선정이다.
