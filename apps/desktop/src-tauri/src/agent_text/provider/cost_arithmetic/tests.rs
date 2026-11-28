use super::*;
use ClassPrice::{Allowed, Excluded, Unknown};

fn rate(n: u128, d: u128) -> Rate {
    Rate::new(n, d).unwrap()
}

fn prices(uncached: Rate, output: Rate) -> Prices {
    Prices {
        input: InputClasses { uncached: Allowed(uncached), cached: Excluded, write_30m: Excluded, write_12h: Excluded },
        output: Some(output),
        extras: Some(ExtraCharges::Excluded),
    }
}

fn basic() -> Prices {
    prices(rate(1, 1), rate(2, 1))
}

fn usage(input: u128, output: u128) -> Usage {
    Usage { input, cached: 0, write_30m: 0, write_12h: 0, total_writes: None, output, reasoning: 0,
        total_tokens: input.checked_add(output).unwrap(), extra_micro_usd: Some(0) }
}

fn caps(input: u128, output: u128) -> Caps { Caps { input, output } }

#[test]
fn uncached_only_exact_cost() {
    let result = basic().estimate(caps(8, 0), usage(8, 0)).unwrap();
    assert_eq!(result, Estimate { reported_micro_usd: 8, bound_micro_usd: 8, exceeded: false });
}

#[test]
fn read_and_two_write_ttls_replace_uncached_tokens() {
    let mut p = basic();
    p.input.cached = Allowed(rate(2, 1));
    p.input.write_30m = Allowed(rate(3, 1));
    p.input.write_12h = Allowed(rate(4, 1));
    let mut u = Usage { cached: 1, write_30m: 2, write_12h: 3, ..usage(10, 0) };
    assert_eq!(p.estimate(caps(10, 0), u).unwrap().reported_micro_usd, 24);
    u.total_writes = Some(5);
    assert_eq!(p.estimate(caps(10, 0), u).unwrap().reported_micro_usd, 24);
}

#[test]
fn reasoning_only_output_is_billed_once() {
    let u = Usage { reasoning: 7, ..usage(0, 7) };
    assert_eq!(basic().estimate(caps(0, 7), u).unwrap().reported_micro_usd, 14);
}

#[test]
fn zero_usage_and_explicitly_free_rates() {
    let p = prices(rate(0, 3), rate(0, 1));
    for u in [usage(0, 0), usage(9, 3)] {
        assert_eq!(p.estimate(caps(9, 3), u).unwrap(),
            Estimate { reported_micro_usd: 0, bound_micro_usd: 0, exceeded: false });
    }
}

#[test]
fn documented_synthetic_example() {
    let p = Prices {
        input: InputClasses { uncached: Allowed(rate(1, 1)), cached: Allowed(rate(1, 10)),
            write_30m: Allowed(rate(5, 4)), write_12h: Allowed(rate(5, 2)) },
        output: Some(rate(4, 1)), extras: Some(ExtraCharges::Capped(0)),
    };
    let u = Usage { cached: 20, write_30m: 30, write_12h: 10, total_writes: Some(40),
        reasoning: 4, ..usage(100, 10) };
    let result = p.estimate(caps(128, 64), u).unwrap();
    assert_eq!(result, Estimate { reported_micro_usd: 145, bound_micro_usd: 579, exceeded: false });
    assert_eq!(result.bound_micro_usd.checked_sub(result.reported_micro_usd), Some(434));
}

#[test]
fn per_class_rounding_needs_k_minus_one() {
    let mut p = prices(rate(1, 5), rate(0, 1));
    p.input.cached = Allowed(rate(1, 10));
    let u = Usage { cached: 1, ..usage(2, 0) };
    let result = p.estimate(caps(2, 0), u).unwrap();
    assert_eq!(rate(3, 10).charge(1), Ok(1)); // Round the exact sum once.
    let no_headroom = rate(1, 5).charge(2).unwrap();
    assert_eq!(no_headroom, 1);
    assert_eq!(result.reported_micro_usd, 2);
    assert!(result.reported_micro_usd > no_headroom);
    assert_eq!(result.bound_micro_usd, no_headroom + 1);
}

