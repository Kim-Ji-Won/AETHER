# P2.2b-3 Gate 0 · 첫 Responses 전송 전 검토

[Issue #62](https://github.com/Kim-Ji-Won/AETHER/issues/62) · 설계 리뷰 · 개발자 본인의 Local BYOK · 2026-11-19

## 1. Executive Summary

**첫 외부 요청은 BLOCKED다.** 로컬 TLS peer에 연결해 자원을 해제한 증거는 있다. 제품 앱이 어느 요청에 동의가 있었는지, 재시작 뒤 어떤 키가 철회 상태인지, 한 번의 전송에 얼마까지 지출할지를 함께 보장하는 증거는 아직 없다. 첫 구현 후보는 키나 소켓 없이 앱의 기존 Host에서 한 개의 provider fixture를 끝까지 처리하는 G1이다.

이 문서에서 **VERIFIED**는 기준 커밋의 소스·메타데이터 또는 명시한 과거 증거, **PROPOSED**는 리뷰할 설계, **OPEN DECISION**은 소유자의 선택이 필요한 값이다. **NOT_IMPLEMENTED**는 제품 경로 구현 부재, **NOT_RUN**은 해당 환경에서 시험하지 않은 항목, **BLOCKED**는 선행 조건 부족으로 실행을 거부하는 상태다.

## 2. Verified Baseline

| 확인 대상 | VERIFIED 근거 |
|---|---|
| Git | GitHub·origin·로컬 main은 `f2b53d7f6aac49bc9f4534580fd8a25ac1a0d11e`. 작업 시작 위치는 이전 feature HEAD `09b1407f942a3955d919f0face9ccd18d402cfe7`이며 두 tree는 같다. 새 브랜치는 squash된 main에서 분기했다. |
| 선행 종료 | #61 MERGED, #60 CLOSED / COMPLETED. 이전 feature의 Rust 164, Frontend 56, network-denied 138, TLS loopback 5는 서로 겹치는 기존 실행 기록이다. [병합 증거](https://github.com/Kim-Ji-Won/AETHER/issues/60#issuecomment-7002164701) |
| 현재 단계 | 문서·소스 대조와 Governance만 수행한다. 제품 앱, Keychain, Provider, billing은 NOT_RUN이다. |

## 3. Existing Contract Audit

| 기준 | 소스에서 확인한 동작 | 제품 통합 시 지킬 조건 |
|---|---|---|
| [P2.0 계약](p2-0-agent-runtime-contract.md) · [Host](../../apps/desktop/src-tauri/src/agent_text/mod.rs) | check_start/start_source가 identity를 다루고 ingest/release/advance_to가 순서·기한·terminal을 결정한다. | 한 attempt, 한 canonical event 순서, 한 terminal. cleanup 상태와 terminal error를 독립시킨다. |
| [P2.2a 계약](p2-2a-native-responses-provider.md) · [decoder](../../apps/desktop/src-tauri/src/agent_text/provider/sse.rs) · [mapper](../../apps/desktop/src-tauri/src/agent_text/provider/mapper.rs) · [오류](../../apps/desktop/src-tauri/src/agent_text/provider/error.rs) | 바이트 단위 UTF-8/SSE framing, 응답·모델·item 결속, text-only payload, 명시적 completion과 usage를 검사한다. 오류는 허용된 코드로 투영한다. | 새 모델이 보내는 미지원 event는 차단한다. 원문 오류나 도구 지시를 renderer로 전달하지 않는다. |
| [b-0 설계](p2-2b-controlled-live-ai-readiness.md) · [b-1 계약](p2-2b-1-native-credential-consent.md) · [resolver](../../apps/desktop/src-tauri/src/agent_text/provider/credential.rs) · [consent](../../apps/desktop/src-tauri/src/agent_text/provider/consent.rs) | DisabledCredentials가 production 기본값이다. 승인 슬롯은 test issuer만 채운다. 본문 Value·identity·credential generation·정책 버전을 비교하고 lease를 빌렸다가 반납한다. 철회 실패는 메모리 latch에 남는다. | 승인한 실제 직렬화 바이트와 native UI, 서명된 항목, 영속 철회 결과를 결속하는 일은 남아 있다. |
| [b-2 계약](p2-2b-2-native-http-transport.md) · [OfflineProvider](../../apps/desktop/src-tauri/src/agent_text/provider/mod.rs) · [전송](../../apps/desktop/src-tauri/src/agent_text/provider/transport.rs) · [HTTP](../../apps/desktop/src-tauri/src/agent_text/provider/transport/http.rs) | OfflineProvider가 library Host를 소유한다. dispatch_http는 live_dispatch에서 ProviderDisabled로 반환한다. loopback 주소와 시험용 CA는 test build 전용이다. | 앱 Host에 adapter만 연결한다. TLS fixture의 승인과 CA가 제품 전송 권한으로 이어지지 않는다. |
| [Ledger](../../apps/desktop/src-tauri/src/agent_text/provider/policy.rs) | UTF-8 바이트를 가상 입력 단위로 쓴다. 메모리에 예약하고 usage를 검증하며 billing_confirmed는 false다. | USD·tokenizer·재시작 내구성은 G5에서 별도로 증명한다. |
| [Lifecycle](../../apps/desktop/src-tauri/src/lifecycle.rs) · [Rust Runtime](../../apps/desktop/src-tauri/src/agent_runtime.rs) · [Shell](../../apps/desktop/src/Shell.tsx) · [UI Runtime](../../apps/desktop/src/agent-runtime.ts) | Shell의 공유 conversation이 Dock과 Diagnostics에 같은 snapshot을 준다. Rust의 Runtime.host, sender gate, Instant 기반 tick은 Synthetic에서 사용한다. | Synthetic sender lease가 native HTTP ingress를 제어할 수 없어야 한다. 제품 provider용 tick·cleanup 통합은 NOT_IMPLEMENTED다. |

## 4. Single Host Integration Architecture

**PROPOSED.** WKWebView → 기존 trusted window·lifecycle 검사 → 앱 Runtime의 유일한 Host → native에서 선택한 source별 adapter → 같은 Host의 snapshot → 기존 Dock/Diagnostics 순서를 유지한다. OfflineProvider의 정책·stream·mapper 부분을 Host를 빌려 쓰는 로직으로 분리하고, 기존 library harness만 자체 Host를 구성한다. 앱 시작 시 두 번째 OfflineProvider/Host를 등록하지 않는다.

1. Native bootstrap에서 instance와 UI epoch를 결속한다. 기존 legacy epoch 0과 wire epoch 1 이상 변환을 유지한다. Host가 예비 request/run/attempt와 session generation을 계산하고 admission에서 확정한다. Renderer의 ID, source 또는 승인 필드만으로 dispatch할 수 없다.
2. Synthetic은 기존 WK sender lease 경로를 유지한다. Provider frame은 owner가 해당 attempt에 붙인 reader에서만 수락한다. 화면은 provider delta·완료·cleanup을 만들어낼 수 없다. native에서 검증한 단일 sequence와 revision 증가분만 공유 snapshot에 투영한다. 출처가 부정확하거나 이전 세대·revision이면 버린다.
3. libcurl easy/multi는 한 native owner에서 생성·poll·해제한다. 앱의 기존 단조 시계로 Host deadline을 진행한다. reader를 별도 owner thread에 둘 경우 bounded 전달 슬롯과 attempt-tagged 해제 확인만 넘긴다. Host lock을 잡은 채 TLS·Keychain·UI 응답을 기다리지 않는다. snapshot 소비 중단 중에도 clock·abort 신호는 전달한다.
4. Main↔Diagnostics 이동은 동일 run을 유지한다. reload는 sender를 폐기하고 기존 attempt를 중단한 후 새 bootstrap/snapshot을 요구한다. Quit은 admission을 닫고 두 owner의 실제 정리를 기다린다. 재시작 후 재전송은 없다. WKWebView 선택은 foreground 개발 경로에 한정된다.

## 5. Native Credential & Keychain Gate

**OPEN DECISION.** 프로젝트 소유자와 macOS 서명 담당자가 아래 값을 승인하고 서명된 격리 빌드에서 증명해야 한다. 저장소에서 관찰한 identifier는 `com.aether.firstwindow`다. 이를 개발용·배포용 최종 ID로 채택했다는 증거는 없다. [Tauri 설정](../../apps/desktop/src-tauri/tauri.conf.json)

| 결정 | 제안과 승인 증거 |
|---|---|
| Team ID, development/release Bundle ID, signing identity | 실제 팀·각 번들·프로파일 대응표를 소유자가 확인한다. 개발/배포 분리와 업데이트 전후 서명 연속성을 비밀 없는 결과로 기록한다. |
| Hardened Runtime, entitlements, Keychain access group | 최종 서명 앱의 effective entitlement와 group을 검사한다. 최소 전용 group, 개발·배포 간 상호 접근 거부, 임의 서명 빌드 거부를 보인다. 그룹 확대나 예외 entitlement가 필요하면 별도 결정한다. |
| Service/account, 저장 및 잠금 | 환경마다 전용 service와 불투명 account 하나를 정한다. data-protection Keychain, device-only·잠금 해제 시 접근·동기화 해제를 제안한다. 명시적 사용자 상호작용을 요구할 연산과 접근 제어를 결정한다. 잠김·Missing·Denied·Unavailable이면 새 lease 0개, 다른 항목 검색·백그라운드 prompt 없이 중단한다. [Apple TN3137](https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains) |
| 등록·조회·회전·폐기 | Native secure entry에서만 입력한다. Resolve는 승인된 attempt의 짧은 native borrow다. Rotate는 admission을 닫고 활성 lease/cleanup 종료 후 성공 시 generation을 전진시킨다. Revoke는 §6 순서로 진행한다. 업데이트·재설치·서명 변경의 잔존/접근 거부는 사용자 주도 조정 대상으로 남긴다. |
| 비밀 수명 | WebView·JS·Node·환경변수·로그·diagnostics·crash capture·clipboard 자동 읽기를 금지한다. Secret/lease를 직렬화·복제·Debug 출력하지 않는다. 소유한 임시 바이트를 지우고 header 복사를 제한한다. OS·libcurl·TLS 복사본, swap, 강제 종료까지 완전 zeroization을 증명할 수 없다. |

**NOT_RUN.** Team·키 목록이나 실제 항목을 조회하지 않았다. 향후 G2의 전용 비밀 아닌 test item, 허용 서명, 생성·폐기 절차 자체도 별도 승인 대상이다. 장기 보관할 실제 키는 G3 통과 전 등록하지 않는다.

## 6. Persistent Revocation Design

**PROPOSED.** Native 전용 durable journal에 schema version, 불투명 credential slot, 단조 generation, operation ID, 철회 단계와 정리 필요 여부만 둔다. 키·키의 지문·승인 토큰·본문·본문 digest·header는 저장하지 않는다. 비용 예약은 §8의 별도 최소 레코드로 관리한다. 저장소 접근통제, 손상·삭제·rollback 감지 수단은 OPEN DECISION이다. 신뢰 가능한 세대 연속성을 증명하지 못하면 기존 키가 있어도 admission을 닫는다.

| 중단 지점 | 명시적 복구와 허용 전이 |
|---|---|
| 철회 접수 / AwaitingCleanup | 같은 직렬화 경계에서 admission을 닫고 미소비 승인을 폐기한다. durable intent를 먼저 확정한 뒤 철회 접수를 응답하고 활성 run을 취소한다. 기록 쓰기 실패는 잠금 유지 및 운영 오류다. Crash 뒤에는 journal과 단일 owner 잠금을 복원하기 전 resolve를 금지한다. |
| Crash 중 미완료 cleanup | 새 instance에는 과거 승인·lease·ACK가 무효다. 재시작만으로 성공 ACK를 생성하지 않는다. 이전 owner 소멸과 해당 로컬 자원 해제를 확인하고 불명확하면 AwaitingCleanup을 유지한다. queued·자원 미생성은 NotRequired, 이미 확인된 cleanup은 재수행하지 않는다. |
| 삭제 진입 / 결과 불명 | 두 cleanup 확인 후 삭제 시도 의도를 먼저 durable 기록하고 정확한 slot만 삭제한다. 실패·응답 유실·삭제 뒤 commit 전 crash는 Failed/Uncertain으로 잠근다. 재시작·재등록·다른 credential 선택으로 해제하거나 자동 삭제 재시도하지 않는다. |
| 명시적 reconciliation | 사용자가 승인한 동일 서명·group·slot에서만 item 유무를 확인한다. 잠금/권한 거부를 부재로 해석하지 않는다. orphan·journal 누락·세대 불일치는 격리한다. 항목이 남았으면 별도의 명시적 폐기 결정이 필요하다. 부재와 정리를 입증해 Confirmed를 durable commit한 후에만 새 명시적 등록을 허용한다. |
| 재등록 / 회전 중 crash | 새 세대를 예약한 intent와 Keychain 변경·commit을 구분한다. 재시작 시 두 기록의 연결이 불확실하면 잠근다. 조정 성공 후 이전보다 큰 generation만 발행하며 과거 consent는 복구하지 않는다. |

G3 증거는 각각의 durable write 전후, 삭제 성공 후 기록 실패, 중복 명령, 재설치 후 고아 항목을 주입한 결정적 시험이다. 미확인 상태의 신규 admission·resolve·자동 retry 횟수는 모두 0이어야 한다.

## 7. Native Consent Gate

**PROPOSED.** Native가 정책으로 만든 최종 직렬화 바이트를 메모리에 고정한다. 신뢰된 foreground 창에는 Provider·정확한 목적지·모델·전송되는 전체 합성 텍스트와 요청 옵션·credential/project의 비밀 없는 범위·최대 예상 비용·취소 한계·보존 요약을 표시한다. 사용자가 확인한 bytes 그대로만 전송한다. 기존 Value 비교만으로 재직렬화 결과를 승인하지 않는다.

승인 슬롯에는 Host instance·UI session/epoch/generation·run/request/attempt·exact bytes·endpoint·model·credential generation·가격/정책 버전·consent 버전·project scope와 만료 시각을 묶는다. native UI의 한 번의 확인만 이를 발급한다. 전송 직전 모든 필드와 예산·취소 여부를 재검사하고 승인 소비와 reservation을 직렬화한다. 입력 편집 후 되돌리기, 모델·정책 변경, 취소, 회전·철회, 만료, reload·재시작은 슬롯을 폐기한다. renderer의 `approved=true`, 모델 출력, 과거 클릭 재생은 승인 권한이 없다.

## 8. Model & Cost Decision Register

**OPEN DECISION.** b-0의 `gpt-5.4-mini-2026-03-17`와 `gpt-6-luna` 및 그 당시 단가·예약 예시는 비교 이력이다. 채택 모델, 가격 버전, 예산 숫자는 미정이다. G5 리뷰어는 시험일에 다음 출처·조회일·프로젝트 적용 범위·정책 버전을 한 기록으로 승인받는다.

| 확인 항목 | 증거와 불확실할 때의 동작 |
|---|---|
| Snapshot, 가용성, 폐기 일정 | 정확한 ID와 해당 프로젝트 허용 여부, alias → 반환 model ID 일치 조건을 확인한다. 날짜 고정 또는 동등한 재현성 기준과 종료일을 증명하지 못하면 차단한다. [Mini](https://developers.openai.com/api/docs/models/gpt-5.4-mini) · [Luna](https://developers.openai.com/api/docs/models/gpt-6-luna) · [Deprecations](https://developers.openai.com/api/docs/deprecations) |
| Responses·SSE·파라미터 | 지원 endpoint, stream·store·background·tools·reasoning·출력 상한 옵션, context/output 한도, event 및 usage schema를 고정 요청과 strict mapper fixture에 대응시킨다. 미지원 event/ID는 승인 없이 완화하지 않는다. [Streaming](https://developers.openai.com/api/docs/guides/streaming-responses) |
| 최악 비용 | 해당 tier/context의 input·cached input·output·reasoning·cache-write 단위를 공식 가격표와 대조한다. cache hit를 전제하지 않고 가능한 최고 비용과 올림을 적용한다. reasoning이 output의 부분집합이면 이중 계산하지 않는다. 미지원 cache-write·모르는 가격·토크나이저/요청 overhead 상한 불명은 차단한다. [Pricing](https://developers.openai.com/api/docs/pricing) |
| 용량과 속도 | 실제 프로젝트의 모델별·공유 RPM/TPM 및 요청당 context/output 한도와 더 낮은 로컬 한도를 대조한다. 정책 변경·만료 시 다시 승인받는다. [Rate limits](https://developers.openai.com/api/docs/guides/rate-limits) |

**PROPOSED.** 정수 최소 화폐 단위와 보수적 올림으로 최대 입력 및 최대 과금 출력·추가 항목을 덮는 USD 상한을 산출한다. 요청별·UTC 일별·월별 한도와 요청/token rate를 한 트랜잭션에서 검사한다. 불투명 고유 attempt, 가격 버전, 시간 창, 예약액, 안전한 집계와 상태만 durable 기록한다. 프로세스 간 동시 요청에도 잔여 한도를 초과할 수 없어야 한다. 가격 유효기간, 시계 역행·일/월 경계와 journal 손실도 시험한다.

전송 의도 기록과 예약 commit이 소켓 열기보다 먼저다. Attempt별 멱등 키로 중복 예약을 차단하되 네트워크 재전송 권한을 주지 않는다. Crash·부분 전송·취소·usage 누락/모순이면 예약을 유지한다. 재시작 시 미정산 예약을 복원하고, 소유자가 프로젝트 과금 근거를 대조해 명시적으로 조정한다. Provider 사용량 보고와 확정 청구액을 별도 필드로 둔다. 일치 근거가 없으면 미확정으로 유지한다.

## 9. Transport & Lifecycle Gate

**VERIFIED** 범위는 libcurl과 IPv4 loopback peer 사이의 정상 완료·취소 시 peer EOF, 307 후속 요청 0건, 미신뢰 CA·잘못된 hostname에서 HTTP 0건이다. **PROPOSED** 제품 경로는 native가 고정 HTTPS POST 목적지에서만 Authorization을 조립하고 사용 뒤 handle과 수명 제한 버퍼를 폐기한다. TLS chain·hostname 검사, redirect/proxy/cookie/netrc/자동 retry 차단 및 닫힌 schema를 유지한다. 실제 외부 endpoint 호출은 G9 이후 별도 1회 시험에만 해당한다.

앱 소유 tick은 consumer credit이 0이어도 run/idle/cleanup 기한과 cancel을 처리한다. 32 KiB 요청, 16 KiB read와 단일 슬롯, frame·stream 정책 상한, 4,096개 event 및 Host 출력 한도를 유지한다. G7은 최종 앱의 owner 경로에서 로컬 제어 peer를 지연·정지시키고 credit을 고갈시켜 추가 read 정지, deadline 선행 처리, UI 비활성·reload·quit 중 취소를 관찰한다. 시험 목적지 주입은 test build 전용이다. G8에서 서명 산출물과 시험 빌드의 차이를 기록한다.

취소 시 새 delta를 거부하고 reader/task/socket 종료 확인 뒤 Adapter ACK, context 회수 뒤 독립 Runtime ACK를 수락한다. Reload 시 기존 Synthetic의 page-replaced 확인만으로 HTTP 해제를 대신하지 않는다. 완료와 두 ACK 이후에만 성공 가능하다. 이미 Failed/ProtocolError면 cleanup 만료는 Pending → Unconfirmed만 바꾸고, CancelRequested에서 첫 terminal은 Failed/CleanupUnconfirmed일 수 있다. late ACK는 cleanup만 갱신한다. DuplicateEvent ingress의 terminal은 ProtocolError 한 번이며 partial output은 보존한다. 로컬 abort 후 원격 처리와 청구는 모두 unknown일 수 있다.

## 10. Privacy & Data Retention

| 결정 대장 | 첫 시험 제안과 확인 주체 |
|---|---|
| 전송 범위 · PROPOSED | 개발자 한 명이 확인한 짧은 합성 text prompt 한 건. 이전 대화, 첨부, 화면, 마이크, Accessibility 데이터, persistent memory를 제외한다. Tools·remote computer control을 끄고 응답은 inert text로만 표시한다. |
| 원격 보존 · OPEN DECISION | 프로젝트 소유자가 실행 직전 조직/프로젝트의 실제 data controls, endpoint·cache 설정, 예외 및 적용 기간을 확인한다. 응답 저장 옵션과 abuse monitoring은 별개다. `store:false`만으로 보존 0일을 약속하지 않는다. 미확인 정책은 G6 차단이다. [공식 Data controls](https://developers.openai.com/api/docs/guides/your-data) |
| 로컬 보존 · OPEN DECISION | 요청·응답 본문, secret·header·raw error를 로그·진단·스크린샷·덤프에서 제외한다. 허용할 증거는 버전, 결과 코드, cleanup 횟수, token/비용 집계다. 접근자·보존 기간·삭제 및 미정산 보관 예외는 소유자가 승인한다. |

**사용자 안내 초안.** “표시된 합성 문구 한 건을 OpenAI의 [목적지]에 [프로젝트 범위]로 전송합니다. 모델은 [고정 ID], 이번 요청의 예상 최대 비용은 [USD 상한]입니다. 이 프로젝트의 데이터 처리·보존 조건은 [확인한 요약]입니다. 취소하면 앱은 수신을 중단하지만 이미 전달된 요청의 처리나 청구가 계속될 수 있습니다. 이 내용으로 한 번 전송할까요?” 실제 표시 시 미확정 칸이 하나라도 있으면 확인 동작을 잠근다.

## 11. Controlled Live Acceptance Matrix

각 행의 증거는 해당 버전의 앱·정책·fixture와 연결한다. 아래 상태는 **제품 경로** 기준이다. 어떤 행도 현재 live 실행을 승인하지 않는다.

| Gate | 필요 구현·판정 증거 | 위험 | 통과 기준 | 실패 시 | 의존성 | 현재 |
|---|---|---|---|---|---|---|
| G1 Single Host | 기존 앱 Host에 메모리 provider 연결, 두 source 경합·위조 sender·reload fixture | 이중 실행·권한 혼동 | 1 admission, source 격리, terminal 1, revision 단조, 기존 Synthetic 회귀 | provider 경로 닫기 | Gate 0 리뷰 | NOT_IMPLEMENTED |
| G2 Credential & Signing | 서명된 개발 앱에서 승인된 전용 test item 등록·잠금·거부·회전·삭제·재설치 시험 | 키 노출·타 앱 접근 | 정확한 group·slot만 접근, 실패 시 lease 0, 안전 코드만 노출 | resolve 차단 | 개발 서명 결정 및 test item 별도 승인 | NOT_RUN |
| G3 Revocation | §6 journal, crash 위치별 결정적 시험과 G2 item 대조 | 재시작 후 철회 우회 | 불명확하면 잠금 지속, 확인된 삭제 뒤에만 재등록 | 격리·사용자 조정 | G2 계약, 최종 확인은 G2 증거 | NOT_IMPLEMENTED |
| G4 Consent | Native preview·issuer, 변경/취소/replay·세대/정책 교체 시험 | 다른 요청 전송 | 동일 바이트·대상·scope·가격으로 1회 소비, 불일치 전송 0 | 슬롯 폐기 | G1·G3·G5·G6 | NOT_IMPLEMENTED |
| G5 Cost | 고정 모델·가격표·tokenizer 대조, durable 원장 동시성·재시작 fixture | 예산 초과·중복 과금 | 최악액 예약, 일·월·rate 초과 거부, unknown 유지 | 비용 admission 차단 | 모델·프로젝트·USD 한도 승인 | BLOCKED |
| G6 Privacy | 프로젝트 정책 확인표, payload allowlist 및 비노출 감사 | 민감정보 전송·잘못된 보존 안내 | 합성 1건만 허용, 실제 보존 조건과 안내 일치 | 동의 발행 차단 | 프로젝트 소유자 결정 | OPEN DECISION |
| G7 Transport | 앱 clock·bounded reader·로컬 peer로 timeout/역압/취소·세대 상실 시험 | 리소스 누수·가짜 ACK | 읽기 정지 중 deadline 유효, peer 종료와 두 ACK 입증, 불확실성 보존 | abort·다음 run 차단 | G1·G4 및 b-2 | NOT_RUN |
| G8 Signed App | 승인 서명 앱의 install/update/quit·restart, 권한·진단·kill switch 점검 | 시험 빌드와 배포 경계 혼동 | 정확한 artifact hash와 서명, fixture 목적지 주입 불가, 재시작 시 차단 | 해당 빌드 격리 | G1–G7 | NOT_RUN |
| G9 One-time Approval | 사용자가 최종 hash·project·모델·합성 본문·비용 상한·만료를 명시 승인 | 무단 전송·재전송 | G1–G8 증거 유효, G4 현재 승인과 운영 승인 모두 유효, 실행 수 최대 1 | dispatch 0, 시도 결과 불명 시 재시도 금지 | G1–G8 및 별도 사용자 승인 | BLOCKED |

## 12. Dependency Graph

```text
Gate 0 리뷰 → G1 ─────────────────────┐
서명/test item 승인 → G2 → G3 ───────┤
모델·프로젝트·예산 결정 → G5 ─────────┼→ G4 → G7
프로젝트 보존 조건 확인 → G6 ─────────┘
G1–G7 증거 → G8 → G9 명시적 일회 승인 → 승인된 실행 1회 이내
```

G3의 메모리 crash fixture와 G5/G6의 정책 검토는 G2 실제 접근을 기다리는 동안 준비할 수 있다. G8 완료 전에 signed 환경에서 검증해야 하는 G2는 별도 승인한 개발용 test item에 한정한다.

G9에서 허용할 개발용 실행 경로는 정확한 서명 산출물과 일회성 승인에 한정한다. 기본 배포 경로의 dispatch는 닫아 둔다. 실행 의도와 비용 예약을 영속 기록한 뒤 최종 native 정책을 한 번 더 대조하고 첫 쓰기를 허용한다. 소켓을 열기 직전 종료되거나 전송 도중 프로세스가 사라져도 같은 승인을 복원하지 않는다. 운영자는 비민감 결과와 미정산 여부를 기록하고 admission이 다시 닫혔는지 확인한다.

## 13. Recommended Implementation Sequence

모든 구현 단위는 **별도 착수 승인 → Issue-first → 독립 worktree → 최소 변경과 결정적 회귀 → 보안 리뷰·Draft PR → 해당 HEAD의 명시적 병합 승인**을 따른다. Gate 통과에는 그 단위의 코드와 §11의 환경별 증거가 모두 필요하다.

1. G1만 요청한다. 메모리 adapter로 두 source와 reload/quit 경합을 시험한다. provider credential, HTTP, UI 변경은 이 단위 밖이다.
2. G2 서명 정책과 test item 계획을 리뷰한 뒤 native resolver를 격리 검증한다. G3 journal을 독립 PR로 검증하고 둘을 대조한다.
3. G5 모델·가격 결정 후 오프라인 원장 PR, G6 프로젝트 보존·데이터 최소화 리뷰를 각각 진행한다.
4. G4 native consent를 승인된 비밀 없는 fixture로 먼저 시험한다. G7에서 앱 timer·transport·cleanup을 로컬 peer로 검증한다.
5. G8은 최종 서명 산출물로 운영 복구를 점검한다. G9는 구현 PR과 독립된 사용자 실행 결정이다. 자동 실행·다음 Issue 생성은 없다.

## 14. Open Decisions

| 결정권자 | 리뷰할 차이 또는 미결 사항 | 해소 조건 |
|---|---|---|
| Host·보안 리뷰어 | b-0는 단일 앱 Host를 요구하고 b-2는 library Host로 외부 접근을 막았다. 앱 timer는 Synthetic 전용이다. | G1의 source·sender·단일 terminal과 G7의 독립 tick/두 ACK 증거 |
| 개발자·서명 담당자 | Local BYOK는 개발 경로다. 정확한 Team/Bundle ID·identity·group·잠금 정책, 배포 방식은 미확정이다. | G2 범위와 전용 항목 승인. 영구 등록 전 G3 crash·rollback·고아 항목 복구 증거 |
| 보안·앱 리뷰어 | 현재 consent는 구조화된 본문을 비교하고 resolve 직후 lease를 반납한다. 실제 바이트 및 Authorization 수명 계약이 필요하다. | G4 immutable bytes·가격/consent 버전 결속, G7 짧은 native lease/header 해제 증거 |
| 프로젝트 소유자 | 모델 snapshot·가격·rate·실제 예산, 원장 보관 기간·조정 권한, 프로젝트 데이터 정책은 미정이다. | G5·G6 서면 결정, 변경 시 동의와 G9 무효화 |
| 사용자 | 실제 단 한 번의 전송 허용 여부 | 특정 서명 artifact와 정책에 한해 G9 승인. 시간 경과나 설계 수용으로 대체하지 않는다. |

## 15. Explicit Approval Boundaries

허용 범위는 이 문서, 정적 대조, Governance 검증과 Draft PR 제출까지다. `ProviderDisabled`를 유지한다. 코드·dependency·IPC·entitlement 변경, 키 등록·Keychain 접근, 실제 외부 요청·과금, P0 선정, 후속 Issue 생성, Ready 전환·병합은 이번 권한 밖이다.

## 16. Rollback / Recovery Considerations

Gate 0만 철회할 경우 문서 한 건을 별도 리뷰로 되돌릴 수 있다. 후속 구현의 운영 중단은 신규 admission 잠금 → 승인 폐기 → 활성 attempt abort → 두 owner cleanup 확인 순이다. 로컬 kill switch는 원격 취소나 환불을 보장하지 않는다. 미확인 철회·예약은 격리하여 소유자가 조정한다. 구버전으로 돌아갈 때도 journal을 이해할 수 없으면 live를 닫는다. 파일 삭제·재설치로 잠금을 초기화하지 않는다. 첫 시험에서 호출 수·청구·비밀 비노출 중 하나라도 불명확하면 재호출 없이 증거를 보존하고 리뷰한다.

## 17. P0 Preservation

**VERIFIED.** 작업 전 고정 packaged app과 원본 289개, 추가 64/14/48개 evidence 및 manifest 해시를 기존 기준과 대조했다. 기존 worktree와 미커밋 작업을 보존한다. E5b/E5c는 PARTIAL, E5d ADR은 PROPOSED, Global Runtime Selection은 NO다. Gate 0 검증은 문서 일관성·기존 계약·보안 경계·민감정보·diff·Governance·whitespace와 PR CI다. Rust 전체·Frontend·TLS loopback·network-denied·packaged GUI는 이 문서 HEAD에서 NOT_RUN이다.
