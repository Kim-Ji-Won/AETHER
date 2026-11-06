# 검증 정책 · Testing Policy

## 현재 검사 · Current Checks

저장소 루트에서 `python3 scripts/validate_governance.py`와 `git diff --check`를 실행합니다. validator는 Python 표준 라이브러리만 사용합니다. JSON은 YAML의 부분집합이므로 `.yml` 파일을 JSON 형식으로 작성했습니다.

필수 파일, Markdown 로컬 파일 링크, Issue form 구조, 일부 workflow 안전 조건, 텍스트 형식과 민감 파일명 패턴을 확인합니다. anchor·외부 URL 가용성·전체 YAML/schema·포괄적 secret scan·앱 동작은 검증하지 않습니다.

CI job은 `governance` 하나입니다. main 대상 PR(본문·제목 수정 포함)과 main push에서 실행합니다. PR 제목·브랜치·Issue 연결 형식도 확인하지만 실제 Issue 완료나 승인을 증명하지 않습니다. 존재하지 않는 앱 명령을 실행하지 않습니다. 실제 검사가 안정적으로 성공한 뒤 승인된 ruleset 변경으로만 필수화합니다.

## 증거 기준 · Evidence

실제 명령, 필요한 환경·버전, exit 결과와 검증 SHA를 기록합니다. PASS / FAIL / BLOCKED / NOT_RUN을 구분하고 실패·보류 이유 및 승인된 예외를 명시합니다. mock/unit 결과와 signed-build/manual integration 결과를 구분합니다. 정적 검사로 UI·권한 PASS를 주장하지 않습니다.

향후 check 이름은 `lint`, `typecheck`, `unit-tests`, `rust-check`, `swift-check`, `desktop-build`, `security-check`이며 아직 계획입니다. 실제 구현과 관련 실패 검증이 준비될 때 도입합니다. 보안 변경은 미승인 호출·권한 거부/철회·replay·race condition을 확인합니다. 단순 내용을 복제하는 불필요한 테스트 대신 변경에 맞는 검증을 수행합니다.

PR CI는 임시 Linux runner, contents 읽기 권한, SHA 고정 checkout, persist-credentials 비활성화, production secret·배포 단계 없음, 5분 timeout을 사용합니다. 불신 코드를 높은 권한으로 실행하는 `pull_request_target`으로 바꾸지 않습니다. workflow 변경은 보안 검토 대상입니다.