#[test]
fn zero_denominator_is_invalid_even_for_free_tokens() {
    assert_eq!(Rate::new(0, 0), Err(CostError::ZeroDenominator));
}

#[test]
fn overlapping_or_excess_input_counts_rejected() {
    for u in [Usage { cached: 3, ..usage(2, 0) }, Usage { cached: 1, write_30m: 1, write_12h: 1, ..usage(2, 0) }] {
        assert_eq!(basic().estimate(caps(2, 0), u), Err(CostError::InconsistentUsage));
    }
}

#[test]
fn conflicting_write_aggregate_rejected() {
    let u = Usage { write_12h: 2, total_writes: Some(1), ..usage(3, 0) };
    assert_eq!(basic().estimate(caps(3, 0), u), Err(CostError::InconsistentUsage));
}

#[test]
fn reasoning_and_token_totals_checked() {
    for u in [Usage { reasoning: 4, ..usage(2, 3) }, Usage { total_tokens: 6, ..usage(2, 3) }] {
        assert_eq!(basic().estimate(caps(2, 3), u), Err(CostError::InconsistentUsage));
    }
}

#[test]
fn unsupported_class_and_extra_charge_rejected() {
    for u in [Usage { cached: 1, ..usage(2, 0) }, Usage { extra_micro_usd: Some(1), ..usage(2, 0) }] {
        assert_eq!(basic().estimate(caps(2, 0), u), Err(CostError::UnsupportedCharge));
    }
}

#[test]
fn unknown_price_or_extra_never_defaults_to_zero() {
    let mut p = basic();
    p.input.cached = Unknown;
    for unknown in [p, Prices { output: None, ..basic() }, Prices { extras: None, ..basic() }] {
        assert_eq!(unknown.reservation_bound(caps(0, 0)), Err(CostError::Unpriced));
    }
    let u = Usage { extra_micro_usd: None, ..usage(0, 0) };
    assert_eq!(basic().estimate(caps(0, 0), u), Err(CostError::Unpriced));
}

#[test]
fn empty_input_allowlist_rejected() {
    let mut p = basic();
    p.input.uncached = Excluded;
    assert_eq!(p.reservation_bound(caps(0, 0)), Err(CostError::NoInputClasses));
}

#[test]
fn exact_ceil_handles_max_numerator_without_adding_denominator() {
    assert_eq!(rate(1, 2).charge(u128::MAX), Ok((u128::MAX / 2) + 1));
    assert_eq!(rate(u128::MAX, u128::MAX).charge(1), Ok(1));
}

#[test]
fn multiplication_overflow_returns_error() {
    assert_eq!(rate(u128::MAX, 2).charge(2), Err(CostError::Overflow));
}

#[test]
fn rational_comparison_overflow_returns_error_in_both_orders() {
    for (a,b) in [(rate(u128::MAX, 1), rate(1, 2)), (rate(1, 2), rate(u128::MAX, 1))] {
        let mut p = prices(a, rate(0, 1));
        p.input.write_12h = Allowed(b);
        assert_eq!(p.reservation_bound(caps(1, 0)), Err(CostError::Overflow));
    }
}

#[test]
fn token_aggregate_overflows_return_error() {
    for u in [Usage { input: u128::MAX, output: 1, ..usage(0, 0) },
              Usage { write_30m: u128::MAX, write_12h: 1, ..usage(0, 0) }] {
        assert_eq!(basic().estimate(caps(1, 1), u), Err(CostError::Overflow));
    }
}

#[test]
fn reported_line_item_sum_overflow_returns_error() {
    let p = prices(rate(u128::MAX, 1), rate(1, 1));
    assert_eq!(p.estimate(caps(0, 0), usage(1, 1)), Err(CostError::Overflow));
}

