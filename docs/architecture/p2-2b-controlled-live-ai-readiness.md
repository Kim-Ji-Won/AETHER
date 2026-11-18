# Controlled Live AI Integration — Architecture & Security Readiness

P2.2b-0 설계 검토안 · [Issue #56](https://github.com/Kim-Ji-Won/AETHER/issues/56) · 2026-11-18

## 1. Executive Summary

첫 원격 시험의 단위는 격리된 개발 프로젝트에서 사용자가 확인한 짧은 텍스트 한 건이다. 목표는 기존 Command Dock과 Rust Host의 취소·부분 출력·종료 계약을 실제 통신에서도 유지하는 것이다. 이 문서가 제안하는 권한은 단계별로 독립적이다. 현재 앱의 Live Admission은 닫혀 있고 Synthetic 경로만 동작한다. 문서 병합 여부와 무관하게 모델, 비용, 자격 증명, 외부 전송은 별도 사용자 결정을 기다린다.

## 2. Baseline & Existing Contracts

설계 기준은 `d62e10f97e9ecab1c045beda1bebcddf38b72a1e`이다. [PR #55](https://github.com/Kim-Ji-Won/AETHER/pull/55)가 squash 병합됐고 [Issue #54](https://github.com/Kim-Ji-Won/AETHER/issues/54)는 완료됐다.

| 기준 | 재사용할 계약 |
| --- | --- |
| [P2.0](p2-0-agent-runtime-contract.md) · [Fixture](p2-0-contract-fixtures.md) | agent.text.v1, 폐쇄형 이벤트, 순서, 정확히 하나의 terminal, cleanup 후 admission |
| [P2.1a](p2-1a-agent-run-contract.md) · [P2.1b](p2-1b-foreground-runtime.md) | Rust Host가 run/request/attempt/session과 단조 시계를 소유한다. 기존 lifecycle gate가 host instance, epoch, generation 및 sender를 검증한다. |
| [P2.1c](p2-1c-command-dock-conversation.md) | Dock과 Diagnostics가 단일 runtime과 revision을 공유한다. 화면 전환은 run을 보존하고 reload는 중단한다. |
| [P2.2a](p2-2a-native-responses-provider.md) · [Rust 구현](../../apps/desktop/src-tauri/src/agent_text/provider/mod.rs) | OfflineProvider는 library 시험에서 Host를 소유한다. 실제 앱의 dispatch는 비활성이다. RequestPreview, SSE parser, typed mapper, safe error와 두 cleanup 확인을 재사용한다. |

## 3. Architecture / Data Flow

1. WKWebView Command Dock은 텍스트와 동작 의도만 기존 AgentRuntime으로 보낸다. Tauri/Rust의 기존 gate가 호출 창·세대·lease를 확인한다. 같은 run을 Diagnostics에서도 snapshot으로 본다.
2. 단일 Host가 중복 요청, 활성 run, cleanup, 입력 크기, 전송 동의, 승인 모델·가격·예산·rate를 검사한다. Native 정책이 source와 endpoint를 정하고 예약을 원자적으로 잡는다. UI의 source 필드나 모델 출력으로 권한을 만들 수 없다.
3. 승인된 경로에서만 native resolver가 macOS Keychain에서 짧은 lease를 얻는다. native transport가 고정 Responses 요청을 구성한다. dispatch 직전 정책 버전·동의·run/attempt와 취소 상태를 재확인한다.
4. Native reader → P2.2a SSE parser → typed mapper → 기존 Host 순으로 처리한다. Reader handle과 attempt는 dispatch 시 결속한다. JSON 안의 ID만으로 sender를 인증할 수 없다. Host가 canonical sequence, partial text, 단 하나의 terminal을 정한다. WKWebView는 최신 revision만 React text node로 표시하고 Synthetic과 원격 출처를 명시한다.
5. Adapter가 socket/reader/task의 중단과 종료를, runtime owner가 해당 attempt 자원 해제를 각각 확인한다. Host가 두 결과와 deadline을 처리한다. P2.2a의 메모리 reader 종료는 실제 socket ACK의 증거가 될 수 없다. 앱에 OfflineProvider의 Host를 하나 더 만들지 않고 기존 Host에 adapter를 연결하는 것이 b-2의 수용 조건이다.

## 4. Trust Boundaries & Threat Model

| 경계와 위험 | 통제 및 거부 동작 |
| --- | --- |
| Renderer → Rust. 위조 sender, 오래된 세대, 임의 URL/모델 | 현재 lifecycle binding과 closed schema를 먼저 검사한다. renderer에 범용 network/shell/secret IPC를 제공하지 않는다. |
| 사용자 승인 → dispatch. 입력 교체, 이중 전송 | native 승인을 목적지·project·정책 버전·정확한 전송 본문·1회 attempt에 묶는다. 본문 변경과 재사용은 재승인 대상이다. |
| 원격 → Host. 과대 frame, 잘못된 sequence, 도구 지시 | bounded parser와 text-only mapper가 거부한다. 출력으로 HTML/script/tool을 실행하지 않는다. 기존 accepted event는 보존한다. |
| 앱 → 관측·지원 자료. 비밀·대화 유출 | 허용된 안전 코드, 단계, terminal/cleanup 횟수, 집계 token 수만 기록한다. body/header/raw error, 사용자 본문, lease, provider ID, 메모리 덤프는 수집 대상에서 제외한다. |
| 외부 프로젝트 → 비용. 공유 키, 가격 변경, 재시작 | 격리 프로젝트와 최소 권한, 가격 버전, UTC 예산 원장, 미정산 예약을 함께 검사한다. 불일치 또는 원장 손실 시 admission을 닫는다. |

## 5. Credential Security

개발자 본인 소유의 **Local BYOK**는 키가 로컬 native process에 존재하고 사용자가 OpenAI 프로젝트와 청구를 관리한다. 배포 앱에 공용 개발 키를 넣지 않는다. **Managed Backend**는 API 키를 서버에 두되 사용자 인증·tenant 격리·남용 방지·서버 보존 및 청구 책임이 추가된다. 후자는 별도 아키텍처 리뷰 대상이다. 두 방식 모두 이번 설계에서 채택을 확정하지 않는다.

| CredentialResolver 연산 | 제안 계약 |
| --- | --- |
| Register | 전용 native secure field에서 사용자가 직접 등록한다. WebView 입력·클립보드 자동 읽기·환경변수 fallback을 금지한다. 별도 개발용 service/account에 사용자 확인 후 기록하고 renderer에는 가용 여부만 반환한다. |
| Resolve | 동의·예산 승인 후 한 attempt 동안 native transport만 빌린다. 항목 누락, 잠금, 접근 거부·OS 오류를 안전한 상태로 돌려주고 dispatch를 막는다. 백그라운드 prompt나 다른 저장소 탐색을 하지 않는다. |
| Replace / Rotate | 신규 admission을 잠그고 활성 lease의 정리를 기다린다. 새 값 저장 성공 시에만 버전을 전환하고 기존 승인을 무효화한다. 실패하면 혼합 상태로 dispatch하지 않는다. |
| Revoke | 즉시 로컬 admission 및 미사용 동의를 무효화한다. 활성 작업 취소·cleanup 후 해당 항목만 삭제한다. 삭제 실패는 사용자에게 알리고 계속 잠근다. 이미 유출된 키의 원격 폐기는 프로젝트 소유자가 별도 수행한다. |
| Shutdown | 새 resolve 차단 → 활성 작업 abort/join → lease와 버퍼 해제. 정상 종료만으로 사용자가 등록한 영구 항목을 삭제하지 않는다. |

기존 borrowed lease를 유지하고 Secret/Lease의 Debug·Clone·Serialize 및 IPC 변환을 금지한다. 소유한 byte buffer는 수명 종료 시 지우고 header와 임시 복사를 최소화한다. OS swap, TLS 라이브러리 복사, 강제 종료 시 완전 삭제를 증명할 수 없다. 따라서 crash reporter의 dump·request capture를 끄고 비밀을 로그·진단·canonical event·Git·빌드 산출물·fixture에 싣지 않는다.

macOS data-protection Keychain, 기기 한정·잠금 해제 시 접근, non-synchronizable 항목, 최소 access group을 후보로 둔다. Team ID, bundle ID, signing identity 및 entitlement가 일치하는 설치물만 허용되는지 서명된 전용 test item으로 확인한다. 개발 빌드와 배포 빌드를 분리하고 update/reinstall 후 접근 가능성과 잔존 항목을 시험한다. 서명 변경으로 접근이 끊기면 재등록을 요청한다. 광범위 ACL, 타 앱 항목 조회 또는 자동 migration은 허용하지 않는다. [Apple TN3137](https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains)

## 6. Transport & Streaming Contract

- 허용 목적지는 native가 만든 HTTPS POST `https://api.openai.com/v1/responses` 하나다. 시스템 신뢰 체인과 hostname을 검증하고 3xx를 전부 거부한다. 임의 proxy·URL·query·추가 header, TLS 검증 해제, 쿠키·자동 재시도·SSE 재연결을 두지 않는다. native에서만 Authorization을 구성하고 SDK 기본 재시도도 점검한다.
- RequestPreview의 단일 user/input_text, stream=true, store=false, background=false, tools=[], tool_choice=none, parallel_tool_calls=false와 max_output_tokens를 유지한다. 직렬화 전후 모두 byte cap을 검사한다. 입력 UTF-8·예상 token·전체 요청 크기·출력 token은 각각 제한한다. 모델별 새 필드가 필요하면 allowlist와 fixture 리뷰가 선행한다. [Responses create](https://developers.openai.com/api/reference/resources/responses/methods/create)
- 성공 status와 text/event-stream을 확인한 뒤 제한된 byte chunk를 읽는다. bounded queue가 차면 읽기를 멈추고 소비자를 기다린다. 단일 chunk/frame, 누적 stream, event 수, 출력 byte와 대기 시간에 상한을 둔다. 압축을 허용한다면 해제 후에도 상한을 적용한다. heartbeat로 run deadline을 연장하지 않는다. 한도 초과·소비자 소실은 abort로 전파한다.
- 분할 UTF-8과 CR/LF는 기존 parser에서 재조립한다. comments와 metadata도 byte budget에 포함한다. 최초 sequence는 0 또는 1, 이후 모두 +1이다. response/model, item/index, role/type와 최종 누적 text가 일치해야 한다. 중간 EOF, 단독 [DONE], 미지원 reasoning/refusal/tool은 성공으로 승격하지 않는다. [Streaming](https://developers.openai.com/api/docs/guides/streaming-responses)
- Host의 monotonic run/idle/cleanup deadline을 connect·read·queue wait에도 적용한다. 타이머와 abort는 WebView에 의존하지 않는다. task 종료와 handle 해제를 확인한 owner만 ACK한다. 단절·timeout·취소 후 전송 가능성이 있었다면 remote outcome과 billing은 unknown이다. 원격 중단을 확인하려고 자동 후속 요청을 보내지 않는다.

## 7. Model / Cost Policy Decision Matrix

2026-11-18 공식 문서 기준. 표는 Standard, 짧은 context, 텍스트 100만 token당 USD다. 컨텍스트와 출력은 모델 한계이며 앱 허용량은 따로 낮게 설정한다.

| 후보 / 버전 안정성 | 입력 / cached input / 출력 | Context / 최대 출력 | 호환성과 선택 조건 |
| --- | --- | --- | --- |
| gpt-5.4-mini · 날짜 고정 후보 gpt-5.4-mini-2026-03-17 | 0.75 / 0.075 / 4.50 | 400,000 / 128,000 | Responses streaming 지원. 고정 ID로 현재 mapper의 model 일치를 시험하기 용이하다. 단일 text item·usage 실측 전에는 미검증이다. |
| gpt-6-luna · 모델 페이지에 날짜 고정 ID 미표시 | 0.10 / 0.01 / 0.50 | 1,050,000 / 128,000 | Responses streaming 지원. 낮은 단가지만 기본 reasoning과 cache-write 비용이 있다. 재현 가능한 ID 정책, reasoning 비활성 설정과 mapper 호환성을 먼저 해결해야 한다. |

Luna short-context cache write는 30분 0.125, 12시간 0.25 USD/100만 token이다. 첫 시험에는 long context·추가 tier·유료 tool을 허용하지 않는다. Alias가 반환한 ID가 달라질 때 자동 허용하지 않는다. 공식 가격·지원 종료·project tier·RPM/TPM을 시험일에 재확인한다. 짧은 입력과 낮은 출력 상한은 지연을 줄일 후보 조건이다. 첫 delta 시간·총 소요시간은 향후 시험에서 측정한다. [GPT-5.4 mini](https://developers.openai.com/api/docs/models/gpt-5.4-mini) · [GPT-6 Luna](https://developers.openai.com/api/docs/models/gpt-6-luna) · [Pricing](https://developers.openai.com/api/docs/pricing) · [Rate limits](https://developers.openai.com/api/docs/guides/rate-limits)

**예약 예시.** gpt-5.4-mini에서 요청 형식까지 포함한 입력 상한 256 token, 출력 상한 64 token을 승인했다고 가정하면 캐시 할인 전 예상치는 (256 × 0.75 + 64 × 4.50) / 1,000,000 = **0.000480 USD**다. 예시를 기본 예산으로 사용하지 않는다. UTF-8 byte 기반 fictional unit을 USD로 환산하지 않는다. 승인된 tokenizer·요청 overhead 상한, 모델·tier·캐시 write를 포함한 최악 단가와 올림 방식이 없으면 dispatch를 거부한다.

Host는 정수 금액 단위로 run/UTC 일/월 잔여액과 요청·token rate window를 동시에 검사·예약한다. 동일 request를 이중 차감하지 않는다. 미설정·만료된 가격, 모르는 비용 항목, 한도 초과는 fail-closed다. 성공·실패·incomplete에서 usage가 있으면 cached input은 input의 부분집합, reasoning은 output의 부분집합으로 집계한다. cache-write는 해당 모델의 독립 과금 규칙을 검증한다. 초과·누락·연결 단절 시 안전하게 예약을 보유하고 후속 admission을 차단해 대조한다. 재시작으로 한도가 초기화되지 않도록 비식별 예약·집계만 원자적으로 유지하거나 복구 확인까지 잠근다.

격리 프로젝트의 모델 권한·rate limits·지출 알림과 조직 결제 설정을 소유자가 확인한다. 프로젝트 예산 경보와 로컬 예상 한도는 청구 hard cap을 보장하지 않는다. Provider-reported usage를 받은 때와 소유자가 billing 자료에서 대조한 때를 별도로 기록한다. [프로젝트 운영 가이드](https://developers.openai.com/api/docs/guides/production-best-practices)

## 8. Privacy / Consent

전송 직전 **OpenAI · 대상 프로젝트 · 승인 모델 · 정확한 단일 입력 · 예상 비용과 상한 · 취소 후 과금 가능성**을 표시한다. 명시적 1회 동의를 native에서 확인한다. 사용자가 취소하면 전송 전에는 요청을 버리고, dispatch 후에는 로컬 중단과 미확정 원격 결과를 안내한다. 목적지·credential·모델·가격 정책·본문 변경, 재시작 또는 동의 철회는 기존 승인을 무효화한다.

이전 대화, previous_response_id, 첨부 파일, 화면, 마이크, Memory, 접근성 데이터, diagnostics는 보내지 않는다. 대화 본문과 응답은 메모리에만 두고 로컬 비용 원장에는 재식별 가능한 본문·원격 식별자를 넣지 않는다. 사용자가 보는 partial 출력은 명시적으로 구분한다.

store=false는 Responses의 application state 저장을 제어한다. 기본 abuse monitoring 로그에 고객 콘텐츠가 통상 최대 30일 보존될 수 있고 법적 예외와 조직별 승인 제어가 적용될 수 있다. Prompt caching의 보존 방식·기간과 ZDR 호환성은 모델/옵션별로 다르다. ZDR을 기본값으로 약속하지 않는다. 시험 전 해당 프로젝트·모델의 실제 정책을 확인해 동의문에 반영한다. [OpenAI 데이터 제어](https://developers.openai.com/api/docs/guides/your-data)

## 9. Lifecycle & Error Contract

| 지점 | Host가 보장할 결과 |
| --- | --- |
| Admission / queued cancel | 자원이 생기기 전 취소는 Cancelled, cleanup=NotRequired. 소켓을 열거나 가짜 ACK를 기다리지 않는다. |
| Running cancel / deadline | CancelRequested와 terminal을 구분한다. 소유자에게 abort를 전달한다. 사용자 취소·run/idle timeout 사유를 구분하고 cleanup deadline까지 두 owner를 추적한다. |
| Completion 경합 / reload / quit | Host가 기존 전이 규칙으로 순서를 직렬화한다. 유효한 완료와 두 cleanup 확인 후 성공할 수 있다. reload는 generation 교체·RuntimeLost·snapshot 재동기화, quit는 admission 잠금·중단·정리를 진행한다. 자동 재전송은 0회다. |
| SSE duplicate / gap / identity 불일치 | Duplicate는 ingress에서 DuplicateEvent, 해당 run의 terminal은 Failed/ProtocolError 한 번이다. 이미 accepted된 event와 partial text는 불변이다. gap·잘못된 결속도 ProtocolError. |
| Cleanup pending / 미확인 | 직전 terminal이 Failed여도 cleanup 확인 전 다음 run을 막는다. deadline 경과 시 기존 Host 규칙에 따라 CleanupUnconfirmed를 나타낸다. 늦은 owner ACK는 cleanup만 갱신하고 늦은 delta/completion은 거부한다. |
| HTTP / transport | 401 → AuthRequired, 403 → PermissionDenied, 5xx → ProviderUnavailable, offline/단절 → Offline. 429는 allowlisted code/type으로 RateLimit → ProviderRateLimit, quota/credit/spend → 각각 native 상세 원인 및 공통 QuotaExceeded, 알 수 없는 429 → ProviderError. |
| 오류 노출 | 오류 body 최대 8,192 bytes, code/type만 분류하고 message/param/raw error를 폐기한다. 원격 incomplete·성공 전 EOF는 partial로 남긴다. 오류 안내 뒤 재시도에는 사용자 신규 요청·동의·예산이 필요하다. |

## 10. Regression & Live Test Matrix

아래 행은 **향후 시험 계획이며 모두 NOT_RUN**이다. P2.2a의 과거 통과 수를 이 문서 HEAD의 증거로 합산하지 않는다.

| 단계 / 입력 | 판정과 남길 증거 |
| --- | --- |
| b-1 · locked/missing/denied, register → rotate → revoke, active lease, update/reinstall/signing 변경 | 격리 test item만 접근, 권한 실패 시 admission 닫힘, 삭제 결과·안전 코드만 기록. IPC/로그/fixture/산출물·crash capture 경로에 실제 secret 없음. |
| b-2 · 기존 Rust/Frontend/network-denied, FakeRun·Native Approval·Synthetic Dock | 단일 controller, Synthetic 표시·기존 lifecycle 및 UI 회귀 검증. 기본 빌드에서 외부 dispatch 불가. |
| b-2 · 로컬 제어 transport의 TLS 실패/redirect/wrong MIME, 느린 소비자, 분할 UTF-8, 과대 frame·EOF | 외부 송신 없이 abort/join, 제한된 메모리, 정확한 safe code와 partial 보존. |
| b-2 · duplicate/conflict, model/item 혼입, late delta/completion, cancel-vs-complete, reload/quit | 가상 clock/barrier로 순서 고정. terminal count=1, 동일 text 재추가 0, source 불변, 두 owner 정리 전 새 start 거부. |
| b-2 · 401/403/429 세부 원인/5xx, usage 누락, cache-write, 원장 손실 | safe classification, 자동 retry 0, 과금 unknown/예약 보유, 재시작·동시 start로 예산 우회 불가. |
| b-3 · 승인된 실제 시험 1건 | 아래 절차로 단일 요청과 실제 streaming, usage, cleanup, 비밀 비노출을 개별 판정. |

**첫 live 시험 절차.** (1) 사용자가 격리된 개발 project, 유효 credential, 고정 모델/가격, run·일·월 한도, rate, 일회성 실행을 승인한다. (2) 해당 서명 앱에서 전송될 전체 synthetic prompt(예: “Reply with OK.”)와 수신처를 보여 주고 동의를 받는다. (3) foreground에서 text 한 건만 전송한다. tools/audio/memory/computer control은 비활성이다. (4) streaming text, source, terminal count=1, adapter/runtime cleanup, usage를 대조한다. (5) 본문/키/헤더가 없는 제한된 증거와 비용 대조 상태만 남긴 뒤 live admission을 닫는다. 전송 횟수 불명, 미지원 SSE, usage/cleanup 누락, 비밀 노출 의심이면 중단한다. 관측하지 못한 항목을 미확인으로 남기고 재호출에는 새 승인이 필요하다.

## 11. P0 Dependencies

Runtime A(WKWebView)는 단일 foreground 개발 경로다. E5b/E5c는 PARTIAL, E5d ADR은 PROPOSED, 전역 Runtime 선정은 NO다. 이 설계나 첫 live 시험으로 P0 비교 측정을 대체할 수 없다. 고정 packaged app, benchmark 원본, P0 및 다른 worktree를 보존하고 측정 재개 시 같은 Mac의 무거운 build/test/GUI를 중단한다.

## 12. Open Questions / Decision Register

| 사용자 승인 대상 | 결정에 필요한 증거 / 미결정 시 동작 |
| --- | --- |
| Local BYOK 또는 backend, 서명·access group | local test item의 수명·배포 경계 검토. 미결정 시 DisabledCredentials 유지 |
| 모델·snapshot·reasoning/cache 정책 | 반환 ID와 strict mapper, tokenizer, 지원 event/usage 양립성. 불일치 시 거부하고 alias를 자동 추가하지 않음 |
| USD 정책과 원장 | 가격 버전, run/일/월·RPM/TPM, cache-write·미정산 복구, 소유자 비용 대조 절차. 하나라도 없으면 live 차단 |
| 전송·보존 동의 및 검증물 | 프로젝트 data controls, 사용자 철회, 최소 집계 저장 기간·접근자. 동의가 없으면 전송 금지 |
| 첫 실행 승인 | 지정 bundle/HEAD, 유효한 프로젝트·credential·한도, 승인된 1회 prompt. 결여 시 실험 NOT_RUN |

P2.0 canonical duplicate 처리와 P2.2a provider SSE duplicate 실패는 서로 다른 ingress에 적용된다. b-2에서는 이 차이, OfflineProvider의 Host 소유 구조, 실제 socket cleanup, alias 및 cache-write 제약을 명시적 fixture로 해소해야 한다. 기존 계약이나 global Runtime ADR의 수정을 이 문서에서 승인하지 않는다.

## 13. P2.2b Implementation Breakdown

- **P2.2b-1 · Credential와 동의 경계.** 사용자 선택 후 native 입력·resolver·정책과 mock 동의를 검증한다. 실제 Keychain 시험은 격리 항목·서명 범위의 별도 권한이 필요하다. 성공 기준은 비노출·폐기·회전·철회 후 차단이다. 원격 권한은 닫힌다.
- **P2.2b-2 · Native transport와 기존 Host 연결.** 별도 구현 승인 후 고정 목적지, TLS, bounded SSE, 비용 원장, deadline·abort/join을 구현한다. 격리된 로컬 transport fixture로 §10을 검증하고 테스트용 목적지 주입이 배포 빌드에 없음을 확인한다. 실제 credential/과금 없이 완료할 수 있어야 한다.
- **P2.2b-3 · Controlled Live Test.** b-1/b-2 증거 리뷰, 비용·보존 설정, 전송 동의와 별도 실제 호출 승인을 모두 받은 뒤 §10의 단일 실행을 평가한다. 불확실한 항목과 실제 billing 자료의 가용성을 따로 보고한다.

## 14. Acceptance & Approval Gates

b-0 완료 조건은 이 한 파일의 계약 대조, 공식 출처·후보 비교, 보안·비용·동의 결정 대장, fixture 및 단계별 승인 경계를 Issue에 연결된 Draft PR로 검토할 수 있는 것이다. 거버넌스·whitespace·diff 범위·민감정보 검사와 해당 HEAD의 CI 결과는 PR에 기록한다. 현재 단계는 설계 리뷰만 요청한다. Ready/병합, b-1, b-2 및 b-3의 실제 비용 발생 승인은 각각 독립적이며 앞 단계 수용으로 자동 이행되지 않는다.
