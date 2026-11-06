## 요약 · Summary

문제와 변경 후 동작을 설명합니다.

## 관련 Issue

Refs #<실제 번호>

병합으로 Issue 전체가 완료될 때만 Closes를 사용합니다.

## 범위와 제외 사항 · Scope

## 아키텍처 영향 · Architecture

ADR을 연결하거나 불필요한 이유를 설명합니다.

## 보안·개인정보 영향 · Security / Privacy

비밀·개인정보·exploit 상세를 포함하지 않습니다. 권한·데이터 경계를 설명합니다.

## 구현 내용 · Changes

## 검증 증거 · Testing Evidence

실제 명령·검증 SHA·PASS / FAIL / BLOCKED / NOT_RUN을 기록합니다.

## 실패·보류 검사 · Deferred Checks

한계와 명시적으로 승인된 예외를 설명합니다.

## 화면 증거 · Screenshots / Recordings

UI 변경 시 필요한 비식별화 자료를 첨부하고, 해당 없으면 명시합니다.

## 리뷰 체크리스트 · Review

- [ ] Issue 완료 조건과 승인 범위를 충족함
- [ ] 최신 head SHA를 검토했으며 중요한 미해결 지적이 없음
- [ ] 관련 CI·로컬 검사가 통과했거나 예외가 명시적으로 승인됨
- [ ] 자격 증명·개인정보·불필요한 산출물이 없음
- [ ] 아키텍처·보안 영향과 복구 방안을 검토함
- [ ] 병합 전에 명시적 사용자 승인을 받음 (CI 성공만으로 승인되지 않음)

## 복구 방안 · Rollback / Recovery

안전한 복구와 migration 제약을 설명합니다. 파괴적 정리를 전제하지 않습니다.
