# E4c Bundled Node 정상 경로와 A/B parity

[Issue #19](https://github.com/Kim-Ji-Won/AETHER/issues/19) · 기준 `a833e00093c40ab3c32467450ac8a8ac51c96e69`.
E4b의 Option A를 유지하고 같은 합성 작업을 Option B에서 실행하는 실험입니다. [RFC](rfc-001-aether-system.md)는 **PROPOSED**이며 Runtime 최종 선택·성능 비교가 아닙니다.

## 패키징 선택과 공급망

[Tauri externalBin](https://v2.tauri.app/develop/sidecar/)은 target triple 이름과 shell plugin 실행 경로를 제공합니다. 이번에는 [bundle.resources](https://v2.tauri.app/develop/resources/)의 명시적 map과 Rust `std::process::Command`를 사용합니다. `Contents/Resources/sidecar/`에 실행 파일·고정 script·LICENSE를 넣습니다. npm/Cargo dependency나 shell plugin을 추가하지 않고, WebView에 shell/파일/프로세스 API를 열지 않습니다.

Node 버전은 로컬 실험 재현을 위해 `22.21.0`으로 고정합니다. 최신 보안 릴리스나 공개 배포에 적합한 버전이라고 주장하지 않습니다. 공식 [release](https://nodejs.org/en/blog/release/v22.21.0), [archive](https://nodejs.org/download/release/v22.21.0/node-v22.21.0-darwin-arm64.tar.gz), [SHASUMS256](https://nodejs.org/download/release/v22.21.0/SHASUMS256.txt)를 사용합니다.

| 대상 | SHA-256 |
|---|---|
| official darwin-arm64 tar.gz | `dbc1a17024a32827adb23b5b11ce98cefcd783145a30fe41bb2845be711e9742` |
| archive의 bin/node | `64bcb939f5f8c921d104b02d0eb960781c13b49b9133d1d71a768eadd62da401` |
| archive의 전체 LICENSE | `e991d81497a85bb24fc6bffae0a3637a6accd6c6bc5ce1f2c5698bd555cf9d49` |

공식 HTTPS checksum과 archive를 비교한 뒤, archive에서 파생한 Node·LICENSE hash도 [provenance manifest](../../apps/desktop/sidecar/node-provenance.json)에 고정합니다. PGP 서명 검증은 **NOT_RUN**이며 HTTPS/checksum 검증과 구분합니다. 준비 script는 고정된 두 regular member만 읽으며 archive 전체를 임의 경로로 추출하지 않습니다. global Node를 복사하거나 설치하지 않습니다.

Node 자체는 MIT 조건이며 배포물에는 여러 third-party 조건이 함께 있습니다. [공식 LICENSE](https://github.com/nodejs/node/blob/v22.21.0/LICENSE)의 copyright·허가 문구·third-party notices를 포함한 전체 파일을 변경 없이 번들에 넣습니다. 라이선스 파일의 hash도 검사합니다. 공개 배포·서명·공증 및 당시 최신 버전/취약점 검토는 별도 gate입니다.

## 준비와 빌드

저장소 루트에서 기존 toolchain으로 실행합니다. 첫 명령의 `--download`만 공식 runtime 다운로드를 허용합니다. 이후 `npm run build` 및 Cargo build는 다운로드하지 않고 준비된 Node·LICENSE hash를 검증하며, 누락/불일치 시 실패합니다. Node archive는 `target/e4c-download/`, 생성된 bundle resources는 `apps/desktop/src-tauri/sidecar/generated/`에 있으며 모두 Git에서 제외됩니다.

```sh
python3 scripts/prepare_node_sidecar.py --download
export PATH="$HOME/.cargo/bin:$PATH"
cd apps/desktop
npm run build
node --experimental-strip-types --test tests/fake-provider.test.mjs
cd ../..
CARGO_TARGET_DIR="$PWD/target/e4c-issue-19" cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --offline
cd apps/desktop
CARGO_TARGET_DIR="$PWD/../../target/e4c-issue-19" CARGO_NET_OFFLINE=true npm run tauri -- build --bundles app --no-sign -- --locked --offline
open -n "$PWD/../../target/e4c-issue-19/release/bundle/macos/AETHER.app"
```

`sidecar/build.mjs`는 기존 TypeScript compiler로 A가 import하는 **같은** `src/fake-provider.ts`를 JS로 컴파일하고 고정 `entry.mjs`를 복사합니다. 스크립트 내용의 변경은 정상 source review 대상이며 런타임에 다운로드하지 않습니다. `package.json`의 build script만 바뀌며 dependency·lockfile은 유지됩니다. `--no-sign`의 로컬 산출물은 배포용 서명 증거가 아닙니다.

## 실행 경계와 IPC

- UI는 인자 없는 `start_node_fake_run`만 호출합니다. local `main`의 기존 합성 작업 capability에 이 명령 하나만 추가합니다. AppManifest 등록, 기존 CSP·E3b 승인은 유지합니다.
- Rust는 Tauri `resource_dir()`에서 고정 `sidecar/node`, `entry.mjs`, `fake-provider.js`, `LICENSE`의 canonical path를 확인합니다. bundle directory 밖으로 향하는 파일·directory symlink를 거부하고 dev 경로나 PATH fallback을 사용하지 않습니다. Node hash는 준비·빌드 때 검증하며 런타임의 OS 보안 격리나 unsigned bundle 변조 방지로 표현하지 않습니다.
- 고정 executable·entry 인자, `env_clear()`, bundle 내부 cwd, private stdin/stdout/stderr를 사용합니다. `NODE_OPTIONS`, API key, host 환경 전체를 전달하지 않습니다. Node 자체의 파일·네트워크·프로세스 능력을 제거한 것은 아닙니다. 이번 코드에는 외부 네트워크·실제 도구 실행이 없습니다.
- protocol은 UTF-8 JSON + LF이며 frame 하나가 LF 포함 최대 512 bytes입니다. Node 입력 buffer/queue, Rust reader 및 전체 frame 수를 제한합니다. stdout은 protocol 전용, stderr는 최대 4097 bytes만 읽어 크기만 진단하며 내용을 공개하지 않습니다.

| 순서 | 방향 | 계약 |
|---|---|---|
| begin | Rust → Node | version 1, type begin, host run_id/request_id/generation, 고정 input |
| ready | Node → Rust | version 1, type ready, 실제 child PID, v22.21.0, arm64 확인 |
| event ×5 | Node → Rust | E4b와 같은 7개 필드: version/run_id/request_id/generation/seq/type/value |
| ACK | Rust → Node | version 1, type ack, run_id, 승인한 seq |
| error | Rust → Node | 고정 PROTOCOL_REJECTED 오류, 이후 이번 child 정리 |
| done / EOF | Node → Rust | version 1, type done, run_id, seq 5; stdout 종료 |
| exit / reap | OS → Rust | exit 0과 회수 확인 후에만 terminal을 원장에 게시 |

Node는 같은 Provider의 각 event에 대한 ACK를 기다리고 다음 100 ms 지연을 시작합니다. Rust는 E4b ID·seq·payload·상태 검증을 공유합니다. 마지막 terminal은 먼저 검증만 하고 child의 정상 종료 후 한 번 commit합니다. 비정상 처리 시 이번 active run만 `failed`로 닫으며 기존 terminal을 덮어쓰지 않습니다. 수신/exit에는 5초 제한이 있고 protocol 오류에서 child kill/wait를 수행합니다. 이는 최소 실패 정리이며 광범위한 crash/cancel 보장은 E4d/E4e에서 검증해야 합니다.

원장은 최신 run 하나입니다. snapshot의 기존 10개 필드 뒤에 `variant` (`webview-a` / `node-b`)를 추가합니다. B session이 끝날 때까지 새 run 생성을 막고, WebView의 기존 event 제출 명령이 B run을 갱신하지 못하게 합니다. Renderer는 B를 시작하고 host snapshot을 50 ms 간격으로 읽지만 Node frame을 직접 전달하지 않습니다. Rust는 state lock을 pipe read/write 또는 child wait 동안 보유하지 않습니다.

현재 E4d 브랜치의 start token/mode, 취소 상태, 실패 검증은 [E4d 문서](e4d-failure-safety.md)를 참고하세요. 아래는 E4c 당시 증거 범위입니다.

## 검증과 해석

같은 최종 `.app`에서 `WebView A 실행`, `Node B 실행`을 각각 클릭하고 host trace와 화면의 variant·동일 run identity·seq 1~5·25/50/75·FAKE_OK·terminal 1건을 대조합니다. 별개의 A/B run_id 숫자가 다르는 것은 정상입니다. Node B의 UI 완료에는 host가 확인한 `exit 0 / reaped`가 포함됩니다.

Rust 테스트는 기존 E3b 2개, E4b 4개, bounded frame과 **실제 pinned Node** handshake/ACK/원장 완료 테스트를 포함합니다. Provider 테스트와 packaged GUI는 별도 증거입니다. 최종 source SHA, executable/Node hash, 실제 GUI 자동화 여부·PID·시작 시각·exit 결과는 Issue에 기록합니다.

독립 실행은 `.app`을 build 폴더 밖 임시 폴더에 복사하고 cwd를 별도 폴더로 설정하며 `PATH=/usr/bin:/bin`의 최소 환경으로 번들 executable을 실행합니다. Node child의 OS 관찰 경로가 복사한 `Contents/Resources/sidecar/node`인지 확인하고, host trace의 PID/시작 시각과 대조하며 종료 후 잔존 여부도 확인합니다. 기존 코어·macOS·AX·승인 취소/승인 및 `--native-bridge-check`를 같은 복사본에서 확인합니다.

기존 앱·DMG·E4b 출력은 보존합니다. E4d malformed/replay/cancel, E4e crash/reload/close/restart, E5 장시간·성능·배터리, 다른 Mac, 서명·공증·공개 배포는 **NOT_RUN**입니다. E4c 성공은 정상 fixture parity와 로컬 패키징의 증거이며 Node sandbox 또는 Runtime 최종 선정의 근거를 완성하지 않습니다.
