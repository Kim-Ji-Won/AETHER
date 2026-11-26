# P2.2b-3 G5 — 모델·비용 결정표와 오프라인 USD 원장 계약 준비

- 상태: **PROPOSED** — 준비 문서. 제품 G5와 첫 실제 Provider 요청은 **BLOCKED**다.
- 관련 작업: [Issue #74](https://github.com/Kim-Ji-Won/AETHER/issues/74). 완료 범위는 이 문서이며 모델 채택·예산 설정·제품 원장 구현이 아니다.
- 소스 기준: `146a38cd8df911ddbd3648c1172bc5eaea0d4a5b`. 공식 공개 자료 확인일: **2026-11-26 KST**.
- 선행 계약: [Gate 0 §7–§8·§12–§13](p2-2b-3-live-integration-preflight.md), [Credential·Consent](p2-2b-1-native-credential-consent.md), [Transport](p2-2b-2-native-http-transport.md), [G1](p2-2b-3-g1-offline-single-host.md), [G3](p2-2b-3-g3-offline-revocation.md), [G6](p2-2b-3-g6-data-minimization-retention.md), [보안 정책](../../SECURITY.md).

## 1. 범위와 현재 판정

G2 서명·Keychain 시험을 기다리는 동안 Gate 0이 허용한 G5 정책 준비를 수행한다. 기존 모델 비교 후보의 공개 사실, 현재 소스, 후속 설계, 소유자 미결정을 구분한다. 이 문서를 승인하거나 병합해도 실제 전송 권한이나 유효한 비용 정책이 생성되지 않는다.

| 항목 | 판정 |
|---|---|
| 이번 산출물 | 새 문서 1개. 제품 코드·테스트·설정·의존성·lockfile·기존 문서 변경 없음 |
| 실제 모델·프로젝트·USD 한도 | **OPEN DECISION / NOT_VERIFIED**. 비교값·합성 산술 예시로 채택 값을 대신하지 않음 |
| 제품 비용 제어 | 실제 tokenizer·USD 계산·영속 예약·프로세스 간 동시성·조정은 **NOT_IMPLEMENTED / NOT_RUN** |
| 제품 실행 | `DisabledCredentials`·`ProviderDisabled` 유지. G2 OS **NOT_RUN**, 제품 G3 영속 journal **NOT_IMPLEMENTED**, G6 **OPEN DECISION** |
| 접근 범위 | 저장소·공식 공개 웹 문서. 실제 계정·모델 조회/토큰 카운트 API·API Key·Keychain·Provider·usage/billing 접근 각각 **0회** |

## 2. 현재 구현과 실제 비용 제어의 차이

| 소스 | 현재 관찰 | 후속 요구사항 |
|---|---|---|
| [policy.rs](../../apps/desktop/src-tauri/src/agent_text/provider/policy.rs) | `text.len()`은 UTF-8 byte 수이고 가격은 가상 정수 단위다. `spent`·요청 count·rate window는 메모리 상태다. | 실제 모델 tokenizer·request overhead·USD·UTC 일/월·영속/동시성 검증을 대신하지 않는다. |
| 같은 `RequestPreview::body()` | 현재 9개 필드는 model/input/stream/store/background/tools/tool_choice/parallel_tool_calls/max_output_tokens다. | `service_tier`, reasoning effort, cache 제어 옵션이 없다. 생략된 옵션의 실효값을 확인하거나 별도 승인된 schema·binding 갱신이 필요하다. |
| [mapper.rs](../../apps/desktop/src-tauri/src/agent_text/provider/mapper.rs) | cached input·reasoning output이 각각 총량 이하인지 검사하고 nonzero `cache_write_tokens`는 거부한다. 요청/응답 model 문자열을 정확히 비교한다. | 실제 가격 분류·tier·모델 고정성을 확인해야 한다. 문서만으로 미지원 usage·다른 model ID를 허용하지 않는다. |
| [policy.rs](../../apps/desktop/src-tauri/src/agent_text/provider/policy.rs)의 `Usage`·`Accounting` | Usage에는 input/output/total 3필드만 남는다. 비용 초과분은 누적하고 불확실한 예약은 자동 환불하지 않는다. `billing_confirmed`는 false다. | cached/reasoning/write 종류별 집계를 비용 계산에 보존하는 계약과 Provider 보고·확정 청구·조정의 분리가 필요하다. |
| [consent.rs](../../apps/desktop/src-tauri/src/agent_text/provider/consent.rs)의 `start_consented()` | 함수 진입에서 승인 슬롯을 `take()`한 뒤 `prepare_binding()`을 수행한다. 실패해도 승인은 소모된다. 결속 일치 후 fixture lease를 resolve하고 즉시 drop한 뒤 Host admission으로 간다. | 순수 비용/결속 검사는 resolve 이전이다. 이후 저장 실패까지 “lease 0회”로 단정하지 않는다. 제품 lease/header 수명은 G4/G7 연결에서 검증한다. |
| [adapter.rs](../../apps/desktop/src-tauri/src/agent_text/provider/adapter.rs)의 `admit()` | 기존 Host가 `start_source()`로 identity를 확정한 다음 메모리 `Ledger::reserve()`를 호출한다. | Host admission 이후 durable 예약, reader 생성 이전 전송 차단, 저장 대기 중 취소/clock 처리가 필요하다. 두 번째 Host를 만들지 않는다. |
| [provider/mod.rs](../../apps/desktop/src-tauri/src/agent_text/provider/mod.rs), [http.rs](../../apps/desktop/src-tauri/src/agent_text/provider/transport/http.rs) | 제품 dispatch는 reader 이전에 `ProviderDisabled`다. G3 revocation 모듈은 전체가 `cfg(test)`다. | 실제 파일 내구성·원장 복원·제품 HTTPS 연결의 증거로 해석하지 않는다. |

## 3. 공식 공개 비교값 — 채택·견적 아님

### 3.1 모델 식별과 한도

Gate 0의 기존 두 비교 후보만 조사했다. 아래 최대치는 공급자 공개 한도이며 AETHER가 사용할 입력/출력 예산이 아니다. 공개 문서의 text·Responses·stream 지원은 현재 앱의 strict SSE/옵션 호환 시험을 통과했다는 뜻이 아니다.

| 비교 후보 | 공개 ID·고정성 근거 | Context / 최대 input / 최대 output | 기본 reasoning effort |
|---|---|---|---|
| GPT-5.4 mini | dated snapshot `gpt-5.4-mini-2026-03-17` 명시 | 400,000 / 272,000 / 128,000 tokens | `none` |
| GPT-6 Luna | 공개 목록은 `gpt-6-luna`. 날짜 고정 ID 또는 동등한 고정성 근거는 이번 조사에서 미확인 | 1,050,000 / 922,000 / 128,000 tokens | `medium` |

근거: [Mini 모델][O1]·[Luna 모델][O2]. Luna의 ID 형태만으로 내부 버전이 변한다고 단정하지 않으며, Gate 0의 재현성 기준 충족도 아직 선언하지 않는다. 실제 프로젝트 가용성·반환 model ID·폐기 일정과 실행일의 유효성은 추가 확인 대상이다.

### 3.2 공개 Standard 가격

단위는 **USD / 1,000,000 tokens**다. [공식 가격표의 Standard pricing data][O3]를 대조했다. 실제 프로젝트 계약·region·처리 tier·세금·할인·크레딧을 확인한 견적이 아니다.

| 모델·input 조건 | 일반 input | Cache read | Write 30m | Write 12h | Output |
|---|---:|---:|---:|---:|---:|
| GPT-5.4 mini | $0.75 | $0.075 | 별도 write 요금 없음* | 별도 write 요금 없음* | $4.50 |
| GPT-6 Luna · 272K 이하 | $0.10 | $0.01 | $0.125 | $0.25 | $0.50 |
| GPT-6 Luna · 272K 초과 | $0.20 | $0.02 | $0.25 | $0.50 | $0.75 |

*Mini의 30m/12h 옵션 지원을 뜻하지 않는다. 이전 모델군의 추가 cache-write 요금 부재와 모델별 지원 옵션을 구분한다. [Cache 계약][O5]. Luna의 긴 입력 요금은 초과분만이 아닌 요청 전체에 적용되며 region·처리 tier에 따른 다른 조건도 있다. [Luna 모델][O2]. 첫 시험에서는 확인한 단일 조건만 허용하는 것을 제안한다.

### 3.3 생략한 옵션과 비용 의미

| 공식 계약 | AETHER에 필요한 확인 |
|---|---|
| `service_tier` 생략은 `auto`로 프로젝트 설정을 따른다. `default`는 Standard이며 응답의 실제 tier는 요청값과 다를 수 있다. [Responses][O4] | 현재 요청에 해당 필드가 없으므로 Standard 확정을 주장하지 않는다. 프로젝트 실효값·반환 tier와 승인된 schema를 대조한다. |
| Cache write는 일반 input에 더하는 수수료가 아닌 배타적 input 요금 분류다. 이후 계열에는 30m/12h write 분류가 있다. [Cache][O5] | 현재 mapper의 nonzero write 거부를 유지한다. 지원하려면 정확한 분류·가격·usage 보존·회귀를 별도로 구현한다. |
| `max_output_tokens`는 visible output과 reasoning 등 생성 토큰을 제한한다. Reasoning도 output 요금이며 보이는 답변 없이 비용이 생길 수 있다. [Reasoning][O7] | 출력 총량에 reasoning을 다시 더하지 않는다. 빈 답변이나 취소를 비용 0으로 처리하지 않는다. |
| 요청의 역할·경계 등 formatting token은 문자열만 로컬 tokenize한 결과에 빠질 수 있다. [Token counting][O6] | 검증된 전체 과금 입력 상한이 필요하다. 토큰 카운트 API도 실제 외부 전송이므로 이번에 호출하지 않는다. |

Cache 옵션 생략·짧은 합성 문구·`store:false`를 write 비용 또는 보존이 0이라는 증거로 사용하지 않는다. 모델별 미지원 옵션을 임의로 넣지 않으며, G6 보존 조건과 가격 조건을 함께 확인한다.

## 4. 제품 정책 결정 대장 — OPEN DECISION

| 결정 | 필요한 값·근거 | 미확인 시 |
|---|---|---|
| 모델·프로젝트 | 정확한 ID/동등한 고정성, 허용 반환 ID, 프로젝트 가용성·로컬 별칭·소유자 | 동의 발행·비용 admission 차단 |
| Endpoint·schema·tier·region | 고정 Responses endpoint, text-only 옵션, 요청/반환 tier, 유료 기능 제외와 실효 조건 | 다른 요금을 추정해 통과시키지 않음 |
| 가격표 | 통화·token 단위·context 분기·cache 종류·추가 요금·출처·버전·유효 종료 | 과거 가격이나 fixture 값으로 대체하지 않음 |
| Tokenizer·입출력 상한 | 정확한 모델/버전, 전체 요청 구조·overhead 근거, 최대 과금 input/output, context 및 본문 byte 한도 | 불명확한 token 상한으로 예약하지 않음 |
| 사용자 한도·rate | 요청당·UTC 일·월 USD 한도, 로컬 요청/token 속도 창, 실제 프로젝트 RPM/TPM과 공유 범위 | 숫자를 임의로 정하지 않음 |
| 저장·시간 연속성 | 원장 형식·원자성/내구성·process lock·stale owner/rollback 방어, UTC 신뢰·재시작 대조 | 새 빈 원장을 만들어 전송하지 않음 |
| 운영 조정·보존 | 소유자·증거 접근자·정산 근거·조정 권한·보존/압축 조건 | 미정산 유지, 임의 환불·삭제 금지 |

가격 유효기간, G6의 프로젝트 정책 확인 유효기간 후보, UTC 일별 예산 창은 서로 다른 값이다. 모델·가격·tokenizer·요금 옵션·프로젝트·예산·정책·credential generation 변경 또는 만료 시 G4의 옛 동의를 폐기한다. 되돌린 설정이나 재시작으로 승인을 복구하지 않는다. G9의 특정 산출물·정확한 요청에 대한 일회 실행 승인은 별도로 필요하다.

## 5. 비용 분류와 산술 계약 — PROPOSED

선택한 모델의 usage와 요금 계약을 먼저 고정한다. 입력의 각 토큰이 일반/read/write 중 정확히 한 분류에 속한다는 근거가 있는 경우 다음 모델을 사용한다. 부분집합을 각각 검사하는 것만으로 서로 겹치지 않음이 증명되지는 않는다.

| 기호 | 의미·검사 |
|---|---|
| `I`, `C`, `W30`, `W12` | 전체 과금 input, cache read, 30m/12h write. 모두 음이 아닌 정수 |
| `U = I - C - W30 - W12` | 일반 input. 모든 합계·뺄셈을 검사하고 음수/overflow/중복 분류는 거부 |
| `W = W30 + W12` | 전체 write 집계와 일치해야 한다. `W` 비용을 TTL별 비용에 다시 더하지 않음 |
| `O`, `R` | 전체 과금 output과 그 안의 reasoning. `0 <= R <= O`; output 총량에 `R`을 다시 더하지 않음 |
| `E`, `Ecap` | 승인 범위의 추가 과금과 그 상한. 각 추가 항목을 개별 올림해 합한 정수 micro-USD이며, 미지원/상한 불명은 거부. 기능을 껐다는 증거 없이 0으로 두지 않음 |

공식 Responses의 TTL별 필드는 `input_tokens_details.cache_write_tokens_details.ttl_30m_tokens`와 `ttl_12h_tokens`다. [Responses][O4]. 해당 모델/schema가 정의하지 않은 항목, 누락 의미가 미확인인 필드, 모르는 유료 분류는 0으로 추정하지 않는다. 미지원 이벤트의 terminal 오류는 `UnsupportedContent`다. 중복 SSE sequence는 ingress에서 `DuplicateEvent`로 거부하고 terminal 오류는 `ProtocolError`로 한 번만 확정한다.

**추천 계산 후보:** 가격 decimal을 정확한 유리수로 읽고 로컬 최소 단위를 `micro-USD = 10^-6 USD`로 두는 방안을 검토한다. USD/백만 token의 숫자는 micro-USD/token과 수치가 같다. 각 항목을 micro-USD 단위로 보수적으로 올린 뒤 합산한다. 이 단위와 정수 폭은 아직 제품 설정이 아니다.

```text
ceil_div(n, d) = n / d + (n % d != 0 ? 1 : 0), d > 0

reported_estimate = ceil(U * Pu) + ceil(C * Pc)
                  + ceil(W30 * Pw30) + ceil(W12 * Pw12)
                  + ceil(O * Po) + E

Pinput_max = max(검증한 허용 input 요금 분류)
K = 해당 정책에서 허용한 input 요금 분류 수, K >= 1
rounding_headroom = K - 1  (micro-USD)
reservation = ceil(Icap * Pinput_max) + rounding_headroom
            + ceil(Ocap * Poutput_max) + Ecap
```

`P`는 micro-USD/token의 정확한 유리수이며 `ceil`은 그 값을 정수 micro-USD로 올린다. `(n+d-1)/d`의 중간 overflow를 피하고, 곱셈·분모 변환·항목 합계·기존 의무액과의 덧셈을 모두 검사한다. float 반올림이나 saturation으로 잘린 값을 정상 예산처럼 사용하지 않는다. 공급자의 실제 최소 과금/반올림 규칙이 별도라면 그보다 작은 상한이 되지 않게 확인한다.

분류별 올림 합은 전체 합의 한 번 올림보다 클 수 있다. `K`개 비음수 항의 올림 합은 전체 합의 올림에 `K-1`을 더한 값 이하이므로 예약에 그 여유를 포함한다. 예를 들어 일반/read 각 1 token, 단가 0.2/0.1이면 보고 추정은 `1+1=2`지만 `ceil(2*0.2)=1`만 예약하면 부족하다. `K=2`의 여유 1을 포함하면 2다. 지원 분류가 늘어나면 `K`와 산술 검증도 함께 바꾼다.

`Icap`은 전체 과금 입력의 검증된 상한, `Ocap`은 reasoning을 포함한 과금 출력 상한이다. 모델의 input/output/context 한도, 승인한 로컬 한도와 요청 크기를 모두 만족해야 한다. UTF-8 byte 수나 32 KiB 직렬화 상한만으로 `Icap`을 증명하지 않는다. Cache hit·무료 크레딧·할인·평균 출력 길이에 기대어 예약액을 줄이지 않는다. 긴 입력 가격·tier·region·추가 요금의 가능한 분기를 덮지 못하면 전송을 차단한다.

### 합성 산술 예시

아래는 **실제 모델·가격·사용자 예산과 무관한 가상 예시**다. `Pu=1`, `Pc=0.1`, `Pw30=1.25`, `Pw12=2.5`, `Po=4` micro-USD/token이고, `I=100`, `C=20`, `W30=30`, `W12=10`, `O=10`, `R=4`라면 `U=40`이다. 항목별 올림 결과는 `40 + 2 + 38 + 25 + 40 = 145 micro-USD`다. `W=40`과 `R=4`를 추가 청구하지 않는다.

같은 가상 가격에서 `Icap=128`, `Ocap=64`, `K=4`, 추가 요금 상한 0이 증명된 경우 예약은 `128*2.5 + 3 + 64*4 = 579 micro-USD`다. 보고 추정 145가 예약 579보다 작아도 차액 434를 자동 환불하거나 청구 확정으로 바꾸지 않는다. 문서 예시 재계산은 제품 원장 시험이 아니다.

## 6. 기존 Host와 durable 예약의 순서 — PROPOSED

1. Native가 불변 요청·모델·프로젝트·가격·token 상한·시간 유효성과 명백한 한도 초과를 순수 검사한다. 미설정·산술 실패는 resolve/reader 이전에 거부한다. 승인 슬롯 소비·실패 시 폐기는 기존 일회 계약을 유지한다.
2. 유효한 현재 요청 동의를 대조한 뒤 기존 Host가 **Queued identity**를 확정한다. Host admission에 실패하면 비용 예약을 만들지 않는다. G5가 별도 run/Host를 생성하지 않는다.
3. 같은 attempt에 대해 §7의 모든 예산/rate 검사, **비용 예약과 전송 의도**를 한 원자적 durable transaction으로 기록한다. 제품 저장소와 동시성 방식은 별도 구현·검증 대상이다.
4. Commit ACK를 owner instance·attempt·operation/sequence·정책 버전과 대조한다. Host clock·deadline·사용자 취소·G3 local abort/cleanup은 저장 완료를 기다리지 않고 계속 처리한다.
5. 같은 native owner가 현재 동의 결속·정책/가격 유효성·취소/철회·deadline과 예약을 최종 대조한다. 승인 소비·예약·전송 허용은 같은 직렬화 경계에 속하며 승인은 한 번만 소비한다. 이미 소비한 동의는 해당 attempt의 결속을 검증할 뿐 다시 소비/발행하지 않는다. 저장 ACK 자체는 승인이나 재전송 permit이 아니다.
6. 모든 조건이 유효할 때만 해당 attempt의 reader를 한 번 만들고 `Http::open()`으로 진행한다. **예약/의도 commit은 DNS·소켓을 시작할 수 있는 reader 생성 이전**에 완료돼야 한다. HTTP body 첫 쓰기 이전만으로는 충분하지 않다.

위는 후속 연결 계약이며 현재의 메모리 reserve를 durable 구현으로 간주하지 않는다. Host lock을 잡은 채 저장·TLS·Keychain·사용자 응답을 기다리지 않는다. 최초 예약의 저장 실패·ACK 유실·대조 불능이면 reader/socket 생성 0, 동의 재사용 금지, 신규 비용 admission 잠금과 기존 Host 취소 절차를 적용한다. 이 전송 전 단계의 새 run에는 아직 Provider partial output이 없다. Dispatch 이후 별도 usage/비용 기록의 지연·실패에서는 이미 수락한 partial output을 보존한다. 어느 경우에도 확정한 terminal을 바꾸거나 중복 생성하지 않는다.

Host admission 후 저장 실패에서도 과거 fixture lease가 항상 0회였다고 주장하지 않는다. 실제 제품 credential lease와 Authorization 수명은 G4/G7에서 검증한다. 취소·철회·만료 뒤 도착한 commit ACK는 미정산 증거를 갱신할 수 있어도 run을 다시 Running으로 만들거나 전송을 허용하지 않는다. G3 cleanup의 양쪽 owner 확인 또는 정당한 `NotRequired`는 원격 처리·환불·비용 정산 증거가 아니다.

## 7. 원자적 한도·시간·동시성 — PROPOSED

예약은 고유 attempt, 불투명 로컬 project scope, model·가격/정책 버전, 계산 단위와 UTC 귀속 기간에 결속한다. 요청당 금액, UTC 일·월 의무액, 요청/token rate를 **같은 transaction에서 전부 검사·갱신**한다. 하나라도 실패하면 부분 카운터 갱신을 남기지 않는다. Check와 commit 사이 다른 owner가 같은 잔여 예산을 소비하지 못하도록 실제 직렬화/잠금·stale owner 차단을 검증한다.

- 성공한 durable 예약을 로컬 요청 count 1로 계수하는 보수적 후보를 둔다. Token rate는 승인된 과금 input/output 상한과 프로젝트 정책을 대조한다. 취소로 count나 미정산액을 묵시적으로 되돌리지 않는다.
- 같은 attempt의 중복 입력은 기존 기록을 확인할 뿐 새 예약·새 전송 permit을 만들지 않는다. 같은 ID에 다른 금액·scope·정책이 오면 충돌로 거부한다. 재시작한 owner의 ID 재사용도 방지한다.
- 일/월 귀속은 durable 예약 시각의 UTC 시작 기간으로 고정하는 후보이며 공급자 청구서의 귀속과 같다고 가정하지 않는다. 자정·월 변경은 새 bucket을 고르는 일이며 기존 의무액 삭제가 아니다.
- **첫 통제 시험에서는 미정산이 남으면 새 전송을 차단하는 보수적 후보**를 둔다. 새 일/월이나 재시작으로 이 잠금을 해제하지 않는다. 다회 실행의 기간 간 hold/정산 정책은 별도 검토 전 활성화하지 않는다.
- 실행 중 rate/deadline에는 기존 단조 시계를 사용하고 UTC와의 관계를 대조한다. 시계 역행·큰 전진 점프·원장의 시간 연속성 불명은 새 예산 발급 근거가 아니며 admission을 닫는다. 이전 프로세스의 `Instant` 값을 복원하지 않는다.
- 정책·가격·UTC 신뢰 복구가 G3 철회 잠금이나 G5 예약을 초기화해서는 안 된다. 원자성·내구성·process lock·rollback 방어는 메모리 fixture와 OS 시험의 증거를 구분한다.

## 8. 미정산 상태와 재시작·조정 — PROPOSED

아래 이름은 개념 상태이며 현재 제품 API/enum이 아니다. 비용 상태는 Host terminal·cleanup 상태와 별개다.

| 상태·중단 지점 | 비용과 전송 처리 |
|---|---|
| Prepared | 메모리 계산/미리보기만 존재. 예약·전송 권한 없음 |
| Reservation pending | Host Queued와 저장 요청 존재. reader 생성 금지. 쓰기 지연/실패 중에도 취소·deadline 처리 |
| Reserved / intent committed | 정확한 예약/전송 의도의 commit 확인. 현재 owner의 최종 대조 전에는 전송 금지 |
| Commit ACK 유실·전송 직전 crash | 기록을 대조하기 전 잠금. 예약 재작성·과거 동의 복원·자동 재전송 금지. 완료 기록 부재를 미전송 증거로 보지 않음 |
| Outcome unknown | 부분 전송·취소·연결 중단·usage 누락/모순 등. 예약 유지, 처리/청구 종료를 선언하지 않음 |
| Usage reported | 검증된 집계와 추정액을 별도로 기록. 확정 청구가 아님. 예약보다 크면 초과 사실을 보존하고 새 admission 차단 |
| Reconciled | 소유자의 명시적 근거·조정 operation과 durable commit까지 확인. 과거 예약을 덮어쓰지 않고 조정 이력을 한 번만 반영 |

새 instance는 admission이 닫힌 상태에서 원장 연속성·단일 owner 권한·모든 미정산액을 복원한다. 동의·lease·전송 permit·옛 ACK는 복원하지 않는다. 누락·손상·구버전·rollback 의심을 빈 원장이나 잔액 초기화로 해석하지 않는다. 명백한 미전송의 예약 해제가 필요하더라도 owner/원장 연속성과 미전송을 입증하는 별도 조정 계약이 필요하다. 자동 환불은 없다.

조정은 가격/정책 버전·attempt·증거와 결속하고 중복 적용을 차단한다. Commit 실패·응답 불명은 기존 의무액과 잠금을 유지한다. 공급자 usage·추정액·확정 청구액을 다른 필드로 두며 확인되지 않은 청구액을 0으로 쓰지 않는다. 조정/압축 후에도 유효한 일/월 예산 집계, attempt 멱등성, 미정산 상태와 세대/rollback 연속성을 보존한다.

## 9. 최소 기록·개인정보·한도의 의미

**PROPOSED.** 원장에는 schema/record sequence, 연속성 기준, 로컬 불투명 project scope와 attempt, model·가격/정책 버전·단위, 과금 상한, UTC 기간, 예약액, 허용 usage 집계, 보고/확정/조정액과 상태, 비민감 조정 근거 참조만 둔다. 요청·응답 본문/그 digest, API Key·Authorization·raw error·동의 토큰은 넣지 않는다. 실제 계정/원격 ID 대응이 조정에 꼭 필요하면 제한된 별도 정보로 최소화하고 G6 접근/보존 정책을 따른다.

일반 진단·공개 Issue/PR에는 원장의 실제 attempt/slot/계정 ID를 내보내지 않는다. G6의 일반 증거 7일 후보나 대화 지우기를 미정산 원장·철회 기록에 적용하지 않는다. 확인된 정산만으로 기록 전체를 즉시 삭제하지 않으며 필요한 예산·멱등성·복구 연속성을 먼저 증명한다. 구체적인 저장 위치·형식·권한·보관/압축 기한·조정 담당은 미정이다.

공식 프로젝트 안내에는 조직/프로젝트 spend control과 `Enforce a hard limit` 선택지가 있으나 집행 지연으로 기록 사용액이 소폭 초과할 수 있고 알림만으로 요청이 중단되지는 않는다고 설명한다. [프로젝트 관리][O8]. 실제 설정은 조회/변경하지 않았다. AETHER의 로컬 예약 한도는 공급자 spend control과 별개이며, 다른 클라이언트의 사용·실효 설정·집계 지연을 통제하지 못하므로 **계정 전체의 확정 청구 hard cap을 보장하지 않는다.**

## 10. 후속 오프라인 수용시험 — 전체 NOT_RUN

비밀 없는 합성 가격·scope·메모리 저장 fixture부터 검증하되 실제 파일 내구성/프로세스 잠금의 증거와 구분한다. 아래 제품 시험 코드는 이번 문서에 추가하지 않았다.

| ID | 핵심 통과 기준 | 현재 |
|---|---|---|
| G5-T01 | 일반/read/write/TTL 분류와 합계 일치. 음수·중복·미지원 유료 항목 거부. write 총량을 재가산하지 않음 | NOT_RUN |
| G5-T02 | Reasoning 포함 output을 한 번만 계산. 빈 출력·incomplete·취소를 무료로 간주하지 않음 | NOT_RUN |
| G5-T03 | 극소 단가·분류별 올림 여유·분모 변환·최대 토큰·합계 overflow에서 과소 예약 0. float/saturation으로 정상 처리 금지 | NOT_RUN |
| G5-T04 | Cache hit 없음·가장 비싼 허용 write·context/tier/region 경계에서도 검증된 예약 상한 유지 | NOT_RUN |
| G5-T05 | Model·가격·tokenizer/overhead·scope·tier·유효기간 미확인/변경 시 pure preflight 거부와 옛 동의 폐기 | NOT_RUN |
| G5-T06 | 요청·일·월·rate 중 하나라도 초과하면 전체 예약 거부. 부분 카운터 갱신 0 | NOT_RUN |
| G5-T07 | 두 owner의 동일 잔여 예산 경쟁과 같은 attempt replay에서 초과 예약·중복 예약·중복 전송 permit 0 | NOT_RUN |
| G5-T08 | UTC 자정·월 경계·역행·큰 전진 점프·재시작으로 미정산액 소실/예산 재발급/잠금 해제 0 | NOT_RUN |
| G5-T09 | 예약/의도 durable commit 이전 reader·`Http::open` 0. Commit/ACK 유실 시 재전송 0 | NOT_RUN |
| G5-T10 | (a) 최초 예약 쓰기 대기 중 clock/cancel/철회 처리, reader·Provider 출력 0. 취소 또는 저장 실패로 종료하면 terminal 1, 늦은 ACK 전송 0. (b) 전송 후 usage/비용 기록이 지연·실패해도 이미 수락한 partial output을 보존한다. 두 경우 모두 저장 지연만으로 terminal을 만들지 않으며 종료 후 늦은 ACK로 terminal을 바꾸거나 재전송하지 않는다. | NOT_RUN |
| G5-T11 | 각 write 전후 crash·원장 누락/손상/구버전/rollback·stale owner에서 동의 복원·자동 환불·빈 원장 초기화로 전송 허용 0 | NOT_RUN |
| G5-T12 | Usage 누락/초과/모순·조정 commit 실패는 의무액 유지. 중복 조정·로그 TTL·대화 삭제가 예산/철회 상태를 초기화하지 않음 | NOT_RUN |

## 11. 후속 구현 후보와 완료 경계

문서 리뷰와 모델/가격 정책 결정을 거친 뒤 가장 작은 코드 후보는 **오프라인 비용 산술과 예약 상태 전이 모델**이다. 합성 가격·프로젝트·메모리 journal로 §10의 불변식을 먼저 검증하고 기존 Host/Synthetic/Credential 계약을 보존한다. 파일 내구성·process lock·서명 앱·실제 프로젝트 대조는 별도 통합 증거가 필요하다. 이 순서 제안은 다음 Issue나 구현의 착수 승인이 아니다.

제품 G5 통과에는 실제 모델/가격·tokenizer·예산 결정, 현재 요청의 불변 binding, 원자적 durable 예약/복구, 해당 환경의 수용 증거가 함께 필요하다. 일반 공개 가격표나 합성 산술 성공만으로 PASS 처리하지 않는다. G4/G6의 동의/보존 및 G9의 일회 실행 권한을 비용 계산 결과로 대신하지 않는다.

## 12. 이번 검증·보존·복구

이번 변경에서는 소스/계약 정적 대조, 공식 출처·조회일·가격 단위, 문서 산술 예시 재계산, 상대 링크·민감정보·범위, `python3 -B scripts/validate_governance.py`, whitespace와 GitHub Governance CI를 확인한다. 실제 명령·최종 HEAD·결과는 PR/Issue Evidence에 기록한다. 문서 예시 계산과 제품 수용시험은 구분한다.

Rust·Frontend·Typecheck·network-denied·TLS loopback·packaged GUI·signed release·§10 시험은 이번 문서 HEAD에서 **NOT_RUN**이다. 현재 Linux 작업에서 macOS P0 고정 앱과 원본 289개/추가 64·14·48개 증거를 새로 해시 검증하지 않았다. 접근 가능한 기존 worktree의 HEAD·미커밋/미추적 상태만 전후 대조하며 과거 Mac PASS를 새 실행으로 옮기지 않는다.

E5b/E5c **PARTIAL**, E5d ADR **PROPOSED**, 전역 Runtime 미선정을 유지한다. PR #71은 별도 Draft·미병합이다. 이 문서는 기존 Gate를 구체화하는 제안이며 새 Runtime·저장 기술·제품 보안 예외를 채택하지 않는다. 오류는 문서 수정으로 바로잡고 병합 후 필요 시 별도 승인된 문서 revert PR을 사용한다. 원장/credential 삭제·예산 초기화·dispatch 활성화는 복구 절차가 아니다. Draft PR 제출 후 최신 HEAD의 별도 승인 없이 병합하거나 다음 Issue를 시작하지 않는다.

[O1]: https://developers.openai.com/api/docs/models/gpt-5.4-mini.md
[O2]: https://developers.openai.com/api/docs/models/gpt-6-luna.md
[O3]: https://developers.openai.com/api/docs/pricing.md
[O4]: https://developers.openai.com/api/reference/resources/responses/methods/create.md
[O5]: https://developers.openai.com/api/docs/guides/prompt-caching.md
[O6]: https://developers.openai.com/api/docs/guides/token-counting.md
[O7]: https://developers.openai.com/api/docs/guides/reasoning.md
[O8]: https://help.openai.com/en/articles/9186755-managing-projects-in-the-api-platform
