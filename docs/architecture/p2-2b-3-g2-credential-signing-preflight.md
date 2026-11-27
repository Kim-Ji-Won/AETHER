# G2 · 서명 정책과 전용 Keychain 시험 항목

**준비 문서 완료 · G2 OS 시험 NOT_RUN.** 이 문서는 서명된 macOS 앱이 비밀이 아닌 시험값 하나를 안전하게 생성하고 폐기하는 절차를 리뷰하기 위한 것이다. 실제 접근 전에는 아래 결정표에 소유자가 승인한 값과 시험 산출물의 해시가 필요하다. [준비 Issue #66](https://github.com/Kim-Ji-Won/AETHER/issues/66) · [Gate 0 §5·§6·§11·§13·§14](p2-2b-3-live-integration-preflight.md)

## 확인한 기준과 산출물

2026-11-25, Apple Silicon macOS 26.6 (25G5043d), macOS SDK 26.2. G1의 승인 feature `e70fdd8d138f96224722d1d4f90f596edabc2ea1`과 squash `7a9177e6b04e35fa425e67afa973df83e5d51cd2`의 tree는 `8d4092a1b82dddb68c85fe412734326a762448e4`로 같다. 이 맥에서 clean main을 squash까지 fast-forward했다. P0 고정 앱·원본 289개·추가 64/14/48개는 기존 digest 및 manifest의 각 파일과 대조했다. 기존 worktree는 main 포함 16개였고 그중 두 곳에 미커밋 작업이 있었다. E5b/E5c PARTIAL, E5d ADR PROPOSED, 전역 Runtime 미선정이다. [G1 완료 기록](https://github.com/Kim-Ji-Won/AETHER/issues/64#issuecomment-7155342886)

소스의 Bundle ID는 `com.aether.firstwindow`이고 benchmark는 이 값과 executable 이름을 검사한다. 전용 signing 설정·추적된 profile/entitlements 파일은 발견되지 않았다. Rust에는 `CredentialResolver`, `DisabledCredentials`, test-only mock이 있고 제품 dispatch는 `ProviderDisabled`다. G1의 단일 Host에는 실제 Keychain resolver·secure entry가 연결되어 있지 않다. [Tauri 설정](../../apps/desktop/src-tauri/tauri.conf.json) · [benchmark](../../scripts/benchmark.py) · [resolver](../../apps/desktop/src-tauri/src/agent_text/provider/credential.rs) · [dispatch](../../apps/desktop/src-tauri/src/agent_text/provider/mod.rs) · [G1](p2-2b-3-g1-offline-single-host.md)

기존 기록으로 특정한 P0 고정 앱과 P2.1c 최종 앱 두 개를 실행 없이 조사했다. 두 Info.plist 모두 Bundle ID `com.aether.firstwindow`, executable `aether-desktop`, short version 및 bundle version `0.1.0`이다. [P2.1c 최종 실행 파일 기록](p2-1c-command-dock-conversation.md)

| 대상 | 식별 근거 (SHA-256) | 서명에서 관찰한 값 | macOS strict 검증 |
|---|---|---|---|
| P0 고정 앱 | bundle manifest `9aead5e26afaa248b009604b9b202f58972d66e65ade8d4f28c0a112cfc3a8b9`, Host `7d7cfa83d2cbb88d9b019319120c4b895c45427c7b1ae9c0778ce92891a99d64` | Host flags `0x20002` (ad-hoc, linker-signed), DR은 CDHash 기반, TeamIdentifier 미설정, effective entitlements 출력 0바이트 | bundle 및 번들 안 Host exit 1. 서명이 요구한 리소스 봉인이 없음 |
| P2.1c 최종 앱 | bundle manifest `a6097801a5b3b946896441cc281bc61b58682616024965458c61bd4db2465776`, Host `1c2f4b695fd6f1408ebd3617db04b69c9dbfd2396c0b41febd62bd20fe3a1d7c` | 같은 flags·DR 형식·Team 미설정·entitlements 0바이트. 두 앱 모두 embedded profile과 Host의 application-identifier 및 access-group entitlement 없음 | bundle 및 번들 안 Host exit 1, 같은 리소스 봉인 오류 |
| 두 앱의 Node Mach-O | SHA-256 `64bcb939f5f8c921d104b02d0eb960781c13b49b9133d1d71a768eadd62da401` | identifier `node`, 별도 공급자 Team 표시, Apple anchor·leaf 조건의 DR, flags `0x10000(runtime)`. Keychain group·application identifier 없음 | macOS에서 sidecar 자체 exit 0. 상위 앱에는 별도의 검증 결과 적용 |

Node에서 표시된 effective entitlements는 `com.apple.security.cs.` 아래의 `allow-jit`, `allow-unsigned-executable-memory`, `allow-dyld-environment-variables`, `disable-executable-page-protection`, `disable-library-validation` 및 `com.apple.security.get-task-allow`이고 모두 true였다. Host에 이 예외를 복사할 근거는 없다. 제한된 셸의 첫 Node strict 검사는 exit 1이었고 같은 파일 해시를 대상으로 일반 macOS 실행 맥락에서 다시 검사해 exit 0을 확인했다. 환경별 차이의 원인은 미확인이다. 이 조사로 허용할 AETHER Team·profile·group이나 update 연속성은 확정할 수 없다.

재현에는 `Info.plist`의 위 네 키만 읽는다. `APP`은 기록과 해시로 식별한 번들이다. Host와 Node 각각에도 display·verify를 적용하고 공개 결과에는 DR의 유형과 entitlement 이름만 남긴다. bundle digest는 benchmark와 같이 상대 파일명→SHA-256 사전을 키 정렬한 JSON 바이트의 SHA-256이다.

```sh
codesign -d --verbose=4 "$APP"
codesign -d -r- "$APP"
codesign -d --entitlements - --xml "$APP"
codesign --verify --strict --deep "$APP"
codesign --verify --strict "$APP/Contents/MacOS/aether-desktop"
codesign --verify --strict "$APP/Contents/Resources/sidecar/node"
shasum -a 256 "$APP/Contents/MacOS/aether-desktop" "$APP/Contents/Resources/sidecar/node"
```

## 서명과 저장 정책 결정표

아래 후보는 모두 **PROPOSED**다. 개발·배포 식별자는 각각 소유자가 통제할 namespace에서 따로 발급한다. 후속 시험은 별도 build directory와 시험용 bundle 설정 override로 생성하고 기본 Tauri 설정·benchmark·고정 앱을 보존한다. 완성된 번들의 hash, embedded profile 허용 범위, effective entitlements와 strict 검증을 확인한 뒤에만 항목 연산으로 넘어간다.

| 결정 | 현재 근거 | 권장 후보와 이유 | 시험 전에 확정할 조건 |
|---|---|---|---|
| Team·Bundle ID | 두 앱 모두 Team 미설정. 기본 ID는 P0에도 결속 | 소유자가 관리하는 개발 Team에서 시험 전용 ID 사용. 배포용 ID와 별개로 유지 | 실제 Team, dev/test·release ID, 배포 격리 음성 대조용 산출물 |
| Identity·profile·application identifier | 승인된 매핑 없음 | dev는 Apple Development, 직접 배포는 Developer ID Application 후보. 각 채널의 profile이 해당 signer·App ID·필요 권한을 허용해야 한다 | 서명 담당자가 profile 승인값과 최종 Host의 Team, Bundle ID, macOS `com.apple.application-identifier`, App ID prefix 및 DR을 대조. prefix를 Team 값으로 추정하지 않는다 |
| Group·Hardened Runtime·nested code | Host에는 group·runtime flag 없음. Node는 별도 서명 | Host의 전용 app-private group을 명시해 query. 별도 공유 group이 꼭 필요하면 정확한 1개만 승인. Host에 Hardened Runtime 적용, Node에 credential group 권한 0 | 최종 Host와 각 helper의 개별 verify·권한 비교 후 outer bundle strict/deep verify. dev/release 양쪽 class·group 분리와 불필요한 예외 부재를 확인. 배포는 별도 정책 적용. [Release](../engineering/release-policy.md) |
| 전용 항목 주소와 값 | 기존 test-only mock은 OS 항목을 만들지 않음 | `kSecClassGenericPassword`, dev 시험 group 하나, 전용 service 후보 `aether.g2.dev.test-item`, 무작위 불투명 account 하나. 비밀 아닌 `G2-fixture-v1` → `G2-fixture-v2`로 회전을 관찰 | 정확한 group·service·slot을 비공개 시험표에 고정. 기존 사용자·배포 항목을 접촉하지 않도록 소유자가 예약. 시작 시 충돌하면 중지 |
| 저장소·접근성 | SDK 상수가 존재. 서명 앱에서의 결과 미검증 | 모든 연산에 `kSecUseDataProtectionKeychain=true`, `kSecAttrSynchronizable=false` 지정. 생성 시 `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`. 로그인 사용자 세션, 잠금 해제 시에만 사용 | 기기·OS·잠금/해제 조건과 관찰 방법 승인. device-only는 다른 기기로의 복원 제한이며 동일 기기 재설치 때 제거된다고 가정하지 않는다 |
| 사용자 상호작용 | 실제 native UI 미구현 | 등록·회전·폐기·reconciliation은 명시적 foreground 사용자 동작. resolve는 승인된 단일 attempt의 짧은 borrow. OS 인증 UI 허용 여부는 별도이며 무인 resolve는 noninteractive | 승인할 foreground 연산 및 취소 절차 확정. 필요한 OS 인증이 불가능하면 중지. Background prompt·자동 retry·다른 group/저장소 탐색 금지 |
| 변경·실패·증거 | b-1 latch는 메모리 전용. G3 영속 복구 미구현 | 시험은 하나의 주소에서 Add, CopyMatching, Update, Delete만 허용. 삭제 실패·결과 불명·잔존은 잠금 유지. 공개는 사례 ID, artifact hash, OS/SDK, 분류된 상태, lease 개수·generation 변화·최종 폐기 판정 | 소유자가 격리 기간·폐기 담당자·미완료 인계·로컬 기록 만료 시점 승인. 정확한 slot과 signer/profile 대응은 접근 제한된 로컬 기록에만 보관. 값·lease·계정·인증서/profile 원문·개인 경로·기기 목록은 공개 기록에서 제외 |

## Apple 문서와 현재 SDK 대조

2026-11-25에 Apple 공식 문서 및 `xcrun --sdk macosx --show-sdk-version`으로 식별한 26.2 SDK의 `Security/SecItem.h`, `Security/SecBase.h`, `LocalAuthentication/LAContext.h` 선언을 확인했다.

- macOS의 SecItem은 file-based 및 data-protection 구현을 지원한다. 시험 대상은 로그인 사용자 맥락에서 실제 앱 Host의 서명·profile·권한으로 호출하는 후자다. 일반 CLI, 임시 file-based keychain 또는 mock은 이 경계를 검증할 수 없다. [TN3137](https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains)
- 저장소 선택은 `kSecUseDataProtectionKeychain`으로, 동기화는 별도의 `kSecAttrSynchronizable`로 지정한다. 해당 SDK에서 전자는 macOS 10.15부터 선언되며, 접근성·동기화·group 상수와 네 SecItem 함수도 확인했다. [Data-protection 선택](https://developer.apple.com/documentation/security/ksecusedataprotectionkeychain)
- 전체 query에 class, group, service, account와 위 저장소·동기화 조건을 고정한다. CopyMatching은 최대 1개, 값 반환은 승인된 resolve에만 허용한다. Update는 value만 바꾸고 Delete는 같은 주소에만 적용한다. group 생략이나 동기화 Any는 검색 범위를 넓힌다. [접근 그룹 속성](https://developer.apple.com/documentation/security/ksecattraccessgroup)
- 서명이 허용하지 않은 group은 조회에서 `errSecItemNotFound`로 보일 수 있다. 그 결과는 접근 격리 증거로만 사용한다. 부재나 폐기는 권한이 검증된 원 소유자의 양성 대조와 동일한 주소에서 판정한다. [Apple의 접근 그룹 모델](https://developer.apple.com/documentation/security/sharing-access-to-keychain-items-among-a-collection-of-apps)

## 후속 OS 시험표 · 전 행 NOT_RUN

전제는 별도 승인된 signed fixture, 검증된 owner와 음성 대조용 서명 앱, 예약된 단일 주소, 비밀 아닌 값이다. 실패한 연산에서는 새 lease 0개다. 잠김은 시험자가 통제한 기기 상태와 함께 기록한다. `errSecInteractionNotAllowed` 하나만으로 잠금을 단정하지 않는다. 엔진에는 기존 안전 코드만 전달한다. 원시 오류·항목 내용은 renderer에 노출하지 않는다.

| 사례 | 조건과 조작 | 필요한 관찰과 실패 처리 |
|---|---|---|
| 01 등록·resolve·중복 | 허용 owner가 예약한 빈 주소에 v1 추가, 승인된 attempt에서 1회 resolve. 같은 주소로 재등록 | 일치 여부만 비교 후 lease 반납. 중복은 `errSecDuplicateItem`, 원값·generation 유지. 사전 충돌/다른 값이면 덮어쓰기나 삭제 없이 중지 |
| 02 Missing | 동일 owner가 정확한 미생성 시험 주소 조회 | group 적격성 확인 후 Missing. 새 lease 0, 대체 주소 조회 0 |
| 03 Locked | v1 등록 후 사용자 세션 잠금, noninteractive resolve | 잠금 중 데이터 반환·prompt·lease 0. 제한 상태는 안전한 AccessDenied/Unavailable로 집계. 해제 후 사용자가 새 시험 시작 |
| 04 Denied / Unavailable | 사용자 인증 거절·취소, 별도로 OS 서비스 사용 불가 조건 확보 | 접근 거부와 서비스 불가를 구별. 새 lease·자동 재시도 0. 안전하게 상태를 유도할 수 없으면 해당 OS 사례 BLOCKED. mock 성공으로 대체 금지 |
| 05 활성 lease·cleanup | lease가 살아 있거나 한쪽 cleanup ACK만 도착한 동안 회전·삭제 요청 | 회전 Busy. 철회는 admission 차단·미소비 동의 폐기·cancel/abort를 먼저 진행하되 실제 삭제 0. 두 owner 확인 또는 자원 미생성 NotRequired까지 대기 |
| 06 회전 | lease 반납·cleanup 완료 후 명시적으로 v1→v2 Update | 성공 시에만 generation 증가, 옛 consent 거부, 새 승인으로만 v2 resolve. 결과가 불확실하면 차단·조정 |
| 07 다른 서명·group | owner가 v2를 읽은 직후 허용 group이 없는 다른 서명으로 같은 주소 조회. 동일 Team의 다른 private group도 별도 검사 | non-owner의 반환값·lease 0. access denied 또는 가려진 not-found를 기록한 뒤 원 owner가 v2를 재확인. 다른 서명 자체가 group 차단을 보장한다고 가정하지 않는다 |
| 08 개발·배포 격리 | 승인된 배포형 시험 앱과 개발형 시험 앱 각각의 전용 시험 주소로 양성 대조, 상대 주소에는 교차 조회만 | 자기 fixture만 접근, 두 방향 교차 lease 0. 기존 배포·사용자 실항목 사용 금지. 대조용 서명 산출물이 없으면 BLOCKED |
| 09 업데이트·재설치·서명 변경 | 같은 기기에서 동일 허용 identity/group을 유지한 업데이트·재설치, 별도로 허가되지 않은 서명으로 변경 | 원 owner의 접근 연속성 또는 거부를 구분해 기록. 잔존을 새 등록이나 사용 권한으로 간주하지 않는다. 원 서명 복구 불가 시 폐기 미확인 |
| 10 삭제와 최종 폐기 | 정상 owner의 직전 양성 조회, lease 반납·cleanup 확인, 명시적 Delete 성공, 같은 서명·group·service·account에서 noninteractive exact lookup | 권한·환경이 유지되고 후속 조회가 ItemNotFound여야 시험 폐기 확인. 잠김·서명 변경·삭제 호출 결과만으로 확인 불가 |
| 11 실패·응답 유실·잔존 | 실패/불명 사례는 비밀 없는 fixture로 유도. 재설치 후에도 원 서명에서 정확한 주소로만 명시적 reconciliation | Failed/Uncertain 잠금. 자동 삭제·재등록·해제 0. 잔존하면 별도 폐기 승인, 부재라도 원 소유권·정리 근거 확인. 원 owner에 접근 못하면 미폐기로 인계 |

08의 반대 방향 양성 대조에는 별도 승인된 **배포형 시험 주소 하나**가 추가로 필요하다. 환경마다 정확한 한 슬롯으로 범위를 고정하며 승인된 각 owner가 자기 항목의 최종 폐기까지 책임진다. 음성 대조 앱이 조회한 not-found로 원 owner의 폐기 책임을 해제하지 않는다.

## 후속 native resolver의 최소 경계

기존 `credential.rs`의 trait·borrowed lease와 `consent.rs`의 단일 승인 소비·generation 검사를 재사용한다. 후속 후보 파일 `provider/credential/macos.rs`에 고정된 시험 주소, Security.framework 호출 및 안전 오류 매핑을 한정한다. 별도 signed native fixture가 이 owner를 호출하고 결과 분류와 카운트만 수집한다. 기존 앱·WebView·Node·환경변수·로그로 secret을 넘기는 경로는 없다. Secret과 lease는 Debug·Clone·직렬화·IPC를 금지하고 native 소유 임시 메모리를 반납 시 지운다. [b-1 계약](p2-2b-1-native-credential-consent.md) · [consent 구현](../../apps/desktop/src-tauri/src/agent_text/provider/consent.rs)

Host는 lock 안에서 연산 ID·세대·승인과 편집 가능 여부를 확인하고 예약한다. 전용 native owner가 Host lock 밖에서 OS/UI 응답을 기다린다. 결과 적용 전에 같은 operation·generation·session·철회 상태를 다시 확인한다. stale 결과는 lease로 발급하지 않고, 변경 결과가 불확실하면 조정 전까지 차단한다. 제품 철회에서는 Gate 0 §6의 두 owner cleanup과 삭제 시도 의도의 durable commit이 선행한다. G2의 시험 항목 폐기는 제품의 영속 철회 증거를 제공하지 않는다. 장기 실제 키 등록은 G3 복구 검증 이후의 별도 승인 사항이다.

## 소유자가 결정할 최소 항목

| 담당 | 다음 OS 시험 승인을 위한 입력 |
|---|---|
| 프로젝트 소유자·서명 담당자 | 개발 Team, 시험·배포 Bundle ID와 각 signer/profile/App ID prefix·허용 group 대응. 시험용 서명 앱 및 음성 대조 앱 해시와 업데이트 쌍 |
| 보안 담당자·시험 사용자 | 환경별 예약 group·service·불투명 slot, 비밀 아닌 초기/회전값, 잠금·interaction 정책, 허용 연산과 시험 시간·기기 맥락 |
| 시험 소유자 | 잔존 항목 격리·reconciliation·최종 폐기 담당, 로컬 기록 보관 기한 및 별도 실행 승인 |

현재 승인 가능한 결과는 이 시험 계획과 기존 산출물의 정적 관찰이다. 실제 G2 test-item 전체, secure entry, Keychain·실제 API Key·Provider·billing, 새 서명 빌드·공증·배포는 NOT_RUN이다.

## 2026-11-26 · 이 Mac의 서명 준비 상태

[검증 Issue #70](https://github.com/Kim-Ji-Won/AETHER/issues/70) · 병합 기준 `b15809841e7f3e9f6cc1f680bae920fe8fb8593a` · macOS 26.6 arm64, SDK 26.2. 위 11개 사례의 실제 G2 OS 시험은 모두 **NOT_RUN**이다. 사례 01을 시작할 수 있는 승인된 서명 앱·전용 주소·실행 승인은 아직 없다.

- 기본 Tauri ID와 기존 해시로 식별한 P0/P2.1c 앱의 Info.plist를 대조했다. 두 앱 모두 `com.aether.firstwindow`, `aether-desktop`, 버전/build `0.1.0`이며 앞 절의 두 bundle digest가 일치했다. 기존 display/DR/XML entitlements/strict 명령을 재실행했다. 두 Host는 linker ad-hoc, Team 미설정, CDHash DR, entitlements 없음이다. 번들·Host strict는 리소스 봉인 오류로 **FAIL**했다. 동일 Node는 별도 공급자 Team, Apple anchor·leaf OU DR, runtime flag와 기존 6개 예외를 표시했고 개별 strict는 **PASS**했다. Node에는 Host용 group이 없다.
- 현재 사용자의 기본 검색 범위에서 `security find-identity -p codesigning` 1회로 확인한 identity는 전체 **0**, 유효 **0**이다. `security find-certificate -a -c`는 Apple Development 및 Developer ID Application 이름으로만 제한했다. 두 필터 모두 exit 0·빈 결과였다(집계 및 빈 출력 확인 각 1회씩, 총 4회). 결과는 메모리에서 집계했고 원문을 저장하거나 공개하지 않았다. 다른 계정·검색 목록 밖의 identity 여부는 미확인이다.
- 두 앱에 embedded provisioning profile이 없었다. Xcode UserData와 사용자·시스템 MobileDevice의 표준 provisioning profile 디렉터리 3곳은 존재하지 않았다. 확인 가능한 signer/profile 조합은 **0개**다. 서명 메타데이터 조회 외에 Credential 시험 항목 생성·조회·회전·삭제, 실제 API Key 접근, Provider 요청, 과금은 각각 **0회**다.

아래의 모든 후보는 **PROPOSED**다. 실제 식별자·profile 대응표 및 향후 산출물 해시는 소유자가 확정한 뒤 제한된 로컬 기록에 둔다.

| 결정 항목 | 실제 관찰한 상태 | 근거 | 추천 후보와 이유 | 사용자 결정 필요 여부 |
|---|---|---|---|---|
| 개발 Team·서명 주체 | 사용 가능한 개발 identity 미관찰 | codesigning 전체/유효 0 | 소유 Team의 Apple Development와 전용 개발 앱으로 검증 | 필요 · Team·서명 담당자 |
| 시험·배포 Bundle ID | 기존 두 앱은 같은 P0 ID 사용 | Info.plist·Tauri·benchmark | 소유 namespace 내 시험 ID와 배포형 대조 ID를 따로 지정. P0 ID 보존 | 필요 · 두 ID |
| Signer·profile·prefix·group | 확인 가능한 profile·대응 없음 | 표준 디렉터리 3곳·번들 2개 | 양 환경의 최종 Host에서 profile의 허용값과 effective entitlement를 대조. App ID prefix를 Team ID에서 유추하지 않음 | 필요 · 각 대응과 권한 |
| 시험 주소 | OS 항목 연산 0 | 사례 01·07·08 | 환경별 app-private group, dev service `aether.g2.dev.test-item`, 배포형 대조 service 후보 `aether.g2.release-test.test-item`, 각 불투명 랜덤 slot 1개. group·service·account 정확 일치 | 필요 · 주소·소유자 예약 |
| 시험값 | 실제 키 사용 0 | 기존 사례 01·06 | 비밀 아닌 `G2-fixture-v1`과 `G2-fixture-v2`로 등록·회전 구별 | 후보 일괄 승인 |
| 잠금·사용자 동작 | OS 잠금 시험 미실행 | 기존 SDK 대조·사례 03·04 | data-protection, 비동기화, WhenUnlockedThisDeviceOnly. 등록·회전·폐기는 명시적 foreground, resolve는 단일 승인에 비대화식. 실패 시 lease 0 | 후보 일괄 승인 |
| 허용 연산 | 제품 resolver 비활성 | b-1·사례 01–11 | 비밀 아닌 주소에 한정한 Add/CopyMatching/Update/Delete. 다른 owner는 교차 조회만, 자동 재시도·범위 확대 없음 | 필요 · 허용 연산·시간 |
| 양성·음성·업데이트 앱 | 확인된 두 Host는 strict 실패, 승인된 대조 쌍 없음 | bundle hash·DR·entitlements | 별도 승인 후 dev owner와 같은 권한의 업데이트, 해당 group 없는 서명 앱 및 배포형 양성 대조 앱 준비. 각자 hash·profile·group 재확인 | 필요 · 산출물·준비 담당 |
| 불확실성·폐기 | 실측 및 담당 지정 없음 | 사례 09–11 | 원 서명·group·slot의 owner가 양성 조회와 최종 부재 확인 담당. 잔존·결과 불명은 잠금·명시적 조정. 교차 조회의 not-found는 폐기 증거에서 제외 | 필요 · 폐기·인계 담당 |
| 로컬 증거 | 조사 중 identity 목록·profile 원문 저장 없음 | 집계 전용 출력 | 시험 소유자·서명 담당자로 접근 제한. 대응표는 폐기·리뷰 완료까지 보관한 뒤 삭제하고 공개 요약만 유지. 미확인 항목은 폐기 담당에게 인계 | 필요 · 접근자·보관 종료 기준 |

다음 승인 단위는 서명 준비와 **비밀 아닌** 전용 항목 시험이다. 개발 identity/profile이 준비되어도 사용자의 특정 산출물·주소 승인이 먼저 필요하다. G3 메모리 fixture의 통과는 서명 앱의 Keychain 부재 확인을 충족시키지 않는다. [이번 G3 Mac 결과](p2-2b-3-g3-offline-revocation.md)
