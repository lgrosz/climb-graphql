use std::{fmt::Display, ops::Bound, str::FromStr};

use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};
use chrono::{Duration, NaiveDate};

#[derive(Debug, PartialEq)]
pub struct DateInterval(pub Bound<NaiveDate>, pub Bound<NaiveDate>);

impl Display for DateInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO This implementation is naive, I can think of several simplifications off the top of
        // my head such as..
        // - [2024-03-01, 2024-03-02) --> 2024-03-02
        // - [2024-03-01, 2024-04-01) --> 2024-03
        // - [2024-01-01, 2025-01-01) --> 2024

        let start = match &self.0 {
            Bound::Included(date) => date.to_string(),
            Bound::Excluded(date) => (*date + Duration::days(1)).to_string(),
            Bound::Unbounded => String::from(".."),
        };

        let end = match &self.1 {
            Bound::Included(date) => date.to_string(),
            Bound::Excluded(date) => (*date - Duration::days(1)).to_string(),
            Bound::Unbounded => String::from(".."),
        };

        write!(f, "{}/{}", start, end)
    }
}

#[derive(Debug)]
pub struct ParseDateIntervalError;

impl FromStr for DateInterval {
    type Err = ParseDateIntervalError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(ParseDateIntervalError);
        }

        let parse_bound = |s: &str| -> Result<Bound<NaiveDate>, ParseDateIntervalError> {
            if s == ".." {
                Ok(Bound::Unbounded)
            } else {
                NaiveDate::from_str(s)
                    .map(Bound::Included)
                    .map_err(|_| ParseDateIntervalError)
            }
        };

        let start = parse_bound(parts[0])?;
        let end = parse_bound(parts[1])?;

        Ok(DateInterval(start, end))
    }
}

#[Scalar]
impl ScalarType for DateInterval {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(s) = &value {
            s.parse::<Self>().map_err(|_| InputValueError::custom("Invalid format"))
        } else {
            Err(InputValueError::custom("Expected a string"))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt() {
        assert_eq!(
            format!("{}", DateInterval(Bound::Unbounded, Bound::Unbounded)),
            "../.."
        );

        assert_eq!(
            format!(
                "{}",
                DateInterval(
                    Bound::Included(NaiveDate::from_ymd_opt(2024, 3, 1).unwrap()),
                    Bound::Unbounded
                )
            ),
            "2024-03-01/.."
        );

        assert_eq!(
            format!(
                "{}",
                DateInterval(
                    Bound::Unbounded,
                    Bound::Included(NaiveDate::from_ymd_opt(2024, 3, 31).unwrap())
                )
            ),
            "../2024-03-31"
        );

        assert_eq!(
            format!(
                "{}",
                DateInterval(
                    Bound::Included(NaiveDate::from_ymd_opt(2024, 3, 1).unwrap()),
                    Bound::Included(NaiveDate::from_ymd_opt(2024, 3, 31).unwrap())
                )
            ),
            "2024-03-01/2024-03-31"
        );
    }

    #[test]
    fn from_str() {
        assert_eq!(
            DateInterval(Bound::Unbounded, Bound::Unbounded),
            "../..".to_string().parse::<DateInterval>().expect("Is valid")
        );

        assert_eq!(
            DateInterval(
                Bound::Included(NaiveDate::from_ymd_opt(2024, 3, 1).unwrap()),
                Bound::Unbounded
            ),
            "2024-03-01/..".to_string().parse::<DateInterval>().expect("Is valid")
        );

        assert_eq!(
            DateInterval(
                Bound::Unbounded,
                Bound::Included(NaiveDate::from_ymd_opt(2024, 3, 31).unwrap())
            ),
            "../2024-03-31".to_string().parse::<DateInterval>().expect("Is valid")
        );

        assert_eq!(
            DateInterval(
                Bound::Included(NaiveDate::from_ymd_opt(2024, 3, 1).unwrap()),
                Bound::Included(NaiveDate::from_ymd_opt(2024, 3, 31).unwrap())
            ),
            "2024-03-01/2024-03-31".to_string().parse::<DateInterval>().expect("Is valid")
        );
    }
}
