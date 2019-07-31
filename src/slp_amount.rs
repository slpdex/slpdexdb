use std::num::ParseIntError;
use std::ops::{Add, Sub, Mul, Div, AddAssign, SubAssign, MulAssign, DivAssign};
use std::iter::Sum;
use std::io::{Cursor, self};
use std::cmp::Ordering;
use diesel::data_types::PgNumeric;
use crate::convert_numeric::{i128_to_pg_numeric, pg_numeric_to_i128};
use std::{fmt, fmt::{Formatter, Display}};
use byteorder::{BigEndian, ReadBytesExt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SLPAmount {
    base_amount: i128,
    decimals: u32,
}

impl SLPAmount {
    pub fn from_str_decimals(s: &str, decimals: u32) -> Result<Self, ParseIntError> {
        let factor: i128 = (10i128).pow(decimals);
        let base_amount = match s.find(".") {
            Some(dot_idx) => {
                let integer_part = s[..dot_idx].parse::<i128>()? * factor;
                let fract_part_str = &s[dot_idx + 1..];
                let preceding_zeros = fract_part_str.chars()
                    .take_while(|c| *c == '0')
                    .count();
                let fract_part_str = if fract_part_str.len() > decimals as usize {
                    &fract_part_str[..decimals as usize]
                } else {
                    fract_part_str
                };
                let num_decimals = fract_part_str.len() as u32;
                let fract_part_str = &fract_part_str[preceding_zeros..];
                if fract_part_str.len() == 0 {
                    integer_part
                } else {
                    let fract_part = fract_part_str.parse::<i128>()?;
                    let factor = (10i128).pow(decimals - num_decimals);
                    integer_part + fract_part * factor
                }
            },
            None => s.parse::<i128>()? * factor,
        };
        Ok(SLPAmount { base_amount, decimals })
    }

    pub fn new(base_amount: i128, decimals: u32) -> Self {
        SLPAmount { base_amount, decimals }
    }

    pub fn from_slice(slice: &[u8], decimals: u32) -> io::Result<Self> {
        Ok(SLPAmount {
            base_amount: Cursor::new(slice).read_u64::<BigEndian>()? as i128,
            decimals,
        })
    }

    pub fn from_numeric_decimals(numeric: &PgNumeric, decimals: u32) -> Self {
        SLPAmount {
            base_amount: pg_numeric_to_i128(&numeric).unwrap_or(0),
            decimals,
        }
    }

    pub fn decimals(&self) -> u32 {
        self.decimals
    }

    pub fn base_amount(&self) -> i128 {
        self.base_amount
    }

    fn _op(&self, other: Self, f: impl Fn(i128, i128) -> i128) -> SLPAmount {
        if self.decimals != other.decimals {
            panic!(format!(
                "Operating on incompatible tokens: decimals {} != {}",
                self.decimals,
                other.decimals,
            ));
        }
        SLPAmount {
            decimals: self.decimals,
            base_amount: f(self.base_amount, other.base_amount),
        }
    }

    pub fn map(&self, f: impl FnOnce(i128) -> i128) -> SLPAmount {
        SLPAmount {
            decimals: self.decimals,
            base_amount: f(self.base_amount),
        }
    }
}

impl Add for SLPAmount {
    type Output = SLPAmount;

    fn add(self, rhs: SLPAmount) -> Self::Output {
        self._op(rhs, |a, b| a + b)
    }
}

impl AddAssign for SLPAmount {
    fn add_assign(&mut self, rhs: Self) {
        *self = self._op(rhs, |a, b| a + b);
    }
}

impl Sub for SLPAmount {
    type Output = SLPAmount;

    fn sub(self, rhs: SLPAmount) -> Self::Output {
        self._op(rhs, |a, b| a - b)
    }
}

impl SubAssign for SLPAmount {
    fn sub_assign(&mut self, rhs: Self) {
        *self = self._op(rhs, |a, b| a - b);
    }
}

impl Mul<i128> for SLPAmount {
    type Output = SLPAmount;

    fn mul(self, rhs: i128) -> Self::Output {
        self.map(|a| a * rhs)
    }
}

impl MulAssign<i128> for SLPAmount {
    fn mul_assign(&mut self, rhs: i128) {
        *self = self.map(|a| a * rhs);
    }
}

impl Div<i128> for SLPAmount {
    type Output = SLPAmount;

    fn div(self, rhs: i128) -> Self::Output {
        self.map(|a| a / rhs)
    }
}

impl DivAssign<i128> for SLPAmount {
    fn div_assign(&mut self, rhs: i128) {
        *self = self.map(|a| a / rhs);
    }
}

impl Mul<SLPAmount> for i128 {
    type Output = SLPAmount;

    fn mul(self, rhs: SLPAmount) -> Self::Output {
        rhs.map(|a| self * a)
    }
}

impl Div<SLPAmount> for i128 {
    type Output = SLPAmount;

    fn div(self, rhs: SLPAmount) -> Self::Output {
        rhs.map(|a| self / a)
    }
}

impl PartialOrd for SLPAmount {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
       self.base_amount.partial_cmp(&other.base_amount)
    }
}

impl Ord for SLPAmount {
    fn cmp(&self, other: &Self) -> Ordering {
        self.base_amount.cmp(&other.base_amount)
    }
}

impl Sum for SLPAmount {
    fn sum<I: Iterator<Item=SLPAmount>>(mut iter: I) -> Self {
        let mut accumulator = match iter.next() {
            Some(slp_amount) => slp_amount,
            None => {
                eprintln!("warning: summing empty slp amount list");
                return SLPAmount::new(0, 0)
            },
        };
        for val in iter {
            accumulator += val
        }
        accumulator
    }
}

impl Display for SLPAmount {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        if self.decimals == 0 {
            f.pad_integral(self.base_amount >= 0, "", &self.base_amount.to_string())?;
            return Ok(())
        }
        let factor = (10i128).pow(self.decimals);
        let integer_part = self.base_amount / factor;
        let fract_part = self.base_amount % factor;
        f.pad_integral(
            self.base_amount >= 0,
            "",
            &format!("{}.{:0decimals$}", integer_part, fract_part, decimals=self.decimals as usize),
        )?;
        Ok(())
    }
}

impl Into<PgNumeric> for SLPAmount {
    fn into(self) -> PgNumeric {
        i128_to_pg_numeric(self.base_amount)
    }
}
