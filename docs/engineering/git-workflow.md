# Git 작업 흐름 · Git Workflow

## Trunk-based Development

일반 작업은 실제 Issue에서 시작합니다. 검토된 main 기준점에서 짧은 topic branch를 만들고 장기 develop 브랜치나 무관한 기능 묶음을 피합니다. main은 검토·통합 검증된 기준점을 지향하지만 보호 설정은 아직 활성화되지 않았습니다. 최초 README 커밋만 승인된 초기화 예외이며 Issue #1에 SHA를 기록했습니다. 이후 거버넌스·앱 개발 모두 main 직접 커밋을 금지합니다.

브랜치 형식:

- `feat/<issue>-<description>`
- `fix/<issue>-<description>`
- `spike/<issue>-<description>`
- `chore/<issue>-<description>`
- `docs/<issue>-<description>`
- `test/<issue>-<description>`
- `security/<issue>-<description>`
- `release/<version>` — PR에는 실제 Issue를 연결합니다.

실제 Issue 번호와 lowercase kebab-case 설명을 사용합니다. 공유 브랜치 force-push, 이력 재작성, 미병합 작업 삭제는 금지합니다.

## 커밋·PR · Conventional Commits

형식은 `type(scope): imperative summary`입니다. type은 feat, fix, docs, test, refactor, perf, build, ci, chore, security를 사용합니다. type(scope)은 영어로 유지하고 요약은 한국어 또는 영어로 명확하게 작성합니다. 예: `docs(repo): 한영 혼합 문서 기준 적용`.

의미 있는 단위로 커밋하고 정확한 파일 경로만 stage합니다. staged diff를 검토하고 lockfile은 추적하되 비밀과 산출물은 제외합니다. 미완료 작업은 Draft PR로 시작합니다. 부분 작업은 `Refs #N`, 병합으로 Issue 전체를 해결할 때만 `Closes #N`을 사용합니다.

최신 head SHA, 완료 조건, 필수 CI·로컬 검사(또는 명시적으로 승인된 예외), 보안 검토, 중요한 지적 해결을 확인하고 사용자에게 명시적 병합 승인을 받습니다. 리뷰 댓글은 정식 승인이 아니며 CI 성공도 병합 허가가 아닙니다. squash 제목도 Conventional Commits를 따릅니다. 승인된 병합 후 main SHA와 로컬·원격 동기화를 검증합니다.

## 관찰한 설정과 승인 필요 항목 · Configuration

2026-11-06 P0-G 점검: Public, main 초기화 완료, ruleset 없음, main 보호 없음, Squash/Merge/Rebase ON, Auto-merge OFF, 자동 head 삭제 OFF. admin 권한이 보고됐지만 설정은 변경하지 않았습니다. 상태는 **POLICY_DOCUMENTED_NOT_TECHNICALLY_ENFORCED**이며 활성화 전에 실시간 상태를 다시 확인합니다.

제안: Squash ON, Merge commits OFF, Rebase OFF, Auto-merge OFF. 자동 head 삭제는 필요한 작업을 지우지 않는지 확인한 뒤 ON을 검토합니다. 이 문서는 브랜치 삭제나 설정 변경 승인이 아닙니다.

main ruleset 제안: `refs/heads/main` 대상 PR 필수, force-push·삭제 차단, 대화 해결 필수, PR 요구로 직접 변경 제한, bypass 최소화·기록. 현실적인 리뷰어 계획 전에는 독립 승인 수를 0으로 둡니다. 서명 준비를 검증하기 전 signed commit을 강제하지 않습니다. 실제 `governance` 검사가 안정적으로 성공한 뒤 정확한 check identity를 필수화하며 계획 단계의 앱 검사를 필수화하지 않습니다.

관리자의 정상 병합 경로를 확인하고 모든 병합을 막는 무조건 update 제한을 설정하지 않습니다. enforcement mode·bypass 범위·증거를 기록합니다. 문서만으로 main을 보호할 수 없습니다.

## 라벨 제안 · Labels

기존 이름을 비교하고 없는 라벨만 생성합니다. 없는 라벨을 할당하지 않으며 기본 라벨은 보존합니다. 이번 bootstrap에서는 라벨을 관리하지 않았습니다. 아래는 수동 설정 목록이며 색상은 #을 제외한 값입니다.

| Labels | Color | 설명 |
|---|---|---|
| type:feature, type:bug, type:research, type:security, type:chore | 1d76db | 작업 유형: 해당 suffix |
| area:desktop, area:ui, area:agent, area:voice, area:memory, area:native, area:security, area:release, area:infra | 5319e7 | 컴포넌트: 해당 suffix |
| priority:p0 | b60205 | 차단 또는 긴급 우선순위 |
| priority:p1 | d93f0b | 중요 후속 작업 |
| priority:p2 | fbca04 | 일반 또는 보류 작업 |
| status:blocked | b60205 | 명시된 의존성 해결 전 진행 불가 |
| status:needs-review | 0e8a16 | 리뷰 대기, 승인 의미 아님 |

## 마일스톤 제안 · Milestones

없는 제목만 생성하고 임의 번호·기한을 만들지 않습니다. 아래는 납기 약속이 아닌 roadmap 분류이며 관리자 설정·할당을 기다립니다. 영어는 식별용 제목, 괄호는 한국어 설명입니다.

- P0 — Architecture & Feasibility (아키텍처·실현 가능성)
- P1 — Desktop Foundation (데스크톱 기반)
- P2 — Agent Runtime (에이전트 실행 환경)
- P3 — Voice Intelligence (음성 상호작용)
- P4 — Computer Control (컴퓨터 제어)
- P5 — Persistent Memory (영속 메모리)
- P6 — Product UI & Experience (제품 UI·사용 경험)
- P7 — Release Engineering (배포 엔지니어링)

Private Vulnerability Reporting은 별도로 활성화·검증한 후 SECURITY.md와 보안 양식을 갱신합니다. 비공개 행동규범 신고 경로와 라이선스도 결정해야 합니다. 문서 PR은 저장소 관리 설정을 암묵적으로 승인하지 않습니다.
