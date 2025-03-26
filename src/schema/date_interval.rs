use std::{error::Error, fmt::Display, ops::Bound, str::FromStr};

use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};
use chrono::{Duration, NaiveDate};
use postgres_types::{accepts, FromSql, ToSql};

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

impl Display for ParseDateIntervalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to parse PostgreSQL DATERANGE")
    }
}

impl Error for ParseDateIntervalError { }


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

impl<'a> FromSql<'a> for DateInterval {
    fn from_sql(ty: &postgres_types::Type, raw: &'a [u8]) -> std::result::Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        match *ty {
            postgres_types::Type::DATE_RANGE => todo!(),
            postgres_types::Type::TEXT => {
                let text = std::str::from_utf8(raw)?;
                let parsed = DateInterval::from_pg_range_text(text)?;
                Ok(parsed)
            },
            _ => Err(format!("Unsupported type: {:?}", ty).into()),
        }
    }

    accepts!(DATE_RANGE, TEXT);
}

impl ToSql for DateInterval {
    fn to_sql(&self, _: &postgres_types::Type, out: &mut bytes::BytesMut) -> Result<postgres_types::IsNull, Box<dyn Error + Sync + Send>>
    where
        Self: Sized {
        let encoded = self.to_pg_range_text();
        out.extend_from_slice(encoded.as_bytes());
        Ok(postgres_types::IsNull::No)
    }

    accepts!(DATE_RANGE);

    fn to_sql_checked(
        &self,
        ty: &postgres_types::Type,
        out: &mut bytes::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<dyn Error + Sync + Send>> {
        if !<Self as ToSql>::accepts(ty) {
            return Err("Unsupported PostgreSQL type".into());
        }
        self.to_sql(ty, out)
    }

    fn encode_format(&self, _ty: &postgres_types::Type) -> postgres_types::Format {
        postgres_types::Format::Text
    }
}

impl DateInterval {
    fn from_pg_range_text(s: &str) -> std::result::Result<Self, ParseDateIntervalError> {
        let trimmed = s.trim();

        if trimmed.len() < 3 || (!trimmed.starts_with(['[', '('])) || (!trimmed.ends_with([']', ')'])) {
            return Err(ParseDateIntervalError);
        }

        let start_closed = trimmed.starts_with('[');
        let end_closed = trimmed.ends_with(']');

        let inner = &trimmed[1..trimmed.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();

        let start_bound = match parts.first() {
            Some(date_str) if !date_str.is_empty() => {
                let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                    .map_err(|_| ParseDateIntervalError)?;
                if start_closed {
                    Bound::Included(date)
                } else {
                    Bound::Excluded(date)
                }
            }
            _ => Bound::Unbounded,
        };

        let end_bound = match parts.get(1).copied() {
            Some(date_str) if !date_str.is_empty() => {
                let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                    .map_err(|_| ParseDateIntervalError)?;
                if end_closed {
                    Bound::Included(date)
                } else {
                    Bound::Excluded(date)
                }
            }
            _ => Bound::Unbounded,
        };

        Ok(DateInterval(start_bound, end_bound))
    }

    fn to_pg_range_text(&self) -> String {
        let start_bound = match &self.0 {
            Bound::Included(date) => format!("[{}", date.format("%Y-%m-%d")),
            Bound::Excluded(date) => format!("({}", date.format("%Y-%m-%d")),
            Bound::Unbounded => "(".to_string(),
        };

        let end_bound = match &self.1 {
            Bound::Included(date) => format!(",{:}]", date.format("%Y-%m-%d")),
            Bound::Excluded(date) => format!(",{:})", date.format("%Y-%m-%d")),
            Bound::Unbounded => ",)".to_string(),
        };

        format!("{}{}", start_bound, end_bound)
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
