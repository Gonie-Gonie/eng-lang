use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GregorianDateComponent {
    Year,
    Month,
    Day,
}

impl GregorianDateComponent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Year => "year",
            Self::Month => "month",
            Self::Day => "day",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GregorianDateError {
    pub component: GregorianDateComponent,
    pub value: i64,
    pub minimum: i64,
    pub maximum: i64,
}

impl fmt::Display for GregorianDateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {} is outside {}..={}",
            self.component.as_str(),
            self.value,
            self.minimum,
            self.maximum
        )
    }
}

impl Error for GregorianDateError {}

pub fn gregorian_month_length(year: i64, month: i64) -> Option<i64> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if is_gregorian_leap_year(year) => Some(29),
        2 => Some(28),
        _ => None,
    }
}

pub fn format_gregorian_date(
    year: i64,
    month: i64,
    day: i64,
) -> Result<String, GregorianDateError> {
    if !(1..=9999).contains(&year) {
        return Err(GregorianDateError {
            component: GregorianDateComponent::Year,
            value: year,
            minimum: 1,
            maximum: 9999,
        });
    }
    if !(1..=12).contains(&month) {
        return Err(GregorianDateError {
            component: GregorianDateComponent::Month,
            value: month,
            minimum: 1,
            maximum: 12,
        });
    }
    let maximum = gregorian_month_length(year, month).expect("validated Gregorian month");
    if !(1..=maximum).contains(&day) {
        return Err(GregorianDateError {
            component: GregorianDateComponent::Day,
            value: day,
            minimum: 1,
            maximum,
        });
    }
    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

fn is_gregorian_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::{
        format_gregorian_date, gregorian_month_length, GregorianDateComponent, GregorianDateError,
    };

    #[test]
    fn formats_valid_gregorian_dates_and_leap_days() {
        assert_eq!(
            format_gregorian_date(2026, 7, 21).as_deref(),
            Ok("2026-07-21")
        );
        assert_eq!(
            format_gregorian_date(2000, 2, 29).as_deref(),
            Ok("2000-02-29")
        );
        assert_eq!(gregorian_month_length(1900, 2), Some(28));
        assert_eq!(gregorian_month_length(2000, 2), Some(29));
    }

    #[test]
    fn reports_the_invalid_gregorian_component() {
        assert_eq!(
            format_gregorian_date(0, 1, 1),
            Err(GregorianDateError {
                component: GregorianDateComponent::Year,
                value: 0,
                minimum: 1,
                maximum: 9999,
            })
        );
        assert_eq!(
            format_gregorian_date(2026, 2, 29),
            Err(GregorianDateError {
                component: GregorianDateComponent::Day,
                value: 29,
                minimum: 1,
                maximum: 28,
            })
        );
    }
}
