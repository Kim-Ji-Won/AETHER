# 아키텍처 검토 · Architecture Evidence

승인된 production 아키텍처는 없습니다. 현재 후보는 Tauri/React, Rust–Swift native 연동, TypeScript orchestration, OpenAI Responses/Realtime, SQLite, Playwright, Three.js/R3F입니다. WebView/sidecar 런타임 배치, native bridge, 권한과 배포는 통제된 실험으로 검증해야 합니다.

검토는 실제 research Issue를 연결하고 목적·선택지·정확한 환경/버전·실험 범위·성공/실패 조건·관찰 증거·한계·후속 ADR을 기록합니다. 주장은 OBSERVED, DOCUMENTED, INFERRED, UNKNOWN, BLOCKED로 구분합니다. 개인 host 정보·민감 경로·자격 증명을 공개 증거에 복사하지 않습니다.

선택은 [ADR](../decisions/README.md)에 기록합니다. API 존재, OS 권한, 배포 정책 적합성, 서명된 앱의 실제 성공은 서로 다른 검증입니다. P0-B 실험은 구체 범위가 승인되기 전까지 시작하지 않습니다.

## E4a 설계 문서

- [RFC 001 — AETHER 통합 시스템](rfc-001-aether-system.md) · PROPOSED. E0~E3b 근거 감사, 모듈·권한·데이터 경계와 후보 선택 조건.
- [E4 Runtime 배치 비교 계획](e4-runtime-comparison-plan.md) · NOT_RUN. 동일 Fake Provider로 WebView / bundled Node를 비교하는 절차.
- [개발 로드맵](../engineering/development-roadmap.md) · PROPOSED. P0~P7의 선행 조건과 완료 기준.

[E4a Issue #15](https://github.com/Kim-Ji-Won/AETHER/issues/15)는 문서 준비 단계입니다. 실험 결과와 별도 승인 후에 [ADR](../decisions/README.md)로 결정을 기록합니다.
