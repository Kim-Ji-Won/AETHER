# E4d 취소·중복·잘못된 IPC 안전성

[Issue #21](https://github.com/Kim-Ji-Won/AETHER/issues/21) · 기준 `ab1ad2c914da96b341cacfffbf1b1c8e47325817`.
[RFC](rfc-001-aether-system.md)는 PROPOSED입니다. 이 실험은 합성 작업의 메모리 상태·IPC 검사이며 외부 효과의 exactly-once 보장이나 Node sandbox 증명이 아닙니다.

## 상태와 취소

Rust Mutex 안에서 `created → running → succeeded/failed`와 `created/running → cancel_requested → cancelled`를 직렬화합니다. cancel_requested가 먼저 확정되면 이후 Provider event는 거부합니다. succeeded/failed가 먼저 확정되면 cancel은 Already terminal로 거부합니다. terminal_count는 세 terminal 상태 중 하나에서만 1이 됩니다. seq와 trace에는 수락한 Provider event 및 최종 cancelled/failed 한 건만 기록됩니다. 최대 trace 길이는 5입니다.

취소 command는 `{run_id, variant}`만 받으며 정확한 현재 작업과 Runtime 소유권을 확인합니다. 같은 작업의 중복 cancel은 동일 snapshot(또는 cleanup이 완료된 cancelled snapshot)을 반환하고 trace를 추가하지 않습니다. 이전 run_id·다른 variant는 거부합니다. A는 외부 프로세스가 없어 cancel 호출에서 cancelled까지 확정한 뒤 Provider AbortSignal을 중단합니다. B는 cancel_requested를 기록하고 worker가 최대 10ms polling 단위로 감지해 **자기가 생성한 child handle**만 kill/wait한 뒤 cancelled를 확정합니다. OS cleanup을 확인하지 못하면 busy/cancel_requested를 유지하고 성공적인 취소로 표현하지 않습니다. pipe/child 대기 중 원장 Mutex를 보유하지 않습니다.

terminal 직전 완료와 취소는 같은 Mutex를 획득한 순서로 결정됩니다. B의 FAKE_OK는 done/EOF 및 exit 0 이후 원장에 commit합니다. 취소가 먼저 기록되면 이미 검증한 FAKE_OK도 commit하지 않습니다. Renderer의 timer나 Provider 출력으로 terminal을 확정하지 않습니다.

## 중복·보존 범위

시작 command는 `{token, mode}`만 받습니다. token은 1..u32::MAX의 단조 증가 요청 식별자이고 mode는 normal/pause25 두 값뿐입니다. Rust가 run_id를 따로 발급하고 request_id에 token을 기록합니다. UI는 host의 마지막 request_id+1을 사용합니다. 동일 token/variant/mode는 Duplicate start, 동일 token의 다른 mode/variant는 Start conflict, 과거 token은 Stale start token입니다. 거부된 시작은 child를 생성하지 않습니다. token이 소진되면 새 시작은 거부되며 영속 복구는 지원하지 않습니다.

수락한 동일 identity/seq/type/value의 재전송은 Duplicate frame, 같은 identity/seq의 다른 type/value는 Frame conflict로 거부합니다. idempotent 성공을 가장하지 않는 명시적 duplicate 정책입니다. event 거부는 trace·seq·terminal_count를 증가시키지 않습니다. cancelled/failed/succeeded 이후에도 새 event는 상태를 바꾸지 못합니다.

보존 범위는 **최신 run 한 건**, 그 run의 최대 5개 trace 및 마지막 시작 token/mode/variant입니다. 이전 run의 상세 payload는 폐기하지만 high-water token과 증가하는 run_id로 오래된 시작/이벤트를 거부합니다. generation은 현재 host에서 1로 고정하며 다른 generation을 거부합니다. reload/host restart 세대 교체는 E4e의 미실행 범위입니다. 프로세스 종료 후 기록이 유지되거나 재시작을 가로지르는 dedupe를 보장하지 않습니다.

## 계약과 크기 한계

Provider의 기존 7-field closed schema(version/run_id/request_id/generation/seq/type/value), u32 범위, 짧은 type/value, 정확한 progress 순서는 유지합니다. A/B가 같은 parser와 원장 전이를 사용합니다. Tauri command의 JSON 크기 검사는 **Tauri가 JSON을 파싱한 이후**의 검사입니다. serialization 기준 event 511 bytes, start/cancel 80 bytes 이내이며 WebView transport 전체의 allocation 상한이라는 의미가 아닙니다.

B transport는 UTF-8 NDJSON LF 포함 512 bytes, ready+5 events+done+EOF의 최대 8개 reader 결과, frame당 기본 5초 timeout을 유지합니다. Node 입력에도 512-byte buffer와 한 건 queue 제한을 적용하고 fatal UTF-8 decode를 사용합니다. ACK의 version/type/run_id/seq와 exact keys, done 및 EOF를 확인합니다. malformed/duplicate/conflict로 B protocol이 끊어지면 해당 child를 회수한 뒤 active run을 failed로 닫습니다. A의 단일 event 거부는 원장을 그대로 두며 UI Provider 중단 시 별도의 valid failed event로 닫습니다. 이 transport 처리 차이와 공통 event 거부의 무변경 성질을 구분합니다. 이미 수락한 prefix가 있는 잘못된 done의 경우 running 기록은 존재하지만 succeeded는 생성하지 않습니다.

stdout은 protocol 전용이며 stderr는 제한된 byte 수만 진단합니다. frame 내용·개인 경로·환경 전체를 로그에 남기지 않습니다. 고정 Node pin/LICENSE·경로·빈 환경·CSP·Swift/E3b 의미는 유지하고 새 권한은 local main의 cancel_fake_run 하나뿐입니다. 임의 PID/실행 파일/URL/script/코드를 받지 않습니다.

## 결정적인 취소 fixture

기본 normal은 기존 100ms 간격을 유지합니다. UI의 `25%에서 대기 (취소 실험)`은 고정 pause25 모드입니다. A는 progress 25의 host ACK 뒤 AbortSignal을 기다리고, B는 Rust가 progress 25를 수락한 뒤 ACK를 보류합니다. 사용자가 작업 취소를 누르면 host 상태를 기준으로 Provider/child를 중단합니다. 무한 대기를 피하기 위해 barrier 상한은 300초이며 초과 시 실패 정리합니다. 이 버튼은 실패 실험용이며 일반 작업/외부 입력을 실행하지 않습니다.

## 검증 방법

[E4c 준비](e4c-sidecar-parity.md)의 pinned runtime을 재사용합니다. 아래 명령의 cwd를 구분합니다. 의존성 설치 없이 기존 lockfile과 Node pin을 유지합니다.

```sh
# repository root
export PATH="$HOME/.cargo/bin:$PATH"
python3 scripts/prepare_node_sidecar.py --verify-only
cd apps/desktop
npm run build
node --experimental-strip-types --test tests/fake-provider.test.mjs tests/sidecar-protocol.test.mjs
cd ../..
CARGO_TARGET_DIR="$PWD/target/e4d-issue-21" cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --offline -- --nocapture
cd apps/desktop
CARGO_TARGET_DIR="$PWD/../../target/e4d-issue-21" CARGO_NET_OFFLINE=true npm run tauri -- build --bundles app --no-sign -- --locked --offline
open -n "$PWD/../../target/e4d-issue-21/release/bundle/macos/AETHER.app"
```

Rust tests는 기존 E3b/E4b/E4c 테스트를 유지합니다. 공통 원장 tests는 A/B 각각 취소 barrier(created/25/75)·replay·start dedupe·terminal race 10회, malformed 논리 조건 42개×5회를 수행합니다. 실제 pinned Node tests는 created/25 취소 각 10회와 실패 fixture 13종×5회마다 wait/reap 및 다음 정상 요청을 확인합니다. `sidecar-fault.mjs`는 test-only peer로 bundle에 포함되지 않고 테스트가 만든 임시 script만 정리합니다. Node entry의 malformed 입력 7종×5회, 잘못된 ACK 6종×5회도 별도 subprocess 테스트합니다. timeout fault에는 빠른 단위 검증을 위한 150ms를 사용하며 제품 5초와 구분합니다.

race는 동시 barrier 10회 외에 cancel→terminal 및 terminal→cancel 두 순서를 강제로 확인합니다. 실제 scheduler에서 양쪽 승자가 모두 관찰됐다고 가정하지 않습니다. 합성 late frame 주입·race·malformed는 Rust/Node 테스트 증거이고 GUI 클릭 증거와 구분합니다. 최종 source SHA, 실제 실행 횟수·실패 수·자식 PID·화면 관찰은 Issue/PR에 기록합니다.

E4e의 앱 Quit/강제 종료, renderer crash/reload, close/reopen, host restart, 비정상 Sidecar 종료·orphan 복구, 승인창 부모 종료와 E5 장시간·성능·배터리는 NOT_RUN입니다. 다른 Mac, 서명·공증도 실행하지 않습니다.
