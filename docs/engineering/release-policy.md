# 배포 정책 · Release Policy

P0-G에서 release, 지원 앱 버전, 서명 pipeline, 배포 자격 증명은 구성하지 않았습니다. 배포 작업에는 별도 승인된 Issue와 명시적 게시 승인이 필요합니다. 병합 승인은 release 승인이 아닙니다.

초기 후보는 Apple Silicon용 Developer ID 서명·공증 DMG입니다. 출시 전에 Hardened Runtime, nested code/sidecar 서명, 공증 결과, stapling, 깨끗한 기기 설치, 권한, Keychain과 서명된 업데이트를 검증합니다. 이는 준비 완료 증거가 아닌 요구사항입니다. Mac App Store 및 entitlement 적합성은 미검증입니다.

서명·업데이트 키는 저장소와 PR workflow 밖에서 보호합니다. 비밀을 출력하거나 production 환경을 불신 코드에 제공하지 않습니다. source SHA, toolchain/lockfile 버전, artifact hash와 검증 증거를 기록합니다. Apple 서명과 update 서명을 구분하고 변조 거부·업데이트 중단·schema 호환성을 시험합니다.

게시 전 개인정보 안내·보존/삭제·의존성 라이선스·제품 라이선스·지원/보안 경로·복구를 검토합니다. 이전 바이너리가 새 schema를 읽지 못할 수 있으므로 rollback은 migration을 고려해야 합니다. 하위 호환 migration과 검증된 backup/recovery를 우선하며 자동 rollback을 증거 없이 약속하지 않습니다. 자동 게시·release 삭제·이력 재작성은 금지합니다.
