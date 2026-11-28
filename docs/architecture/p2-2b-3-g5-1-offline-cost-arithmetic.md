# G5.1 합성 비용 산술 · 오프라인 검증

[Issue #76](https://github.com/Kim-Ji-Won/AETHER/issues/76)의 기준은 `a435183b9f7b02b48c96c1a632a54fda705ac531`이다. [Gate 0](p2-2b-3-live-integration-preflight.md)와 [G5 §5·§10·§11](p2-2b-3-g5-model-cost-preflight.md)에 따라 사용자가 먼저 승인한 합성 산술만 검증했다. 제품 G5와 첫 실제 요청은 **BLOCKED**다. 실제 모델·프로젝트·가격·예산, 모델별 usage 분류, tokenizer와 과금 상한은 여전히 미결정이다.

## 입력과 결과

[산술 모듈](../../apps/desktop/src-tauri/src/agent_text/provider/cost_arithmetic.rs)은 Provider의 비공개 `#[cfg(test)]` 선언으로만 컴파일된다. Host, mapper, adapter, consent, 기존 `OfflinePolicy`와 `Ledger`는 이 모듈을 호출하지 않는다. 제품의 `DisabledCredentials`와 `ProviderDisabled`, 기존 UTF-8 byte·가상 정수 정책을 유지한다.

- `Rate::new`는 micro-USD/token 분수의 분자와 분모를 받는다. 합성 token·금액·중간 곱과 합에 같은 `u128`을 사용한다. 음수는 API 입력 타입에 표현할 수 없고 분모 0은 거부한다. 이 폭은 합성 예시와 최대 정수 경계를 함께 시험하기 위한 선택이다.
- `Usage`의 각 수치는 호출자가 명시한다. 입력은 배타적인 일반·read·write 30m·write 12h 분류를 가정한다. 두 write를 checked add하고 전체에서 read와 write를 checked sub해 일반 입력을 구한다. 선택적 전체 write는 제공된 경우 검산에만 쓴다. `R ≤ O`와 `I + O = total`을 확인하며 출력 비용에는 `O` 한 번만 반영한다.
- `Prices::reservation_bound`는 동일한 허용 분류 목록에서 최대 분수 단가와 개수 K를 구한다. `ceil(Icap × Pmax) + (K−1) + ceil(Ocap × Po) + Ecap`을 계산한다. 비교에는 같은 분모의 분자 비교 또는 checked 교차곱을 쓴다. 각 올림은 몫과 나머지를 사용한다. 곱·합·비교의 중간값이 표현 범위를 벗어나면 계산을 중단한다.
- `Prices::estimate`는 각 입력 분류와 출력을 따로 올린 뒤 합성 추가 금액 E를 더한다. 추가 요금은 제외 또는 단일 명시 상한으로 모델링한다. 제외된 항목의 양수 사용량, 미확인 단가·상한·금액은 거부한다. 허용된 항목에서 유효한 초과 사용이 발생하면 추정액 전체와 상한 및 `exceeded`를 함께 반환한다. 금액이 상한 이하라도 input·output·추가 요금 cap 위반을 표시한다.

| 오류 | 발생 조건 |
|---|---|
| `ZeroDenominator` | 가격 분모 0 |
| `Unpriced` / `NoInputClasses` | 미확인 가격·추가 요금 또는 허용 입력 분류 없음 |
| `UnsupportedCharge` | 명시적으로 제외된 분류에 양수 사용량이나 추가 금액 보고 |
| `InconsistentUsage` | 분류 합계·reasoning·전체 token 불일치 |
| `Overflow` | checked 산술 또는 분수 교차곱을 표현할 수 없음 |

결과는 상태를 바꾸지 않는 합성 추정치다. 예산 예약·차감·자동 환불·청구 확정·전송 허가는 생성하지 않는다. [G3 오프라인 철회](p2-2b-3-g3-offline-revocation.md) 및 [G6 데이터 정책](p2-2b-3-g6-data-minimization-retention.md)의 후속 조건도 유지한다.

## G5 시험 범위

| 기준 | 이 Fixture가 확인하는 부분 | 남은 제품 검증 |
|---|---|---|
| G5-T01 | 배타적 합성 분류, 두 TTL의 합, 전체 token, 미지원·미확인 항목 거부 | 실제 모델 usage schema와 누락·중복 의미 |
| G5-T02 | 전부 reasoning인 출력 및 일부 reasoning을 전체 과금 출력 안에서 한 번 계산 | 실제 incomplete·취소·원격 청구 |
| G5-T03 | 항목별 올림, K−1, 유리수 비교, 최대 정수와 overflow 거부 | 제품 가격표 변환·실제 청구 반올림 |
| G5-T04 | cache hit 0부터 허용된 가장 비싼 입력까지 유한한 합성 집합의 상한 | 실제 context 구간, tier·region·모델 및 추가 비용 정책 |
| G5-T05~T12 | **NOT_RUN** | binding·예약 원장·복구·전송·조정 |

[집중 시험](../../apps/desktop/src-tauri/src/agent_text/provider/cost_arithmetic/tests.rs)은 G5 §5 예시의 추정 145, 상한 579, 차이 434 micro-USD와 두 소액을 각각 올린 2 대 한 번 올린 1을 검산한다. 유한 시험은 비어 있지 않은 허용 목록 15개(K=1~4), 가격 세트 2개, input 합 0~4, output 0~3, Ecap 0·2에서 가능한 E를 열거한다. **10,240개** 조합에서 독립적인 1/60 micro-USD 정수 기준으로 분류별 추정액·정확한 상한을 대조하고 `reported ≤ bound`를 확인한다. 이 결과는 명시된 입력 집합에 한정된다.

## macOS 검증

macOS 26.6 arm64, SDK 26.2, Rust/Cargo 1.99.1. 기존 pinned sidecar와 LICENSE를 verify-only로 검증했고 Issue worktree 전용 Cargo 출력과 Swift 모듈 캐시를 사용했다.

| 실행 | 결과 |
|---|---|
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --offline --locked --lib agent_text::provider::cost_arithmetic::` | **21/21 PASS** |
| 같은 manifest의 전체 `cargo test --offline --locked` | **219/219 PASS**. Library 175, 앱 binary 42, compile-fail doctest 2 |
| 기존 집중 실행 | G3 25, Provider 49, Credential/Consent 19, Transport 15, Host/Synthetic 46, 앱 Runtime 23 모두 PASS |
| 로컬 TLS loopback | **5/5 PASS**. 기존 Transport 시험 포함 |
| OS network-denied | **193/193 PASS**. Loopback을 제외한 library 170, 앱 Runtime 23 |

집중 실행의 실제 통과 목록은 다음과 같다.

- `conflicting_write_aggregate_rejected`
- `documented_synthetic_example`
- `empty_input_allowlist_rejected`
- `exact_ceil_handles_max_numerator_without_adding_denominator`
- `finite_bound_invariant_for_all_nonempty_class_subsets`
- `multiplication_overflow_returns_error`
- `overlapping_or_excess_input_counts_rejected`
- `per_class_rounding_needs_k_minus_one`
- `rational_comparison_overflow_returns_error_in_both_orders`
- `read_and_two_write_ttls_replace_uncached_tokens`
- `reasoning_and_token_totals_checked`
- `reasoning_only_output_is_billed_once`
- `reported_line_item_sum_overflow_returns_error`
- `token_aggregate_overflows_return_error`
- `uncached_only_exact_cost`
- `unknown_price_or_extra_never_defaults_to_zero`
- `unsupported_class_and_extra_charge_rejected`
- `valid_overruns_keep_the_full_estimate_and_flag_each_cap`
- `worst_case_product_headroom_and_extra_overflow_return_errors`
- `zero_denominator_is_invalid_even_for_free_tokens`
- `zero_usage_and_explicitly_free_rates`

OS 차단은 테스트 실행 파일에 `sandbox-exec -p '(version 1)(allow default)(deny network*)'`를 적용했다. 같은 프로필에서 loopback bind 거부를 별도로 확인했다. TLS 5개는 로컬 peer를 허용한 별도 실행이다. Cargo `--offline --locked`는 의존성 해석·다운로드 조건을 고정한다. 세 검증의 범위와 중복 실행 수를 분리한다.

Frontend·Typecheck·packaged GUI·signed release는 **NOT_RUN**이다. 기존 Linux 기록은 TLS 포함 153 PASS / 1 FAIL로 유지한다. 실제 Provider·Credential·Keychain·usage/billing 접근 및 과금은 각각 0회다.

## 복구

병합 전에는 이 Draft PR을 보류할 수 있다. 병합 이후 복구가 필요하면 해당 변경만 되돌리는 별도 승인 PR을 사용한다. 제품 원장·저장 데이터에 적용할 migration은 없다.