#[test]
fn worst_case_product_headroom_and_extra_overflow_return_errors() {
    let mut p = prices(rate(u128::MAX, 1), rate(0, 1));
    assert_eq!(p.reservation_bound(caps(2, 0)), Err(CostError::Overflow));
    p.extras = Some(ExtraCharges::Capped(1));
    assert_eq!(p.reservation_bound(caps(1, 0)), Err(CostError::Overflow));
    p.extras = Some(ExtraCharges::Excluded);
    p.input.cached = Allowed(rate(0, 1));
    assert_eq!(p.reservation_bound(caps(1, 0)), Err(CostError::Overflow));
}

#[test]
fn valid_overruns_keep_the_full_estimate_and_flag_each_cap() {
    let p = Prices { extras: Some(ExtraCharges::Capped(3)), ..basic() };
    for (u, expected) in [(usage(5, 0), 5), (usage(0, 5), 10),
        (Usage { extra_micro_usd: Some(4), ..usage(0, 0) }, 4),
        (Usage { extra_micro_usd: Some(5), ..usage(5, 5) }, 20)] {
        let result = p.estimate(caps(2, 2), u).unwrap();
        assert_eq!(result, Estimate { reported_micro_usd: expected, bound_micro_usd: 9, exceeded: true });
    }
}

fn input_distributions(cap: u128) -> Vec<[u128; 4]> {
    let mut samples = Vec::new();
    for u in 0..=cap {
        for c in 0..=cap - u {
            for w30 in 0..=cap - u - c {
                for w12 in 0..=cap - u - c - w30 {
                    samples.push([u, c, w30, w12]);
                }
            }
        }
    }
    samples
}

#[test]
fn finite_bound_invariant_for_all_nonempty_class_subsets() {
    // All denominators in these two sets divide 60. Use sixtieths as an
    // independent oracle for the maximum, the per-line ceilings and K.
    let schemes = [
        [rate(1, 5), rate(1, 10), rate(5, 4), rate(5, 2)],
        [rate(5, 3), rate(0, 1), rate(1, 2), rate(1, 3)],
    ];
    let distributions = input_distributions(4);
    let mut cases = 0;
    for mask in 1u8..16 {
        for scheme in schemes {
            let enabled = |i: usize| mask & (1 << i) != 0;
            let [uncached, cached, write_30m, write_12h] = std::array::from_fn(|i| {
                if enabled(i) { Allowed(scheme[i]) } else { Excluded }
            });
            let units = scheme.map(|p| p.numerator * (60 / p.denominator.get()));
            let max_units = (0..4).filter(|&i| enabled(i)).map(|i| units[i]).max().unwrap();
            let headroom = u128::from(mask.count_ones() - 1);
            for extra_cap in [0, 2] {
                let p = Prices {
                    input: InputClasses { uncached, cached, write_30m, write_12h },
                    output: Some(rate(3, 2)),
                    extras: Some(ExtraCharges::Capped(extra_cap)),
                };
                let bound = (4 * max_units).div_ceil(60) + headroom + 5 + extra_cap;
                assert_eq!(p.reservation_bound(caps(4, 3)), Ok(bound));
                for &counts in &distributions {
                    if (0..4).any(|i| counts[i] != 0 && !enabled(i)) {
                        continue;
                    }
                    let [_, cached, write_30m, write_12h] = counts;
                    let input = counts.iter().sum();
                    let input_cost: u128 = (0..4).map(|i| (counts[i] * units[i]).div_ceil(60)).sum();
                    for output in 0..=3 {
                        for extra in 0..=extra_cap {
                            let sample = Usage {
                                cached, write_30m, write_12h, extra_micro_usd: Some(extra),
                                ..usage(input, output)
                            };
                            let result = p.estimate(caps(4, 3), sample).unwrap();
                            assert_eq!(result.reported_micro_usd, input_cost + (3 * output).div_ceil(2) + extra);
                            assert!(!result.exceeded);
                            assert!(result.reported_micro_usd <= result.bound_micro_usd);
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 10_240);
}
