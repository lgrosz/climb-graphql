use std::{fmt::Display, ops::Bound, str::FromStr};

use async_graphql::{InputValueResult, Scalar, ScalarType, Value};
use chrono::NaiveDate;

pub enum DateRangeError {
    InvalidBounds,
}

#[derive(Debug)]
pub enum DateRangeParseError {
    BadFormat,
    BadBounds,
}

#[derive(Debug)]
#[derive(PartialEq)]
pub struct DateRange {
    start: Bound<NaiveDate>,
    end: Bound<NaiveDate>,
}

impl DateRange {
    pub fn new(start: Bound<NaiveDate>, end: Bound<NaiveDate>) -> Result<Self, DateRangeError> {
        if Self::validate(&start, &end) {
            Ok(DateRange { start, end })
        } else {
            Err(DateRangeError::InvalidBounds)
        }
    }

    fn validate(start: &Bound<NaiveDate>, end: &Bound<NaiveDate>) -> bool {
        use Bound::*;
        match (start, end) {
            (Included(s), Included(e)) => s <= e,
            (Included(s), Excluded(e)) => s < e,
            (Excluded(s), Included(e)) => s < e,
            (Excluded(s), Excluded(e)) => s < e,
            (_, Unbounded) => true,
            (Unbounded, _) => true,
        }
    }
}

impl Display for DateRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Bound::*;

        let start = match &self.start {
            Included(d) => format!("[{}", d),
            Excluded(d) => format!("({}", d),
            Unbounded => "(".to_string(),
        };

        let end = match &self.end {
            Included(d) => format!(",{}]", d),
            Excluded(d) => format!(",{})", d),
            Unbounded => ",)".to_string(),
        };

        write!(f, "{}{}", start, end)
    }
}

impl FromStr for DateRange {
    type Err = DateRangeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if s.len() < 2 {
            return Err(DateRangeParseError::BadFormat);
        }

        let start_bound_char = &s[0..1];
        let end_bound_char = &s[s.len() - 1..];

        let inner = &s[1..s.len() - 1]; // strip brackets
        let parts: Vec<&str> = inner.split(',').collect();

        if parts.len() != 2 {
            return Err(DateRangeParseError::BadFormat);
        }

        let parse_bound = |s: &str, bound_type: BoundType| -> Result<Bound<NaiveDate>, DateRangeParseError> {
            let s = s.trim();
            if s.is_empty() {
                return Ok(Bound::Unbounded);
            }

            if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                return Ok(match bound_type {
                    BoundType::StartIncluded => Bound::Included(d),
                    BoundType::StartExcluded => Bound::Excluded(d),
                    BoundType::EndIncluded => Bound::Included(d),
                    BoundType::EndExcluded => Bound::Excluded(d),
                });
            }

            Err(DateRangeParseError::BadFormat)
        };

        enum BoundType { StartIncluded, StartExcluded, EndIncluded, EndExcluded }

        let start_bound = match start_bound_char {
            "[" => BoundType::StartIncluded,
            "(" => BoundType::StartExcluded,
            _ => return Err(DateRangeParseError::BadFormat),
        };

        let end_bound = match end_bound_char {
            "]" => BoundType::EndIncluded,
            ")" => BoundType::EndExcluded,
            _ => return Err(DateRangeParseError::BadFormat),
        };

        let start = parse_bound(parts[0], start_bound)?;
        let end = parse_bound(parts[1], end_bound)?;

        let range = DateRange::new(start, end)
            .map_err(|_| DateRangeParseError::BadBounds)?;

        Ok(range)
    }
}

#[Scalar]
impl ScalarType for DateRange {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => s
                .parse::<DateRange>()
                .map_err(|_| "Invalid DateRange format".to_string().into()),

            v => Err(format!("Expected string for DateRange, got {v:?}").into()),
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
    fn validate() {
        use Bound::*;

        let d1 = NaiveDate::from_ymd_opt(2025, 10, 1).expect("invalid or out-of-range date");
        let d2 = NaiveDate::from_ymd_opt(2025, 10, 10).expect("invalid or out-of-range date");

        // Included / Included
        assert!(DateRange::validate(&Included(d1), &Included(d2)));
        assert!(DateRange::validate(&Included(d1), &Included(d1)));
        assert!(!DateRange::validate(&Included(d2), &Included(d1)));

        // Included / Excluded
        assert!(DateRange::validate(&Included(d1), &Excluded(d2)));
        assert!(!DateRange::validate(&Included(d2), &Excluded(d2)));

        // Excluded / Included
        assert!(DateRange::validate(&Excluded(d1), &Included(d2)));
        assert!(!DateRange::validate(&Excluded(d2), &Included(d2)));

        // Excluded / Excluded
        assert!(DateRange::validate(&Excluded(d1), &Excluded(d2)));
        assert!(!DateRange::validate(&Excluded(d2), &Excluded(d2)));

        // Any / Unbounded
        assert!(DateRange::validate(&Included(d1), &Unbounded));
        assert!(DateRange::validate(&Excluded(d1), &Unbounded));
        assert!(DateRange::validate(&Unbounded, &Unbounded));

        // Unbounded / Any
        assert!(DateRange::validate(&Unbounded, &Included(d1)));
        assert!(DateRange::validate(&Unbounded, &Excluded(d1)));
    }

