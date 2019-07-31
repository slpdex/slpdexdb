use diesel::data_types::PgNumeric;
use rug::{Rational, Integer, ops::Pow};
use std::{fmt, fmt::{Formatter, Display}};

pub fn pg_numeric_to_rational(numeric: &PgNumeric) -> Option<Rational> {
    let (is_signed, weight, digits) = match *numeric {
        PgNumeric::Positive { weight, ref digits, .. } => (false, weight, digits),
        PgNumeric::Negative { weight, ref digits, .. } => (true, weight, digits),
        PgNumeric::NaN => { return None },
    };
    let ten = Rational::from(10u8);
    let ten_thousand = Integer::from(10_000u16);
    let mut result = Integer::from(0u8);
    for digit in digits {
        result *= &ten_thousand;
        result += Integer::from(*digit);
    }
    if is_signed {
        result *= Integer::from(-1);
    }
    let count = digits.len() as i32;
    // First digit got factor 10_000^(digits.len() - 1), but should get 10_000^weight
    let correction_exp = 4 * ((weight as i32) - count + 1);
    let factor = ten.pow(-correction_exp);
    let result = Rational::from(result) * factor;
    Some(result)
}

pub fn pg_numeric_to_i128(numeric: &PgNumeric) -> Option<i128> {
    let (is_signed, weight, digits) = match *numeric {
        PgNumeric::Positive {weight, ref digits, ..} => (false, weight, digits),
        PgNumeric::Negative {weight, ref digits, ..} => (true, weight, digits),
        PgNumeric::NaN => { return None },
    };
    let mut result = 0i128;
    for digit in digits {
        result *= 10_000;
        result += *digit as i128;
    }
    if is_signed { result *= -1; }
    let count = digits.len() as i32;
    let correction_exp = 4 * ((weight as i32) - count + 1);
    if correction_exp < 0 { return None; }
    Some(result * (10i128).pow(correction_exp as u32))
}

pub fn i128_to_pg_numeric(mut val: i128) -> PgNumeric {
    let is_positive = val > 0;
    val = val.abs();
    let mut digits = Vec::new();
    while val > 0 {
        let remainder = val % 10_000;
        val = val / 10_000;
        digits.push(remainder as i16);
    }
    digits.reverse();
    let trailing_zeros = digits.iter().rev().take_while(|i| **i == 0).count();
    let weight = digits.len() as i16 - 1;
    let scale = 0;
    digits.resize(digits.len() - trailing_zeros, 0);
    if is_positive {
        PgNumeric::Positive { digits, weight, scale }
    } else {
        PgNumeric::Negative { digits, weight, scale }
    }
}

pub fn rational_to_pg_numeric(rational: Rational, scale: u16) -> PgNumeric {
    let zero = Rational::from(0);
    let ten_thousand = Integer::from(10_000);
    let ten_thousand_r = Rational::from(10_000);
    let (mut fract, mut trunc) = rational.fract_trunc(Integer::new());
    let is_nonnegative = trunc >= zero;
    trunc.abs_mut();
    fract.abs_mut();
    let mut digits = Vec::new();
    let mut digit = Integer::new();
    for _ in 0..scale {
        fract *= &ten_thousand_r;
        fract.fract_trunc_mut(&mut digit);
        let digit = digit.to_i16_wrapping();
        digits.push(digit);
        if fract == zero {
            break
        }
    }
    digits.reverse();
    let mut n_digits_pre = 0;
    while trunc > zero {
        n_digits_pre += 1;
        let mut remainder = ten_thousand.clone();
        trunc.div_rem_mut(&mut remainder);
        let remainder = remainder.to_i16_wrapping();
        digits.push(remainder);
    }
    digits.reverse();
    let preceding_zeros = digits.iter().take_while(|i| **i == 0).count();
    let trailing_zeros = digits.iter().rev().take_while(|i| **i == 0).count();
    let weight = if n_digits_pre == 0 { -1 } else { n_digits_pre - 1 };
    let weight = weight - preceding_zeros as i16;
    let digits = digits[preceding_zeros..digits.len() - trailing_zeros].to_vec();
    if is_nonnegative {
        PgNumeric::Positive { digits, weight, scale }
    } else {
        PgNumeric::Negative { digits, weight, scale }
    }
}

pub struct PrettyRational(pub Rational);

impl Display for PrettyRational {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        fmt_rational(self.0.clone(), f)
    }
}

fn fmt_rational(rational: Rational, f: &mut Formatter) -> Result<(), fmt::Error> {
    let zero = Rational::from(0);
    let ten = Rational::from(10u8);
    let is_nonnegative = rational >= zero;
    let (mut fract, mut trunc) = rational.fract_trunc(Integer::new());
    trunc.abs_mut();
    let mut string = trunc.to_string();
    let max_digits = f.precision().unwrap_or(100);  // rationals can have infinite digits
    if max_digits > 0 && (f.precision().is_some() || fract != 0) {
        string += ".";
        if !f.alternate() {
            for digit_idx in 0..max_digits {
                fract *= &ten;
                if digit_idx == max_digits - 1 {
                    fract.fract_round_mut(&mut trunc);
                } else {
                    fract.fract_trunc_mut(&mut trunc);
                }
                trunc.abs_mut();
                string += &trunc.to_string();
                if f.precision().is_none() && fract == zero {
                    break;
                }
            }
        } else {
            let max_groups = (max_digits + 2) / 3;
            let last_group_size = max_digits % 3;
            for group_idx in 0..max_groups {
                let (is_last_group, group_size) = if group_idx == max_groups - 1 {
                    (true, last_group_size)
                } else {
                    (false, 3)
                };
                for digit_idx in 0..group_size {
                    fract *= &ten;
                    if is_last_group && digit_idx == group_size - 1 {
                        fract.fract_round_mut(&mut trunc);
                    } else {
                        fract.fract_trunc_mut(&mut trunc);
                    }
                    trunc.abs_mut();
                    string += &trunc.to_string();
                }
                if f.precision().is_none() && fract == zero {
                    break;
                }
                if !is_last_group {
                    string += " ";
                }
            }
        }
    }
    f.pad_integral(is_nonnegative, "", &string)?;
    Ok(())
}
