# 개발 로드맵 · Development Roadmap

**PROPOSED** · 2026-11-11 · [E4a Issue #15](https://github.com/Kim-Ji-Won/AETHER/issues/15) · [RFC 001](../architecture/rfc-001-aether-system.md)

완료 상태는 아래에 링크한 기록 범위에 한합니다. 아래 제안 ID는 GitHub Issue 번호가 아닙니다. 각 작업은 선행 조건과 범위를 확인하고 한 단위씩 별도 Issue 및 승인으로 진행합니다. 일정과 완료율은 정하지 않았습니다.

## P0 — Architecture & Feasibility

| 완료 단계 | 증거와 범위 |
|---|---|
| P0-G | [Issue #1](https://github.com/Kim-Ji-Won/AETHER/issues/1) / [PR #2](https://github.com/Kim-Ji-Won/AETHER/pull/2). 문서·governance 초기화 |
| E0 | [Issue #3](https://github.com/Kim-Ji-Won/AETHER/issues/3). Apple Silicon Swift/Node, 후속 Rust 실행 |
| E1a / E1b-1 | [Issue #4](https://github.com/Kim-Ji-Won/AETHER/issues/4) / [PR #5](https://github.com/Kim-Ji-Won/AETHER/pull/5), [Issue #6](https://github.com/Kim-Ji-Won/AETHER/issues/6). 첫 창·arm64 앱과 로컬 DMG 복사 실행 |
| E2a / E2b | [Issue #7](https://github.com/Kim-Ji-Won/AETHER/issues/7) / [PR #8](https://github.com/Kim-Ji-Won/AETHER/pull/8), [Issue #9](https://github.com/Kim-Ji-Won/AETHER/issues/9) / [PR #10](https://github.com/Kim-Ji-Won/AETHER/pull/10). C ABI와 제한 IPC |
| E3a / E3b | [Issue #11](https://github.com/Kim-Ji-Won/AETHER/issues/11) / [PR #12](https://github.com/Kim-Ji-Won/AETHER/pull/12), [Issue #13](https://github.com/Kim-Ji-Won/AETHER/issues/13) / [PR #14](https://github.com/Kim-Ji-Won/AETHER/pull/14). AX 상태·프로세스 내부 승인 카운터 |

| 작업 / 사용자 가치 | 선행 | 구현 범위 / 제외 | 산출물 | 검증과 완료 조건 | 승인·보안 영향 |
|---|---|---|---|---|---|
| E4a · 리뷰 가능한 설계 초안 | E0~E3b | 근거 감사·문서 / 앱 실행·변경 제외 | RFC, 실험 계획, 로드맵, 진입 문서 | SHA·링크·diff·HEAD governance 확인과 Draft 제출 | 문서·실험·아키텍처 승인 분리 |
| E4b 제안 · 화면에서 작업 진행 확인 | E4a 검토 및 별도 착수 승인 | A 정상 Fake Provider 1건 / 실제 AI·실패 실험 제외 | 공통 frame 초안과 A trace | 패키징 앱의 동일 run progress와 terminal 1건 | 합성 데이터, local-only IPC |
| E4c 제안 · 같은 작업을 창과 독립 실행 | E4b, bundled Node 범위 승인 | B 정상 1건·외부 복사본 / 서명·외부 네트워크 제외 | bundled fixture와 A/B parity trace | 전역 Node 없는 동일 결과와 child 경로 검증 | 바이너리 출처·버전·해시, 최소 권한 검토 |
| E4d 제안 · 취소·재전송·입력 오류 안전 처리 | E4b/c | cancel·late·malformed·duplicate / 실제 도구 제외 | 공통 실패 테스트·trace | terminal 한 번, 잘못된 요청에서 effect 0 | 승인·IPCs의 schema와 범위 유지 |
| E4e 제안 · 창·프로세스 실패 뒤 상태 확인 | E4d | reload·close·child crash·Quit·승인 대기 중단 / 실사용 데이터 제외 | 두 후보 lifecycle evidence | orphan과 승인 재사용 0, snapshot 일치, 부모 창 종료 검증 | 합성 승인, 테스트 PID만 종료 |
| E5 제안 · 긴 작업·성능 비교 | E4 evidence | 반복 lifecycle·RSS/CPU/GPU·energy·latency / provider 호출 제외 | p50/p95·실패율·artifact 크기, ADR 제안 | 합의 예산·안전 gate, 재현 가능한 비교 후 명시 결정 | Runtime 결정은 별도 승인 |

세부 절차는 [E4 계획](../architecture/e4-runtime-comparison-plan.md)입니다. E4a가 E4 전체 완료를 의미하지 않습니다. E5에서 성능과 지속 요구가 충돌하면 요구·후보를 재검토합니다.

## P1 — Desktop Foundation 및 첫 A+B 3D Core

| 작업 / 사용자 가치 | 선행 | 구현 범위 / 제외 | 산출물 | 검증과 완료 조건 | 승인·보안 영향 |
|---|---|---|---|---|---|
| P1.1 제안 · 탐색·기능 상태 파악 | E1a 및 UI 범위 승인 | Main/Command Center/Memory Center/Settings shell, mock 표시 / 실제 AI·DB 제외 | 창과 경량 화면, 상태 fixture | 키보드·resize·hide·reopen, 실제/Mock 명확, 기존 진단 회귀 | capability 확대 없이 UI 독립 진행 가능 |
| P1.2 제안 · A+B가 결합된 실제 첫 3D 장면 | P1.1, 렌더러·버전·라이선스 승인 | 다중 궤도 링·Neural Plasma, 8개 mock 상태, 2D fallback / 실시간 음성·고급 효과 제외 | 패키징된 3D Core, reduced-motion·경량 옵션 | 가시·숨김·context failure·resize 실제 앱 확인, 자원 기준선 기록 | GPU·접근성 검토, 개인정보 미사용 |

## P2 — Agent Runtime

| 작업 / 사용자 가치 | 선행 | 구현 범위 / 제외 | 산출물 | 검증과 완료 조건 | 승인·보안 영향 |
|---|---|---|---|---|---|
| P2.1 제안 · 텍스트 작업 진행·취소 표시 | E4/E5 결정, P1.1 | Rust ledger, Runtime·ordered events, Fake Provider / 실제 AI 제외 | Command Center 상태 연결과 진단 | reload·cancel·late result 후 UI와 host 일치, Mock 명시 | 좁은 IPC와 timeout, 민감 로그 제외 |
| P2.2 제안 · 첫 실제 텍스트 응답 | P2.1, provider·credential·비용 승인 | Responses adapter, stream/cancel, 개발 BYOK 또는 제한 backend / 도구 실행·음성 제외 | auth·quota·오류·폐기 경로 | 합성 질문 실제 응답, 키 비노출, 401/429/offline·rotation 검증 | 외부 데이터 전송 설명·동의, 운영 키 WebView 금지 |
| P2.3 제안 · 제한 사용자용 연결 | P2.2, backend 운영 결정 | 앱 인증·사용량 제한과 서버 키 회전 / 공개 출시 제외 | Managed Backend 최소 경로·운영 문서 | 만료 로그인·과한 요청 거부, 로그/비용/장애 복구 | 서버 운영·개인정보 승인, Axiom 의존성 없음 |

## P3 — Voice Intelligence

| 작업 / 사용자 가치 | 선행 | 구현 범위 / 제외 | 산출물 | 검증과 완료 조건 | 승인·보안 영향 |
|---|---|---|---|---|---|
| P3.1 제안 · PTT 장치와 권한 상태 | P1, 별도 시스템 권한 실험 승인 | WKWebView와 Swift audio를 나눠 패키징 비교 / 실제 클라우드 전송 제외 | 장치 시작·정지, 권한 상태 표시 | 실제 마이크 허용·거부·철회, 장치 교체·Quit 후 capture 0 | 사용자만 권한 조작, 목적 문구와 데이터 범위 |
| P3.2 제안 · 한국어 음성 응답과 Core 반응 | P3.1, P2.2/2.3 인증 경로 | PTT/Realtime, envelope, 발화 종료·끼어들기 / Wake Word 제외 | Core listening/speaking, 회복 가능한 voice session | 한국어+영어 기술어 fixture, echo·barge-in·재연결·만료·Quit 검증 | 단기 인증, 원음 미보존, 전송 동의 |

## P4 — Computer Control

| 작업 / 사용자 가치 | 선행 | 구현 범위 / 제외 | 산출물 | 검증과 완료 조건 | 승인·보안 영향 |
|---|---|---|---|---|---|
| P4.1 제안 · 실제 OS 접근 상태 안내 | E3a, 권한 실험 별도 승인 | AX 및 필요한 권한만 허용·거부·철회 검사 / 광범위한 제어 제외 | 패키징별 권한 결과와 접근 차단 | 패키징 앱 실제 허용/철회에서 상태 갱신·접근 중단 | OS 설정은 사용자 동의, 화면 캡처는 필요한 기능에 별도 승인 |
| P4.2 제안 · 승인 대상과 만료 확인 | E3b, E4e, P2.1 | native 승인 ID·TTL·revalidation·자기 승인 방지 / 일반 도구 제외 | Rust broker와 승인 UI·trace | 부모 창 종료, 위조·replay·race, 사용자 직접 승인/자동화 구분, 자기 창 우회 차단 | 외부 자동화 잔여 위험, 고위험 도구는 gate 전 비노출 |
| P4.3 제안 · 제한 도구 한 개와 결과 확인 | P4.1/4.2, 도구별 승인 | 좁은 allowlist 도구 1개, receipt/reconcile / 무제한 shell·외부 송신 제외 | Command Center 실행·취소·결과 확인 | 합성 대상, path·권한 재검사, crash 시 outcome_unknown·자동 재실행 0 | 정확한 인자·대상 결합, 개인정보 최소화 |

## P5 — Persistent Memory

| 작업 / 사용자 가치 | 선행 | 구현 범위 / 제외 | 산출물 | 검증과 완료 조건 | 승인·보안 영향 |
|---|---|---|---|---|---|
| P5.1 제안 · 선택적 저장·조회·수정·삭제 | P1.1, SQLite·암호화 범위 결정 | Rust DB, 로컬 CRUD·출처·보존 / 클라우드 동기화 제외 | Memory Center 실제 연결, migration 계획 | 빈 DB·업그레이드·rollback, 권한·쓰기 실패·보존 검증 | 로컬 보호·키 복구·삭제 범위 사용자 표시 |
| P5.2 제안 · 삭제가 검색과 복구에도 반영 | P5.1 | 파생 데이터 삭제, backup/export/restore / 원격 embedding 제외 | 삭제 증거와 백업 복원 절차 | 삭제 후 원본·요약·인덱스 검색 0, 복원 시 삭제 정책 적용 | 사본·WAL·FTS 잔여 위험과 사용자 관리 |

## P6 — Product UI & Experience 고도화

| 작업 / 사용자 가치 | 선행 | 구현 범위 / 제외 | 산출물 | 검증과 완료 조건 | 승인·보안 영향 |
|---|---|---|---|---|---|
| P6.1 제안 · 상태와 회복 행동을 읽기 쉬운 화면 | P1.2, 연결된 P2~P5 기능 | 실제 상태·오류·offline·대기 시각 개선, 동작 줄이기 / 새 권한 기능 제외 | 일관된 네 화면, 접근성 검토 | 키보드·읽기·reduced motion·저사양 fallback 실제 앱 확인 | UI가 host 상태만 투영, 승인 문구 가독성 검증 |
| P6.2 제안 · Core 품질과 에너지 균형 | P6.1, E5 기준선 | 효과·DPR·frame pacing 개선 / 사용자 설정 밖의 진단 수집 제외 | 모드별 실측과 품질 비교 | p95 frame·RSS·GPU·배터리 합의 예산, 숨김 렌더 정지 | 비식별 로컬 측정 |

## P7 — Release Engineering

| 작업 / 사용자 가치 | 선행 | 구현 범위 / 제외 | 산출물 | 검증과 완료 조건 | 승인·보안 영향 |
|---|---|---|---|---|---|
| P7.1 제안 · 신뢰 가능한 설치 | E1b-1, 배포·자격 증명 승인 | Developer ID·Hardened Runtime·nested code, 공증·stapling / 공개 게시 제외 | 해시가 있는 서명 DMG 및 검사 기록 | Gatekeeper와 깨끗한 다른 Apple Silicon Mac, offline staple 검증, 권한·Keychain 재확인 | 빌드/서명 키 분리, 서버·CI 노출 금지 |
| P7.2 제안 · 안전한 업데이트·복구 | P7.1, 저장 호환 정책 | 서명 update 검증·복구, provenance / 자동 게시 제외 | 제한 배포 채널, rollback·migration 안내 | 변조·중단 거부, 데이터 호환·복구 실제 검증 | Apple 서명과 업데이트 키 분리 |
| P7.3 제안 · 사용자용 직접 배포 | P7.1/7.2, 제공 기능별 gate 및 게시 승인 | 테스터→공개 범위 선택, 개인정보·지원·라이선스 / 미검증 기능 제외 | 릴리스 노트, 설치·지원·보안 경로 | 다른 Mac에서 설치→권한→핵심 동작→업데이트/삭제, 증거 기록 | 명시적 게시 승인, 운영 비용·정책 결정 |

P4의 실제 권한 허용·철회와 자기 승인·부모 창 종료가 검증되기 전에는 컴퓨터 제어 노출을 보류합니다. P3의 마이크 권한·세션 종료가 남아 있으면 음성 출시를 보류합니다. P7의 서명·공증·stapling과 다른 Mac 검증은 공개 설치의 gate입니다. P1의 로컬 UI·3D·Mock, P6의 시각·접근성 개선은 해당 위험 기능의 활성화 없이 진행할 수 있습니다. [배포 정책](release-policy.md) · [검증 정책](testing-policy.md)