    #[test]
    fn display() {
        struct TestCase {
            range: DateRange,
            expected: &'static str,
        }

        let d = |y, m, d| NaiveDate::from_ymd_opt(y, m, d).unwrap();

        let test_cases = [
            TestCase {
                range: DateRange {
                    start: Bound::Included(d(2025, 10, 8)),
                    end: Bound::Included(d(2025, 10, 10)),
                },
                expected: "[2025-10-08,2025-10-10]",
            },
            TestCase {
                range: DateRange {
                    start: Bound::Excluded(d(2025, 10, 8)),
                    end: Bound::Included(d(2025, 10, 10)),
                },
                expected: "(2025-10-08,2025-10-10]",
            },
            TestCase {
                range: DateRange {
                    start: Bound::Unbounded,
                    end: Bound::Included(d(2025, 10, 10)),
                },
                expected: "(,2025-10-10]",
            },
            TestCase {
                range: DateRange {
                    start: Bound::Included(d(2025, 10, 8)),
                    end: Bound::Unbounded,
                },
                expected: "[2025-10-08,)",
            },
            TestCase {
                range: DateRange {
                    start: Bound::Unbounded,
                    end: Bound::Unbounded,
                },
                expected: "(,)",
            },
            TestCase {
                range: DateRange {
                    start: Bound::Included(d(2025, 10, 8)),
                    end: Bound::Excluded(d(2025, 10, 10)),
                },
                expected: "[2025-10-08,2025-10-10)",
            },
            ];

        for case in test_cases {
            let output = case.range.to_string();
            assert_eq!(
                output, case.expected,
                "Display output mismatch for {:?}",
                case.range
            );
        }
    }

    #[test]
    fn from_str() {
        #[derive(Debug)]
        enum Expect {
            Ok { start: Bound<NaiveDate>, end: Bound<NaiveDate> },
            Err,
        }

        struct TestCase<'a> {
            input: &'a str,
            expect: Expect,
        }

        let test_cases = [
            TestCase {
                input: "[2025-10-08,2025-10-10]",
                expect: Expect::Ok {
                    start: Bound::Included(NaiveDate::from_ymd_opt(2025, 10, 8).unwrap()),
                    end: Bound::Included(NaiveDate::from_ymd_opt(2025, 10, 10).unwrap()),
                },
            },
            TestCase {
                input: "(2025-10-08,2025-10-10]",
                expect: Expect::Ok {
                    start: Bound::Excluded(NaiveDate::from_ymd_opt(2025, 10, 8).unwrap()),
                    end: Bound::Included(NaiveDate::from_ymd_opt(2025, 10, 10).unwrap()),
                },
            },
            TestCase {
                input: "(,2025-10-10]",
                expect: Expect::Ok {
                    start: Bound::Unbounded,
                    end: Bound::Included(NaiveDate::from_ymd_opt(2025, 10, 10).unwrap()),
                },
            },
            TestCase {
                input: "[2025-10-08,)",
                expect: Expect::Ok {
                    start: Bound::Included(NaiveDate::from_ymd_opt(2025, 10, 8).unwrap()),
                    end: Bound::Unbounded,
                },
            },
            TestCase {
                input: "(,)",
                expect: Expect::Ok {
                    start: Bound::Unbounded,
                    end: Bound::Unbounded,
                },
            },
            TestCase {
                input: "[2025-10-08T12:34:56Z,2025-10-10T00:00:00Z)",
                expect: Expect::Err,
            },
            TestCase {
                input: "[2025-10-08;2025-10-10]",
                expect: Expect::Err,
            },
            TestCase {
                input: "[2025-13-08,2025-10-10]",
                expect: Expect::Err,
            },
        ];

        for case in test_cases {
            let result = case.input.parse::<DateRange>();
            match (&result, &case.expect) {
                (Ok(range), Expect::Ok { start, end }) => {
                    assert_eq!(
                        range.start, *start,
                        "Start mismatch for '{}'", case.input
                    );
                    assert_eq!(
                        range.end, *end,
                        "End mismatch for '{}'", case.input
                    );
                }
                (Err(_), Expect::Err) => {}
                (Ok(_), Expect::Err) => panic!("Expected error but got Ok for '{}'", case.input),
                (Err(_), Expect::Ok { .. }) => panic!("Unexpected error for '{}'", case.input),
            }
        }
    }
}
