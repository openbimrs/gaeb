#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Decimal {
    units: i128,
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
        let mut units = digits.parse::<i128>().ok()?;
        if negative {
            units = -units;
        }
        Some(Self {
            units,
            scale: u32::try_from(fraction.len()).ok()?,
        })
    }

    pub(super) fn add(self, other: Self) -> Option<Self> {
        let scale = self.scale.max(other.scale);
        let left = self.rescale_exact(scale)?;
        let right = other.rescale_exact(scale)?;
        Some(Self {
            units: left.checked_add(right)?,
            scale,
        })
    }

    pub(super) fn multiply_rounded(self, other: Self, scale: u32) -> Option<Self> {
        let units = self.units.checked_mul(other.units)?;
        Self {
            units,
            scale: self.scale.checked_add(other.scale)?,
        }
        .round(scale)
    }

    pub(super) fn multiply_discounted_rounded(
        self,
        other: Self,
        discount_percent: Self,
        scale: u32,
    ) -> Option<Self> {
        let percent_scale = discount_percent.scale;
        let hundred = 100_i128.checked_mul(pow10(percent_scale)?)?;
        let complement = hundred.checked_sub(discount_percent.units)?;
        if complement < 0 {
            return None;
        }
        Self {
            units: self
                .units
                .checked_mul(other.units)?
                .checked_mul(complement)?,
            scale: self
                .scale
                .checked_add(other.scale)?
                .checked_add(percent_scale)?
                .checked_add(2)?,
        }
        .round(scale)
    }

    pub(super) fn equals_at(self, other: Self, scale: u32) -> bool {
        self.round(scale)
            .zip(other.round(scale))
            .is_some_and(|(left, right)| left == right)
    }

    pub(super) fn scale(self) -> u32 {
        self.scale
    }

    fn rescale_exact(self, scale: u32) -> Option<i128> {
        if scale < self.scale {
            return None;
        }
        self.units.checked_mul(pow10(scale - self.scale)?)
    }

    fn round(self, scale: u32) -> Option<Self> {
        if self.scale <= scale {
            return Some(Self {
                units: self.rescale_exact(scale)?,
                scale,
            });
        }
        let divisor = pow10(self.scale - scale)?;
        let quotient = self.units / divisor;
        let remainder = self.units % divisor;
        let increment = if remainder.abs().checked_mul(2)? >= divisor {
            self.units.signum()
        } else {
            0
        };
        Some(Self {
            units: quotient.checked_add(increment)?,
            scale,
        })
    }
}

fn pow10(power: u32) -> Option<i128> {
    10_i128.checked_pow(power)
}
