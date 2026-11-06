# 아키텍처 결정 기록 · Architecture Decision Records

실제 Issue에 기반한 결정이 있을 때만 다음 미사용 번호의 `NNN-short-title.md`를 만듭니다. 빈 ADR 파일이나 승인되지 않은 ACCEPTED 결정을 만들지 않습니다.

필수 항목은 제목·번호, 상태, 배경, Issue/PR, 선택지, trade-off, 제안, 날짜·출처가 있는 증거, 보안·개인정보 영향, 남은 불확실성, 필요한 검증, 복구·migration 영향입니다.

상태는 PROPOSED, DEFERRED, BLOCKED, ACCEPTED, SUPERSEDED를 사용합니다. ACCEPTED에는 충분한 증거와 Issue/PR에 기록된 명시적 관리자 승인이 필요합니다. 과거 근거를 지우지 말고 대체 결정을 연결합니다.

후보 주제는 desktop framework, agent runtime 배치, macOS native 연동, 로컬 저장, provider/credentials, 승인 모델, 배포, 백그라운드 실행입니다. 아직 승인된 ADR은 아닙니다.
