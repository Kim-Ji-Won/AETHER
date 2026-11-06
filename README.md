# AETHER

**자연스러운 한국어 음성 대화와 권한 기반 컴퓨터 작업을 목표로 하는 macOS AI Assistant**입니다. Apple Silicon을 초기 대상으로 하며, 누구나 설치할 수 있는 공개 배포용 앱을 지향합니다.

## 현재 상태 · Status

**P0-B-E1a — 첫 창과 로컬 앱 번들 검증** 단계입니다. `apps/desktop/`에 Tauri 2 + React + TypeScript + Vite 기반 최소 앱을 구현했습니다. Apple Silicon Mac에서 `AETHER.app` 빌드·독립 실행과 실제 창의 클릭 동작을 확인했습니다. 초기 `코어 대기 중`에서 `코어 점등` 버튼을 누르면 링이 밝아지고 `코어 점등됨`으로 바뀝니다. 상태는 앱을 다시 시작하면 초기화됩니다.

이는 로컬 실험이며 배포 가능한 제품이 아닙니다. 음성·AI·컴퓨터 제어·메모리 기능은 아직 구현하지 않았습니다. Developer ID 서명·공증·DMG 배포는 E1b 범위입니다.

## macOS 실행 · Local Run

Apple Silicon Mac에서 기존 Xcode / Command Line Tools, macOS SDK, Node.js와 npm, `aarch64-apple-darwin` Rust toolchain이 필요합니다. 검증 환경은 Node.js `22.21.0`, npm `10.9.4`, Rust / Cargo `1.99.1`입니다. 저장소 루트에서 실행합니다. 의존성 최초 설치·빌드는 네트워크와 여유 디스크가 필요합니다.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cd apps/desktop
npm ci
npm run build
npm run tauri -- build --bundles app
open "$PWD/src-tauri/target/release/bundle/macos/AETHER.app"
```

`npm run build`는 TypeScript 검사와 Vite production build를 실행합니다. `npm run tauri`는 프로젝트 로컬 CLI를 사용합니다. 생성된 `.app`은 개발 서버 없이 실행하며, `src-tauri/target/`에 보존되고 Git에서는 제외됩니다. 개발 중에는 같은 디렉터리에서 `npm run tauri -- dev`를 사용할 수 있습니다.

2026-11-06 로컬 검증: production build, `.app` 생성, `Info.plist`의 실행 파일에 대한 `Mach-O arm64` 확인, 네이티브 창 제목과 클릭 전후 문구·링 밝기 변화 모두 `PASS`입니다. GitHub의 `governance` CI는 저장소 정책 검사이며 macOS 빌드·GUI 검증을 대신하지 않습니다. 실험과 검토 기록은 [E1a Issue #4](https://github.com/Kim-Ji-Won/AETHER/issues/4)를 참고하세요.

## 후보 아키텍처 · Candidate Architecture

| 영역 | 검토 중인 기술 |
|---|---|
| Desktop / UI | Tauri 2, React, TypeScript |
| Native host | Rust, Swift를 통한 macOS 연동 |
| Agent / AI | TypeScript, OpenAI Responses API / Realtime API |
| Storage | SQLite |
| Browser automation | Playwright |
| Visualization | Three.js / React Three Fiber |
| Distribution | Developer ID 서명, 공증된 DMG |

Desktop / UI의 최소 구성만 E1a에서 검증했습니다. 나머지 기술은 **후보이며 확정된 결정이나 검증된 기능이 아닙니다.** [아키텍처 검토](docs/architecture/README.md)와 [ADR 작성 기준](docs/decisions/README.md)을 참고하세요.

## 참여 및 검증 · Contributing

변경 전에 [CONTRIBUTING](CONTRIBUTING.md), [AGENTS](AGENTS.md), [Git workflow](docs/engineering/git-workflow.md)를 읽어주세요. Python 3와 Git이 있는 환경에서 저장소 루트를 기준으로 다음 검사를 실행할 수 있습니다.

```sh
python3 scripts/validate_governance.py
```

외부 패키지를 설치하지 않는 거버넌스 검사이며, 앱 테스트가 아닙니다. 문서와 Issue·PR은 **한국어 설명을 기본으로 하고 기술 용어·명령·식별자는 영어로 유지**합니다.

보안 문제는 [SECURITY](SECURITY.md)를 따르며 공개 Issue에 취약점 상세를 올리지 않습니다. 커뮤니티 참여에는 [Code of Conduct](CODE_OF_CONDUCT.md)가 적용됩니다.

## 라이선스 · License

소프트웨어 라이선스는 아직 선정하지 않았습니다. 공개 저장소라는 사실만으로 사용 허가가 부여되지는 않습니다. 외부 코드의 본격적인 기여 수락이나 제품 배포 전에 관리자가 결정해야 합니다.
