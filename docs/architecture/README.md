# 아키텍처 검토 · Architecture Evidence

승인된 production 아키텍처는 없습니다. 현재 후보는 Tauri/React, Rust–Swift native 연동, TypeScript orchestration, OpenAI Responses/Realtime, SQLite, Playwright, Three.js/R3F입니다. WebView/sidecar 런타임 배치, native bridge, 권한과 배포는 통제된 실험으로 검증해야 합니다.

검토는 실제 research Issue를 연결하고 목적·선택지·정확한 환경/버전·실험 범위·성공/실패 조건·관찰 증거·한계·후속 ADR을 기록합니다. 주장은 OBSERVED, DOCUMENTED, INFERRED, UNKNOWN, BLOCKED로 구분합니다. 개인 host 정보·민감 경로·자격 증명을 공개 증거에 복사하지 않습니다.

선택은 [ADR](../decisions/README.md)에 기록합니다. API 존재, OS 권한, 배포 정책 적합성, 서명된 앱의 실제 성공은 서로 다른 검증입니다. P0-B 실험은 구체 범위가 승인되기 전까지 시작하지 않습니다.

## E4a 설계 문서

- [RFC 001 — AETHER 통합 시스템](rfc-001-aether-system.md) · PROPOSED. E0~E3b 근거 감사, 모듈·권한·데이터 경계와 후보 선택 조건.
- [E4 Runtime 배치 비교 계획](e4-runtime-comparison-plan.md) · NOT_RUN. 동일 Fake Provider로 WebView / bundled Node를 비교하는 절차.
- [개발 로드맵](../engineering/development-roadmap.md) · PROPOSED. P0~P7의 선행 조건과 완료 기준.

[E4a Issue #15](https://github.com/Kim-Ji-Won/AETHER/issues/15)는 문서 준비 단계입니다. 실험 결과와 별도 승인 후에 [ADR](../decisions/README.md)로 결정을 기록합니다.

## P2.0 Runtime-independent 계약 준비

- [Agent Runtime 계약 명세](p2-0-agent-runtime-contract.md) · PROPOSED / DESIGN_ONLY. 상태·identity·event·cancel·오류·Provider/credential·UI 경계.
- [결정적 Fixture 설계](p2-0-contract-fixtures.md) · NOT_RUN. 가상 clock/barrier와 기대 oracle; 실행 가능한 테스트나 실환경 증거가 아님.

[Issue #46](https://github.com/Kim-Ji-Won/AETHER/issues/46)의 설계 리뷰 범위다. P0 Preflight 6/6 PASS와 E5c PARTIAL은 구분하며 Runtime A/B 선정·실제 Provider 호출·P2.1 구현을 승인하지 않는다.

2026-11-15 사용자 후속 승인으로 P2.0 PR #47은 개발 기준선으로 병합됐다. [P2.1a Rust Host 구현/검증 기록](p2-1a-agent-run-contract.md)은 별도 Issue #48의 Draft 범위이며, 앞선 P2.0 설계 당시의 미실행 기록과 구분한다. Runtime A/B 선정·실제 Provider·P2.1b/c는 여전히 미착수다.
