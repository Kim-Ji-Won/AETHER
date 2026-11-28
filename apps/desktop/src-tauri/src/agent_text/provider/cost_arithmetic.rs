//! Pure synthetic micro-USD arithmetic, compiled only by the provider's test module declaration.
use std::cmp::Ordering;
use std::num::NonZeroU128;

#[derive(Debug, PartialEq, Eq)]
enum CostError {
    ZeroDenominator,
    Overflow,
    InconsistentUsage,
    Unpriced,
    UnsupportedCharge,
    NoInputClasses,
}

type CostResult<T> = Result<T, CostError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rate {
    numerator: u128,
    denominator: NonZeroU128,
}

impl Rate {
    fn new(numerator: u128, denominator: u128) -> CostResult<Self> {
        let denominator = NonZeroU128::new(denominator).ok_or(CostError::ZeroDenominator)?;
        Ok(Self { numerator, denominator })
    }

    fn charge(self, tokens: u128) -> CostResult<u128> {
        let n = tokens.checked_mul(self.numerator).ok_or(CostError::Overflow)?;
        let d = self.denominator.get();
        let whole = n.checked_div(d).ok_or(CostError::ZeroDenominator)?;
        whole.checked_add(u128::from(n % d != 0)).ok_or(CostError::Overflow)
    }

    fn compare(self, other: Self) -> CostResult<Ordering> {
        if self.denominator == other.denominator {
            return Ok(self.numerator.cmp(&other.numerator));
        }
        let left = self.numerator.checked_mul(other.denominator.get()).ok_or(CostError::Overflow)?;
        let right = other.numerator.checked_mul(self.denominator.get()).ok_or(CostError::Overflow)?;
        Ok(left.cmp(&right))
    }
}

#[derive(Clone, Copy)]
enum ClassPrice {
    Excluded,
    Allowed(Rate),
    Unknown,
}

#[derive(Clone, Copy)]
struct InputClasses<T> {
    uncached: T,
    cached: T,
    write_30m: T,
    write_12h: T,
}

impl<T> InputClasses<T> {
    fn values(self) -> [T; 4] {
        [self.uncached, self.cached, self.write_30m, self.write_12h]
    }
}

#[derive(Clone, Copy)]
enum ExtraCharges {
    Excluded,
    Capped(u128),
}

#[derive(Clone, Copy)]
struct Prices {
    input: InputClasses<ClassPrice>,
    output: Option<Rate>,
    extras: Option<ExtraCharges>,
}

#[derive(Clone, Copy)]
struct Caps {
    input: u128,
    output: u128,
}

/// Every count is explicit. The optional write aggregate is only a cross-check.
#[derive(Clone, Copy)]
struct Usage {
    input: u128,
    cached: u128,
    write_30m: u128,
    write_12h: u128,
    total_writes: Option<u128>,
    output: u128,
    reasoning: u128,
    total_tokens: u128,
    extra_micro_usd: Option<u128>,
}

impl Usage {
    fn classified(self) -> CostResult<InputClasses<u128>> {
        let writes = self.write_30m.checked_add(self.write_12h).ok_or(CostError::Overflow)?;
        let total = self.input.checked_add(self.output).ok_or(CostError::Overflow)?;
        if self.total_writes.is_some_and(|reported| reported != writes)
            || self.total_tokens != total || self.reasoning > self.output
        {
            return Err(CostError::InconsistentUsage);
        }
        let uncached = self.input.checked_sub(self.cached)
            .and_then(|remainder| remainder.checked_sub(writes))
            .ok_or(CostError::InconsistentUsage)?;
        Ok(InputClasses { uncached, cached: self.cached, write_30m: self.write_30m, write_12h: self.write_12h })
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Estimate {
    reported_micro_usd: u128,
    bound_micro_usd: u128,
    exceeded: bool,
}

impl Prices {
    /// The maximum rate and the rounding allowance come from the same allow-list.
    fn reservation_bound(self, caps: Caps) -> CostResult<u128> {
        let mut maximum: Option<Rate> = None;
        let mut classes = 0u128;
        for class in self.input.values() {
            let rate = match class {
                ClassPrice::Excluded => continue,
                ClassPrice::Unknown => return Err(CostError::Unpriced),
                ClassPrice::Allowed(rate) => rate,
            };
            classes = classes.checked_add(1).ok_or(CostError::Overflow)?;
            maximum = Some(match maximum {
                Some(previous) if rate.compare(previous)? != Ordering::Greater => previous,
                _ => rate,
            });
        }
        let maximum = maximum.ok_or(CostError::NoInputClasses)?;
        let headroom = classes.checked_sub(1).ok_or(CostError::NoInputClasses)?;
        let input = maximum.charge(caps.input)?;
        let output = self.output.ok_or(CostError::Unpriced)?.charge(caps.output)?;
        let extra_cap = match self.extras.ok_or(CostError::Unpriced)? {
            ExtraCharges::Excluded => 0,
            ExtraCharges::Capped(cap) => cap,
        };
        [input, headroom, output, extra_cap].into_iter().try_fold(0, add_cost)
    }

    fn estimate(self, caps: Caps, usage: Usage) -> CostResult<Estimate> {
        let counts = usage.classified()?;
        let bound = self.reservation_bound(caps)?;
        let extra = usage.extra_micro_usd.ok_or(CostError::Unpriced)?;
        let extra_exceeded = match self.extras.ok_or(CostError::Unpriced)? {
            ExtraCharges::Excluded if extra != 0 => return Err(CostError::UnsupportedCharge),
            ExtraCharges::Excluded => false,
            ExtraCharges::Capped(cap) => extra > cap,
        };
        let mut reported = add_cost(extra, self.output.ok_or(CostError::Unpriced)?.charge(usage.output)?)?;
        for (price, tokens) in self.input.values().into_iter().zip(counts.values()) {
            match price {
                ClassPrice::Allowed(rate) => reported = add_cost(reported, rate.charge(tokens)?)?,
                ClassPrice::Excluded if tokens == 0 => (),
                ClassPrice::Excluded => return Err(CostError::UnsupportedCharge),
                ClassPrice::Unknown => return Err(CostError::Unpriced),
            }
        }
        Ok(Estimate {
            reported_micro_usd: reported,
            bound_micro_usd: bound,
            exceeded: usage.input > caps.input || usage.output > caps.output || extra_exceeded || reported > bound,
        })
    }
}

fn add_cost(a: u128, b: u128) -> CostResult<u128> {
    a.checked_add(b).ok_or(CostError::Overflow)
}

mod tests;
