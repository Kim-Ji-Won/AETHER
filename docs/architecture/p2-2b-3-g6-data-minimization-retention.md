# P2.2b-3 G6 — 데이터 전송 최소화·로그·보존 정책 준비

- 상태: **PROPOSED** — 정책 초안과 후속 수용시험 계약. 제품 G6는 **OPEN DECISION**, 첫 실제 Provider 요청은 **BLOCKED**다.
- 관련 작업: [Issue #72](https://github.com/Kim-Ji-Won/AETHER/issues/72). 이 Issue의 완료 범위는 준비 문서이며 제품 G6 통과가 아니다.
- 소스 대조 기준: `b15809841e7f3e9f6cc1f680bae920fe8fb8593a`. 공개 Provider 문서 확인일: **2026-11-26 KST**.
- 선행 계약: [Gate 0 §10–§13](p2-2b-3-live-integration-preflight.md), [Credential·Consent](p2-2b-1-native-credential-consent.md), [Transport](p2-2b-2-native-http-transport.md), [G1](p2-2b-3-g1-offline-single-host.md), [G2](p2-2b-3-g2-credential-signing-preflight.md), [G3](p2-2b-3-g3-offline-revocation.md), [보안 정책](../../SECURITY.md).

## 1. 범위와 판정

Gate 0 §12에 따라 G2 실제 서명·Keychain 시험을 기다리는 동안 G6 정책을 준비한다. 이번 변경은 이 문서 한 개이며 코드·설정·의존성·기존 검증 문서를 바꾸지 않는다. 아래 **현재 관찰**과 **PROPOSED** 요구사항을 구분한다. 정책을 문서화하거나 병합해도 제품 동의 발행·dispatch 권한은 생기지 않는다.

| 구분 | 이번 작업의 결과 또는 유지할 상태 |
|---|---|
| 준비 범위 | 첫 합성 요청의 허용 목록, 로그·진단 경계, 로컬 보존 후보, 공개 정책과 프로젝트 미확인 항목, 후속 수용시험 |
| 제품 G6 | **OPEN DECISION**. 실제 프로젝트 정책 확인과 비노출·보존 제어 구현/시험 전에는 전송 차단 |
| 다른 Gate | G2 OS 시험 **NOT_RUN**, 제품 G3 영속 journal 통합 **NOT_IMPLEMENTED**. G4·G5·G7·G8·G9 완료를 주장하지 않음 |
| 현재 제품 | `DisabledCredentials`·`ProviderDisabled` 유지. 실제 Provider 요청·API Key 접근·Keychain 항목 연산·billing 접근 각각 **0회** |
| 조사 경계 | 저장소와 공식 공개 웹 문서만 읽음. API 조직·프로젝트·계정 설정을 조회하거나 바꾸지 않음 |
| 독립된 검증 기록 | [PR #71](https://github.com/Kim-Ji-Won/AETHER/pull/71)의 macOS 증거 문서는 별도 리뷰 대상. 이번 문서가 그 PR의 병합이나 재실행을 대신하지 않음 |

## 2. 현재 코드에서 확인한 사실과 남은 차이

| 경로 | 현재 관찰 | G6에서 남은 요구사항 |
|---|---|---|
| [policy.rs](../../apps/desktop/src-tauri/src/agent_text/provider/policy.rs) | `RequestPreview::body()`는 단일 `user/input_text`, `stream:true`, `store:false`, `background:false`, 빈 tools와 닫힌 옵션을 생성한다. `Ledger::prepare()`는 nonempty text와 정책 한도를 검사한다. | 임의 입력을 합성 문구로 판별하지 않는다. 첫 시험의 정확한 문구 한 건만 허용하는 제품 경계가 필요하다. |
| [consent.rs](../../apps/desktop/src-tauri/src/agent_text/provider/consent.rs) | Binding은 identity·session·endpoint·JSON Value body·credential generation·policy version을 비교한다. 승인은 일회 소비되며 fixture 발행은 test-only다. | 실제 native issuer와 프로젝트 정책 결속이 없다. 구조화 JSON 동등성은 G4의 불변 직렬화 바이트 승인 증거가 아니다. |
| [provider/mod.rs](../../apps/desktop/src-tauri/src/agent_text/provider/mod.rs) | `live_dispatch()`는 `ProviderDisabled`이며 HTTP reader를 열기 전에 차단한다. `revocation` 모듈 전체는 `cfg(test)`다. | 실제 전송과 파일 journal·재시작 복구가 연결됐다고 해석하지 않는다. |
| [transport.rs](../../apps/desktop/src-tauri/src/agent_text/provider/transport.rs), [http.rs](../../apps/desktop/src-tauri/src/agent_text/provider/transport/http.rs), [error.rs](../../apps/desktop/src-tauri/src/agent_text/provider/error.rs) | 요청 직렬화 상한은 32 KiB. 고정 HTTPS 목적지·TLS 검증·redirect/proxy 차단이 있다. HTTP 오류는 제한된 버퍼를 거쳐 허용 코드로 정규화한다. | 분류 전 raw error는 메모리에 존재한다. 제품 경로의 비노출·수명 시험이 필요하며 앱 Synthetic 입력 한도를 Provider 예산으로 대신하지 않는다. |
| [Host](../../apps/desktop/src-tauri/src/agent_text/mod.rs), [Runtime](../../apps/desktop/src-tauri/src/agent_runtime.rs), [renderer owner](../../apps/desktop/src/agent-runtime.ts) | Host input·snapshot text·events·마지막 frame과 renderer의 submitted·lastStart·lastRun·view에 본문이 남는다. 다음 admission 성공 시 Host run/events가 교체된다. | 취소·reload·cleanup 완료는 본문 삭제 완료가 아니다. 별도 삭제 동작과 소유자별 수명 관리가 필요하다. |
| [ConversationResponse](../../apps/desktop/src/ConversationResponse.tsx), [AgentDiagnostics](../../apps/desktop/src/AgentDiagnostics.tsx), [VisualSnapshot](../../apps/desktop/src/scene/visual-state.ts) | 대화 화면은 입력·출력을 표시한다. 현재 Synthetic 진단 화면도 `view[1][8]` 전체 출력을 표시한다. 시각 상태 snapshot은 별도 metadata 구조다. | Provider 본문을 대화 화면과 native preview에 한정하고 진단 화면은 metadata만 받도록 검증해야 한다. 현재 실제 Provider 노출을 관찰했다는 뜻은 아니다. |

조사한 frontend와 Host/Provider 경로에서 앱이 직접 구현한 영속 대화 저장·본문 로그 경로는 확인하지 못했다. 그러나 앱 전체의 stderr metadata 출력은 존재한다. `Start`·`RunSnapshot`·`Frame`·`Event`처럼 본문을 담고 `Debug`를 구현한 타입도 있어, credential 타입의 `Debug` 금지만으로 전체 로그 비노출을 보장할 수 없다. 저장 API 검색 결과는 WKWebView/OS cache, swap, crash dump, DevTools, 사용자 캡처나 셸 리다이렉션의 부재 증거가 아니다. 소스 감사는 실행 검증을 대체하지 않는다.

## 3. 첫 전송의 데이터 허용 목록 — PROPOSED

첫 시험은 개발자 한 명이 native 화면에서 확인한 합성 text prompt **한 건**, 응답도 실행하지 않는 일반 텍스트로 한정한다. 문구 후보는 “연결 확인용 합성 문구입니다. 한 문장으로 답해주세요.”이며 최종 문구·모델·출력 한도·비용은 G5/G9에서 확정한다. 이 후보를 기록한 것은 전송 승인이 아니다.

| 계층 | 허용할 데이터와 조건 |
|---|---|
| 입력 | 승인한 문구와 정확히 일치하는 단일 `user/input_text`. 공백·유니코드 변경을 포함해 미리보기 이후 바뀌면 다시 승인 |
| 본문 | 현재 닫힌 schema의 `model`, `input`, `stream`, `store`, `background`, `tools`, `tool_choice`, `parallel_tool_calls`, `max_output_tokens`만 후보. 모델은 G5의 고정 ID, `stream:true`, `store:false`, `background:false`, `tools:[]`, `tool_choice:"none"`, `parallel_tool_calls:false` |
| 목적지·인증 | 고정 `https://api.openai.com/v1/responses`. 실제 승인된 프로젝트 범위와 native credential만 사용. 인증 헤더는 native transport에서만 구성하며 preview·renderer·로그로 복사하지 않음 |
| 응답 | strict SSE/mapper가 수락한 text와 허용 usage 집계. 원격 텍스트는 명령·HTML·도구 실행 권한이 아님 |
| 한도 | 기존 32 KiB 직렬화 상한과 더 낮은 G5 입력/출력·USD 한도를 모두 적용. 기존의 가상 비용 단위는 실제 tokenizer·가격 증거가 아님 |

이전 대화·system/developer prompt·첨부·파일 경로/내용·화면·마이크·Accessibility·clipboard·persistent memory·벡터 검색·도구 결과·사용자 식별 metadata·`previous_response_id`·conversation 참조·background 작업을 이 첫 요청에 넣지 않는다. 앱이 계정·환경 정보를 숨겨 덧붙이지 않는다. 인증과 네트워크 연결에 필요한 데이터까지 익명이라고 안내하지 않는다.

Host/native가 승인 스키마와 한 건의 내용을 검사하며 renderer 주장의 신뢰나 사후 마스킹으로 대체하지 않는다. 모르는 필드나 지원 여부가 미확인인 옵션은 lease/reader 생성 전에 거부한다. 모델별 cache 제어를 위해 필드 추가가 필요하면 G5/G6 계약·직렬화·동의 binding을 먼저 갱신한다. 존재하지 않는 “cache 끄기” 옵션을 임의로 만들거나 옵션 생략을 비활성으로 해석하지 않는다. 이번 변경은 schema를 수정하지 않는다.

## 4. 메모리 표시·진단·증거의 경계 — PROPOSED

정상 대화에 필요한 본문과 운영 증거를 서로 다른 구조로 전달한다. 운영 로그는 객체 전체나 임의 문자열을 받은 뒤 지우는 방식 대신 허용 필드만 새 구조에 담는다. 이 요구는 stdout/stderr, JS console, native 오류·panic 경로, benchmark, diagnostics, 자동 screenshot/dump, 지원용 export에 동일하게 적용한다.

| 데이터 | 정상 UI/수명 제한 메모리 | 일반 진단·로컬 운영 증거·공개 Issue/PR |
|---|---|---|
| 입력·응답·partial output·원시 SSE | 사용자가 확인할 본문과 처리 중 필요한 버퍼에 한정 | 원문·일부 발췌·암호화 사본·본문 digest 모두 제외 |
| API Key·Authorization·쿠키·raw header·raw error | 필요한 native 처리 범위만 허용. 키/헤더는 UI에 표시하지 않음 | 값·일부 문자·길이 기반 지문·hash·객체 Debug 출력 제외 |
| 안전한 실행 결과 | 표시용 닫힌 error/status enum, terminal·cleanup 상태 | 정책/소스 버전, 시험 case ID, 안전한 결과 코드, cleanup 확인 수, token/비용 집계만 허용 |
| run/request/attempt·credential slot·원격 response ID·계정/프로젝트 ID | 권한 대조·파싱·복구에 필요한 해당 owner만 보유 | 일반 증거와 공개 기록에서 제외. 필요한 안전 journal/원장은 §5의 별도 계약 적용 |
| 검증 대상 식별 | 비밀 아닌 소스 SHA·승인 앱 산출물 hash·정책 버전 | 재현성 식별자로 허용. 본문·credential hash와 혼동하지 않음 |

Token/비용 집계도 공개 전 검토한다. 단일 run의 원시 ID·정밀 시각·로컬 사용자/경로를 결합해 추적 가능한 기록을 만들지 않는다. API 응답의 자유 형식 error message나 서버가 돌려준 URL을 오류 설명에 그대로 붙이지 않는다. 오류는 안전 코드와 사용자가 할 수 있는 조치로 표시한다.

대화 화면과 native preview에서 본문을 보여주는 것은 진단 수집 권한이 아니다. 현재 Synthetic 진단의 본문 표시를 실제 Provider에 연결하기 전에 제거하거나 source별 metadata projection으로 격리해야 한다. `RequestPreview`의 비직렬화/비Debug 성질과 기존 consent 시험만으로 이 조건을 통과 처리하지 않는다.

## 5. 로컬 접근·보존·삭제 후보 — PROPOSED / 소유자 결정 필요

아래 숫자와 담당 범위는 검토 가능한 **추천 후보**이며 승인된 제품 설정이 아니다. 첫 시험의 접근자는 실행 담당 개발자 한 명을 기본으로 제안한다. 추가 검토자에게는 내용을 확인한 비민감 요약만 전달하며, 앱의 자동 업로드·자동 공개 export는 두지 않는다.

| 구분 | 추천 보존·접근 후보 | 종료·실패 처리 |
|---|---|---|
| 본문·미리보기·대화 상태 | 앱이 직접 만드는 영속 대화 저장 없음. 처리·표시를 위한 메모리만 사용 | 취소 중 partial output을 보존한다. 결과 확인 후 명시적 결과 지우기 또는 시험 세션 종료로 참조를 해제한다. 활성 처리 중 지우기 요청은 즉시 표시를 숨기고 취소·cleanup 후 각 owner의 본문 참조를 정리한다. 현재 해당 삭제 기능은 미구현이다. |
| 일반 운영 로그 | 첫 시험은 디스크 기록·외부 전송 기본 비활성. 필요한 순간의 안전 metadata 표시만 허용 | stderr 수집 등 별도 기록 경로가 있다면 묵시적으로 허용하지 않고 시험 구성과 보존 책임에 포함한다. |
| 선택한 비민감 로컬 시험 증거 | opt-in, 실행 담당자만 접근. 사용자 전용 비공유·비동기화 위치를 후보로 하며 macOS 권한/ACL을 검증. 생성 후 **최대 7일**, 리뷰가 먼저 끝나면 즉시 삭제 | 생성 시각·삭제 기한을 기록. 만료/시각 신뢰 상실/삭제 실패는 안전한 운영 오류로 표시하고 새 증거 쓰기·새 live 승인 차단. 잠자기·종료 동안 삭제 실행을 보장하지 않으며 다음 활성화 시 접근·admission보다 먼저 만료 처리한다. |
| 실제 프로젝트 정책 확인표 | 계정 ID 대신 필요한 로컬 별칭 사용. 대응 정보는 실행 담당자만 보유. 첫 실행마다 재확인하고 **확인 후 최대 24시간**을 유효기간 후보로 둠 | 만료·변경·대조 불능이면 동의 발행 차단. 일반 사본은 위 7일을 적용하되 미정산 조정에 필요한 최소 연결 정보는 아래 원장 계약으로 분리한다. |
| 공개 검증 근거 | 사람이 확인한 소스/앱/정책 버전·case별 PASS/FAIL/NOT_RUN·집계만 수동 게시 | GitHub/원격 복제본은 로컬 7일 정책 대상이 아니다. 공개 후 완전 회수나 자동 삭제를 약속하지 않는다. |
| Credential | G2의 정확한 전용 주소·서명·접근 정책 적용 | 일반 로그 TTL이나 “대화 지우기”로 삭제하지 않는다. Dispatch된 활성 작업은 G3의 양쪽 cleanup 확인이 필요하며, 미실행/queued 작업은 기존 계약의 정당한 `NotRequired`를 포함한다. 이후 durable 삭제 의도와 정확한 항목의 삭제 확인을 따른다. |
| 철회 journal·tombstone·세대/operation 상태 | 본문/secret 없는 최소 안전 상태. 일반 증거와 별도 저장·접근·복구 계약 | **일반 TTL 제외.** 삭제 확인 뒤에도 재시작·rollback·옛 ACK 방어에 필요한 연속성을 유지한다. 검증된 안전한 압축/이관 전에는 기록 삭제·앱 재설치로 잠금을 초기화하지 않는다. |
| 미정산 예약·비용 원장 | 불투명 attempt·가격/정책 버전·시간 창·예약/정산 상태·안전 집계와 필요한 최소 대조 연결 정보 | **일반 TTL 제외.** 명시적 조정 근거 전에는 예약을 지우거나 환불하지 않는다. 정산 후에도 유효한 일/월 예산·멱등성에 필요한 집계를 보존하는 별도 압축/종료 계약이 필요하다. |

위 7일·24시간은 로컬 정책 후보이며 Provider 보존 기간과 관계없다. 안전 기록의 “보존 예외”는 본문·secret을 추가 저장하는 예외가 아니다. 해당 journal/원장의 영속 저장, 접근 제어, rollback 탐지, 조정·압축 API는 이번 작업으로 구현되지 않는다.

본문을 지워도 terminal·cleanup 판정이나 revision·identity의 단조 증가 기준을 초기화하지 않는다. 현재 renderer의 `newer()`는 같은 terminal run의 본문 변경이나 run 제거를 받아들이지 않으므로, 표시 숨김과 본문 폐기는 별도 버전의 후속 프로토콜로 검증해야 한다. 기존 snapshot을 임의로 비우거나 새 admission으로 위장해 지우기를 구현하지 않는다.

결과 숨김, 객체 참조 해제, 파일 삭제 확인, Keychain 부재 확인, 원격 삭제를 별개 상태로 판정한다. Rust/JS 메모리 해제만으로 swap·OS 진단·백업의 물리적 완전 삭제나 zeroization을 보장하지 않는다. 시험 담당자는 자동 캡처·백업/동기화·crash reporter의 범위를 확인해야 하며, 수동 캡처·복사본은 별도 책임으로 안내한다. 불명확한 삭제 결과를 “완전 삭제됨”으로 표시하지 않는다.

## 6. Provider 공개 정책 조사 — 실제 프로젝트 미확인

다음은 **2026-11-26**에 읽은 공식 공개 문서의 요약이다. 실제 AETHER 조직·프로젝트·모델 적용값이나 계약을 확인한 결과가 아니다. 공개 페이지 열람, Apple/ChatGPT 로그인, ChatGPT 대화 설정은 API 프로젝트 정책의 확인 증거가 아니다.

| 항목 | 공개 문서에서 확인한 범위 | 이번 제안에 미치는 영향 |
|---|---|---|
| 학습·공유 | API 입력/출력의 foundation model 개선 공유는 기본 비활성이며 소유자가 조직/프로젝트별로 선택할 수 있다. 일부 고객 콘텐츠의 안전성 classifier 학습·평가는 별도 적용될 수 있다. [공유 정책][O1] | “어떠한 학습에도 쓰지 않음”을 약속하지 않는다. 실제 공유 설정과 안전성 활용 적용 여부를 확인한다. |
| Abuse monitoring | 요청·응답과 파생 metadata가 로그에 포함될 수 있다. 기본 최대 30일이며 법률·서비스/제3자 보호에 필요한 더 긴 보존 예외가 있다. [Data controls][O2] | “항상 30일 내 삭제”나 `store:false`에 근거한 보존 0일 표시를 하지 않는다. |
| Responses application state | 일반 Responses는 기본 또는 `store:true`에서 30일 보존 정책이 있으며 해당 경우 최소 30일 저장을 설명한다. 별도 기능 예외가 있다. [Data controls][O2] | 첫 요청은 `store:false` 후보를 유지하되 다른 보존까지 없어지는 것으로 해석하지 않는다. |
| MAM·ZDR | 자격·사전 승인이 필요하고 조직/프로젝트별 상속·선택·예외가 존재한다. [Data controls][O2] | 현재 적용 여부 **NOT_VERIFIED**. ZDR을 가정하거나 일반 정책만으로 프로젝트를 승인하지 않는다. |
| Background | `store:false` 또는 ZDR에서도 처리·결과 조회를 위한 임시 저장이 발생할 수 있다. [Background][O3] | 첫 시험에서 제외한다. |
| Prompt caching | 설정·기본값은 모델/조직에 따라 다르다. 암호화 KV가 GPU-local application state로 최대 24시간 보존될 수 있고, 일부 모델의 TTL은 최소 수명이다. [Prompt caching][O4] | 옵션 생략·짧은 입력·ZDR만으로 cache/보존 0을 추정하지 않는다. 실제 모델의 지원 설정과 효과를 확인한다. |

원격 접근·보존·삭제 범위와 예외는 실행 직전 최신 공식 자료 및 실제 프로젝트의 실효 설정을 함께 검토한다. 이번에는 데이터 공유 opt-in, 보존 설정 변경, 모델 조회 API, 요청 전송을 실행하지 않았다. 다른 도구·파일·오디오·화면 기능은 이 첫 시험의 범위 밖이며, 향후 추가 시 해당 기능과 제3자 정책을 따로 검토한다.

## 7. 결정 대장과 정책 유효성 — OPEN DECISION

| 결정 항목 | 현재 값 | 실행 전에 필요한 근거와 담당 |
|---|---|---|
| 프로젝트와 접근자 | 미확정 | 프로젝트 소유자·실행 담당자, 실제 프로젝트와 로컬 별칭의 대조. 실제 ID를 공개 문서에 넣지 않음 |
| 공유·보존 실효값 | **NOT_VERIFIED** | 소유자가 input/output·feedback·evaluation 공유, retention 상속/override, 별도 통지·모델 예외를 확인 |
| 모델·옵션·비용 | G5 **BLOCKED** | 정확한 model snapshot·지원 schema·cache 실효 설정·출력 한도·가격 버전·요청/일/월 USD 한도 |
| 로컬 접근·보존 | §5 후보, 미승인 | 접근자, 7일/24시간 후보의 수락 또는 대안, 로컬 위치·권한·복사본·삭제 실패 조정 담당 |
| 정책 버전·기한 | 미확정 | 확인 시각·유효 종료·검토자·공식 근거·프로젝트 적용값을 한 버전으로 묶음. 만료·시계 이상 시 차단 |
| 실제 비노출·삭제 | **NOT_IMPLEMENTED / NOT_RUN** | §9 제품 수용시험, G2/G3/G4/G5/G7/G8 증거와 대조 |

실제 값이 없는 칸을 기본값·샘플·fixture 값으로 채워 PASS 처리하지 않는다. 프로젝트·모델·endpoint·본문·cache·공유/보존 설정·로컬 보존·가격·credential generation·정책 버전이 바뀌거나 유효성이 만료되면 기존 미소비 동의를 폐기한다. 변경이 나중에 원래 값으로 돌아와도 옛 승인을 살리지 않는다. 재시작 뒤 동의나 네트워크 재시도 권한을 복원하지 않는다.

## 8. 사용자 안내와 일회 동의 — PROPOSED

다음은 G4 native 화면의 안내 후보다. 대괄호는 개발용 placeholder이며 실제 화면에 미확정 값이 하나라도 있으면 확인 동작을 비활성화한다.

> 표시된 합성 문구 한 건을 OpenAI의 [고정 목적지]에 [확인한 프로젝트 범위]로 전송합니다. 이전 대화·파일·화면·마이크 데이터는 포함하지 않습니다. 모델은 [고정 ID], 이번 요청의 예상 최대 비용은 [USD 상한]입니다. 인증에 필요한 정보와 네트워크 metadata도 공급자가 처리합니다. 이 프로젝트의 공유·보존 및 cache 조건은 [확인한 요약/예외]입니다. 앱의 본문 표시·지우기와 로컬 증거 보관 조건은 [승인된 로컬 정책]입니다. 취소하면 앱은 수신을 중단하지만 이미 전달된 요청의 처리·보존·청구가 계속될 수 있습니다. 이 내용으로 한 번 전송할까요?

전체 합성 문구와 실제 전송할 불변 직렬화 본문을 native 경계에서 대조하고, 읽을 수 있는 내용 및 모델·목적지·옵션·정책·비용을 사용자에게 빠짐없이 보여준다. 비밀 헤더는 표시하지 않는다. 승인된 바이트와 scope는 정확한 attempt·세션·credential generation·정책/가격 버전·만료와 결속하며 전송 직전 재검사한다. 기존 JSON Value 비교를 이 계약의 완료 증거로 사용하지 않는다.

OS 권한, API Key 등록, 이 문서 승인, G9 운영 승인을 G4의 현재 요청 동의로 대신하지 않는다. G9는 G1–G8의 유효한 증거와 특정 산출물/요청에 대한 별도 일회 실행 결정이다. 취소·전송 결과 불명 시 자동 재시도하지 않으며 로컬 cleanup 성공으로 원격 처리·보존·청구의 종료를 확정하지 않는다.

## 9. 후속 수용시험 계약 — 전체 NOT_RUN

비밀 아닌 canary와 메모리/로컬 제어 fixture부터 검증한다. 아래 시험 코드는 이번 작업에 추가하지 않았으며 기존 Rust 회귀의 통과 수에 포함하지 않는다. 실제 키·외부 Provider 호출이 필요한 시험은 해당 Gate와 별도 실행 승인 뒤에만 수행한다.

| ID | 시험 | 통과 기준 | 현재 |
|---|---|---|---|
| G6-T01 | 합성 한 건과 닫힌 schema | history·첨부·추가 input·도구·임의 metadata·모르는 필드 혼입은 lease/reader 이전 거부. 승인한 한 건만 수락 | NOT_RUN |
| G6-T02 | 미리보기와 바이트 결속 | 공백·유니코드·목적지·모델·프로젝트/정책 변경을 거부. native 확인 바이트와 전송 바이트 일치 | NOT_RUN |
| G6-T03 | 정책 미확인·만료·시계 이상 | 프로젝트 적용값/예외·cache·소유자 근거가 없거나 만료되면 동의 발행/dispatch 0. `store:false`만으로 보존 0일 표시 금지 | NOT_RUN |
| G6-T04 | 동의 replay·generation·재시작 | 소비/취소된 승인, 변경 전 승인, 재시작 복구 승인 재사용 0. 네트워크 자동 재시도 0 | NOT_RUN |
| G6-T05 | 악성 원시 오류 | header·본문·secret 유사 canary, malformed/oversized error를 안전 코드로만 전달. 오류 UI/로그/증거에 canary·digest 0 | NOT_RUN |
| G6-T06 | 진단·시각 상태·export 격리 | 본문은 정상 대화/native preview에만 표시. Synthetic 진단 경로가 Provider 본문을 받지 않으며 visual snapshot·metadata export에 본문 0 | NOT_RUN |
| G6-T07 | 로그 전체 경로 | stdout/stderr·JS console·native 오류/panic·benchmark에서 본문/secret/header canary 및 hash 0. 객체 전체 Debug 우회 없음 | NOT_RUN |
| G6-T08 | 메모리 수명과 지우기 | 제출·완료·취소·페이지 이동·reload·quit·지우기별 owner 수명 확인. partial output/terminal 계약 보존과 참조 정리를 별도로 판정 | NOT_RUN |
| G6-T09 | 영속 저장·자동 수집 | 앱이 직접 생성·수집·export하는 web storage·파일·자동 screenshot/dump·업로드에 본문 기록 0. OS/외부 기록은 별도 조사하고 제어·검증 한계를 명시 | NOT_RUN |
| G6-T10 | 접근·만료·삭제 실패 | 승인한 접근자/필드/기한만 허용. 잠자기·재시작·시계 이상·삭제 실패 후 거짓 성공/무단 재승인 없음 | NOT_RUN |
| G6-T11 | 일반 삭제와 안전 상태 독립 | 로그 만료/대화 삭제가 철회 상태·세대·미정산 예약·유효 예산 집계를 지우거나 admission을 열지 않음 | NOT_RUN |
| G6-T12 | 철회 최초 쓰기 지연·실패 | 즉시 취소/local abort·cleanup, 취소 후 delta 0, partial output/terminal 1 보존. 단일 ACK/기록 실패로 삭제 성공·원격 처리 종료를 선언하지 않음 | NOT_RUN |

## 10. 검증·보존·복구

이번 변경의 적정 검증은 소스 계약과 이 문서의 일치, 상대 링크, 민감정보/변경 범위, `python3 -B scripts/validate_governance.py`, `git diff --check`, 최종 커밋의 whitespace 및 GitHub Governance CI다. 실행한 명령·최종 HEAD·실제 결과는 PR/Issue의 Evidence에 기록한다. 설계 리뷰를 제품 privacy 시험 PASS로 바꾸지 않는다.

Rust·Frontend·Typecheck·network-denied·TLS loopback·packaged GUI·signed release와 §9의 새 시험은 이번 문서 HEAD에서 **NOT_RUN**이다. 현재 Linux 작업에서 macOS P0 고정 앱·원본 289개/추가 64·14·48개 증거를 다시 접근·해시 검증하지 않았다. 기존 macOS 보존 기록을 새 PASS로 복사하지 않으며 접근 가능한 기존 worktree의 HEAD·미커밋/미추적 상태 보존만 별도로 대조한다.

E5b/E5c는 **PARTIAL**, E5d ADR은 **PROPOSED**, 전역 Runtime은 미선정이다. 이번 문서는 기존 Gate를 세분화하는 제안이며 새로운 Runtime·배포 경로·보안 예외를 확정하지 않는다. 잘못된 정책은 후속 문서 수정으로 바로잡고, 병합 후 되돌릴 필요가 있으면 별도 승인된 문서 revert PR을 사용한다. Credential·journal·원장 삭제나 dispatch 활성화를 복구 절차에 포함하지 않는다. Draft PR 이후 다음 Gate 구현·Issue 생성·병합은 자동으로 진행하지 않는다.

[O1]: https://help.openai.com/en/articles/10306912-sharing-feedback-evaluation-and-fine-tuning-data-and-api-inputs-and-outputs-with-openai
[O2]: https://developers.openai.com/api/docs/guides/your-data
[O3]: https://developers.openai.com/api/docs/guides/background
[O4]: https://developers.openai.com/api/docs/guides/prompt-caching
