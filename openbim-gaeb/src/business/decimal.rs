use num_bigint::{BigInt, Sign};

const MAX_DECIMAL_DIGITS: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Decimal {
    units: BigInt,
    scale: u32,
}

impl Decimal {
    pub(super) fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        let (negative, unsigned) = match value.as_bytes().first()? {
            b'-' => (true, &value[1..]),
            b'+' => (false, &value[1..]),
            _ => (false, value),
        };
        if unsigned.is_empty() {
            return None;
        }
        let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        if whole.is_empty() && fraction.is_empty() {
            return None;
        }
        if !whole
            .bytes()
            .chain(fraction.bytes())
            .all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        let digits = format!("{whole}{fraction}");
        if digits.len() > MAX_DECIMAL_DIGITS {
            return None;
        }
        let mut units = BigInt::parse_bytes(digits.as_bytes(), 10)?;
        if negative {
            units = -units;
        }
        Some(Self {
            units,
            scale: u32::try_from(fraction.len()).ok()?,
        })
    }

    pub(super) fn add(&self, other: &Self) -> Option<Self> {
        let scale = self.scale.max(other.scale);
        let left = self.rescale_exact(scale)?;
        let right = other.rescale_exact(scale)?;
        Some(Self {
            units: left + right,
            scale,
        })
    }

    pub(super) fn multiply_rounded(&self, other: &Self, scale: u32) -> Option<Self> {
        Self {
            units: &self.units * &other.units,
            scale: self.scale.checked_add(other.scale)?,
        }
        .round(scale)
    }

    pub(super) fn multiply_discounted_rounded(
        &self,
        other: &Self,
        discount_percent: &Self,
        scale: u32,
    ) -> Option<Self> {
        let percent_scale = discount_percent.scale;
        let hundred = BigInt::from(100_u8) * pow10(percent_scale)?;
        let complement = hundred - &discount_percent.units;
        if complement.sign() == Sign::Minus {
            return None;
        }
        Self {
            units: &self.units * &other.units * complement,
            scale: self
                .scale
                .checked_add(other.scale)?
                .checked_add(percent_scale)?
                .checked_add(2)?,
        }
        .round(scale)
    }

    pub(super) fn equals_at(&self, other: &Self, scale: u32) -> bool {
        self.round(scale)
            .zip(other.round(scale))
            .is_some_and(|(left, right)| left == right)
    }

    pub(super) fn scale(&self) -> u32 {
        self.scale
    }

    fn rescale_exact(&self, scale: u32) -> Option<BigInt> {
        if scale < self.scale {
            return None;
        }
        Some(&self.units * pow10(scale - self.scale)?)
    }

    fn round(&self, scale: u32) -> Option<Self> {
        if self.scale <= scale {
            return Some(Self {
                units: self.rescale_exact(scale)?,
                scale,
            });
        }
        let divisor = pow10(self.scale - scale)?;
        let quotient = &self.units / &divisor;
        let remainder = &self.units % &divisor;
        let magnitude = if remainder.sign() == Sign::Minus {
            -remainder
        } else {
            remainder
        };
        let increment = if magnitude * 2_u8 >= divisor {
            match self.units.sign() {
                Sign::Minus => BigInt::from(-1_i8),
                Sign::NoSign => BigInt::from(0_u8),
                Sign::Plus => BigInt::from(1_u8),
            }
        } else {
            BigInt::from(0_u8)
        };
        Some(Self {
            units: quotient + increment,
            scale,
        })
    }
}

fn pow10(power: u32) -> Option<BigInt> {
    if usize::try_from(power).ok()? > MAX_DECIMAL_DIGITS * 3 + 2 {
        return None;
    }
    Some(BigInt::from(10_u8).pow(power))
}
