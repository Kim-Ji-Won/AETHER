# E4b WebView Fake Provider 정상 경로

[Issue #17](https://github.com/Kim-Ji-Won/AETHER/issues/17) · 기준 `374eb6c8c696453706e4d503d61ccc4236841898`.
Option A의 좁은 합성 실험입니다. [RFC](rfc-001-aether-system.md)는 **PROPOSED**이며 E4 전체 비교나 Runtime 선정 결과가 아닙니다.

아래는 E4b 당시 계약·실행 기록입니다. E4c 작업 브랜치의 현재 A/B 실행법과 Node 준비 단계, snapshot variant 확장은 [E4c 정상 경로](e4c-sidecar-parity.md)를 참고하세요.

## 책임과 흐름

`FakeRun.tsx` → `create_fake_run` → Rust `created` snapshot → `fake-provider.ts` → `submit_fake_event` → Rust 검증·갱신 → 반환 snapshot 표시 → `get_fake_run` 재조회.

입력은 고정 fixture이고 사용자 입력 필드는 없습니다. Provider가 `start`, 기본 100 ms 간격의 `progress 25 / 50 / 75`, `terminal FAKE_OK`를 순차 전송합니다. 이전 IPC ACK 이후 다음 지연을 시작하므로 정확한 wall-clock 100 ms나 성능 수치를 보장하지 않습니다. React는 Provider의 timer나 결과만으로 완료를 확정하지 않습니다. 표시 이력은 Rust snapshot의 승인된 이벤트입니다.

Rust는 프로세스당 최신 작업 하나를 보유합니다. `created`/`running`이면 추가 생성은 거부하며, terminal 이후 명시적 새 요청만 새 ID를 만듭니다. 재시작하면 원장·ID·이력은 초기화됩니다. `generation=1`은 이번 host의 단일 연결 실험 값이며 reload/reconnect 구현이 아닙니다. ID는 프로세스 내부에서만 유효합니다.

## IPC 계약 v1

| command | 입력 | 반환 |
|---|---|---|
| `create_fake_run` | 없음 | Rust가 발급한 `created` snapshot |
| `submit_fake_event` | 아래 7개 필드의 JSON object | 검증 이후 snapshot 또는 고정 오류 |
| `get_fake_run` | 없음 | 현재 snapshot 또는 `null` |

frame 필드는 `version`, `run_id`, `request_id`, `generation`, `seq`, `type`, `value`입니다. 숫자는 `u32`, `type`은 최대 8 UTF-8 bytes, `value`는 최대 7 bytes입니다. 추가/누락 필드, nested 값, raw body는 거부합니다. 허용된 의미의 frame은 canonical JSON 기준 256 bytes 미만입니다. 이는 Tauri가 JSON을 파싱한 **후**의 schema/크기 제한이며 transport의 사전 allocation 제한은 아닙니다. 임의 텍스트·질문·metadata는 원장에 저장하지 않습니다.

| seq | type / value | Rust 상태 / 진행률 |
|---|---|---|
| 0 | host 생성 | `created` / 0 |
| 1 | `start` / 빈 문자열 | `running` / 0 |
| 2 | `progress` / `25` | `running` / 25 |
| 3 | `progress` / `50` | `running` / 50 |
| 4 | `progress` / `75` | `running` / 75 |
| 5 | `terminal` / `FAKE_OK` | `succeeded` / 75, terminal 1건 |

활성 상태의 다음 seq에 `failed` / 빈 문자열을 보내면 `failed` terminal 한 건으로 닫습니다. UI는 IPC 실패 시 원장을 다시 확인한 뒤 자신이 생성한 활성 작업에만 이 실패를 제출합니다. 재조회·실패 제출도 실패하면 마지막 확인 상태와 통신 오류를 표시하며 host의 실패를 꾸며내지 않습니다. 자동 timeout·복구는 이번 구현에 없습니다.

ID·version·generation·다음 seq·상태·고정 payload를 모두 확인한 뒤 `Mutex` 내부에서 한 번 갱신합니다. 거부는 원장을 변경하지 않습니다. terminal 이후 모든 이벤트를 거부하며 진행률 역행·건너뛰기를 허용하지 않습니다. 이력은 최대 5건이고 다음 작업이 대체합니다. `failed`는 synthetic 작업 상태이며 기존 E3b 승인 카운터에 영향을 주지 않습니다.

새 serialization 의존성을 추가하지 않기 위해 snapshot wire 형식은 JSON tuple입니다. 순서: `[version, run_id, request_id, generation, state, seq, progress, terminal_count, result, trace]`. `trace` 항목은 `[seq, type, value]`이며 바깥 snapshot의 식별자를 공유합니다. UI adapter가 이를 이름 있는 필드로 변환합니다.

## 권한·개인정보

세 명령을 `AppManifest::commands`에 등록하고 `main-fake-run` capability에서 `local: true`, `windows: ["main"]`으로만 허용합니다. [Tauri Runtime Authority](https://v2.tauri.app/security/runtime-authority/)가 origin/capability를 확인합니다. CSP, 원격 origin, 기존 native 승인 경계는 유지합니다. 자동 생성 permission 파일은 Git에서 제외합니다.

Provider에는 네트워크·파일·키·도구 호출이 없고 CSP의 `connect-src`는 기존 IPC endpoint만 허용합니다. Rust 진단은 고정 합성 ID·seq·type·상태만 stderr에 출력합니다. 화면이나 본문·개인 경로를 기록하지 않습니다. Renderer가 손상되면 허용된 합성 이벤트를 직접 제출할 수 있으므로 이 원장은 실제 작업 수행이나 사용자 승인을 증명하지 않습니다.

## 재실행

저장소 루트에서 기존 toolchain과 설치된 의존성을 사용합니다.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
CARGO_TARGET_DIR="$PWD/target/e4b-issue-17" cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --offline
python3 scripts/validate_governance.py
git diff --check
cd apps/desktop
node --experimental-strip-types --test tests/fake-provider.test.mjs
npm run build
CARGO_TARGET_DIR="$PWD/../../target/e4b-issue-17" CARGO_NET_OFFLINE=true npm run tauri -- build --bundles app --no-sign -- --locked --offline
open -n "$PWD/../../target/e4b-issue-17/release/bundle/macos/AETHER.app"
```

`Fake Run 실행` 클릭 후 동일 `run_id`, `succeeded · 완료`, Rust 확인 이력 `start → progress 25 → progress 50 → progress 75 → terminal FAKE_OK`, `terminal: 1`을 대조합니다. CLI는 해당 번들의 `Info.plist`에서 `CFBundleExecutable`을 찾아 `--native-bridge-check`로 실행합니다. 기존 코어·버전·AX·승인 취소/승인도 별도로 클릭합니다. 실제 결과·SHA·PID 및 GUI 자동화 여부는 Issue 검증 기록에 남깁니다. Rust/Provider 단위 테스트와 packaged GUI는 서로 다른 증거입니다.

## 남은 범위

E4c bundled Node 정상 경로, E4d 광범위한 malformed/replay/cancel, E4e reload/close/crash/Quit, E5 장시간·성능 비교는 **NOT_RUN**입니다. 다른 Mac, Developer ID·공증, 실제 AI·외부 도구, 실제 사용자 직접 승인도 이번 gate가 아닙니다. Swift bridge·기존 E3b 승인 의미·의존성·CI는 변경하지 않습니다.
