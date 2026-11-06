# AETHER

**자연스러운 한국어 음성 대화와 권한 기반 컴퓨터 작업을 목표로 하는 macOS AI Assistant**입니다. Apple Silicon을 초기 대상으로 하며, 누구나 설치할 수 있는 공개 배포용 앱을 지향합니다.

## 현재 상태 · Status

**P0 — Architecture & Feasibility (아키텍처·실현 가능성 검토)** 단계입니다. 현재 저장소에는 개발 거버넌스만 있으며, 데스크톱 앱·음성·에이전트·컴퓨터 제어·메모리 기능은 아직 구현되지 않았습니다. 실행 가능한 앱이나 지원되는 앱 빌드 명령도 없습니다. P0-B 구현은 별도 Issue와 명시적 범위 승인 후 시작합니다.

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

위 기술은 **후보이며 확정된 결정이나 검증된 기능이 아닙니다.** [아키텍처 검토](docs/architecture/README.md)와 [ADR 작성 기준](docs/decisions/README.md)을 참고하세요.

## 참여 및 검증 · Contributing

변경 전에 [CONTRIBUTING](CONTRIBUTING.md), [AGENTS](AGENTS.md), [Git workflow](docs/engineering/git-workflow.md)를 읽어주세요. Python 3와 Git이 있는 환경에서 저장소 루트를 기준으로 다음 검사를 실행할 수 있습니다.

```sh
python3 scripts/validate_governance.py
```

외부 패키지를 설치하지 않는 거버넌스 검사이며, 앱 테스트가 아닙니다. 문서와 Issue·PR은 **한국어 설명을 기본으로 하고 기술 용어·명령·식별자는 영어로 유지**합니다.

보안 문제는 [SECURITY](SECURITY.md)를 따르며 공개 Issue에 취약점 상세를 올리지 않습니다. 커뮤니티 참여에는 [Code of Conduct](CODE_OF_CONDUCT.md)가 적용됩니다.

## 라이선스 · License

소프트웨어 라이선스는 아직 선정하지 않았습니다. 공개 저장소라는 사실만으로 사용 허가가 부여되지는 않습니다. 외부 코드의 본격적인 기여 수락이나 제품 배포 전에 관리자가 결정해야 합니다.
