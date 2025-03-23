use std::str::FromStr;

use async_graphql::{Scalar, ScalarType, Value, InputValueError, InputValueResult};
use regex::Regex;

#[derive(Clone, Debug)]
pub struct VerminGrade(pub u8);

#[derive(Debug, PartialEq, Eq)]
pub struct ParseVerminGradeError;

impl FromStr for VerminGrade {
    type Err = ParseVerminGradeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(r"^(v|V)?([0-9]+)$").unwrap();
        if let Some(captures) = re.captures(s) {
            if let Some(grade_match) = captures.get(2) {
                if let Ok(grade) = grade_match.as_str().parse::<u8>() {
                    return Ok(VerminGrade(grade));
                }
            }
        }

        Err(ParseVerminGradeError)
    }
}

#[Scalar]
impl ScalarType for VerminGrade {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(s) = &value {
            s.parse::<VerminGrade>().map_err(|_| InputValueError::custom("Invalid format"))
        } else {
            Err(InputValueError::custom("Expected a string"))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(format!("V{}", self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_good() {
        let values = [
            Value::String("V0".into()),
            Value::String("V255".into()),
            Value::String("v0".into()),
            Value::String("v255".into()),
            Value::String("0".into()),
            Value::String("255".into()),
        ];

        let expected = [
            0,
            255,
            0,
            255,
            0,
            255,
        ];

        for (v, e) in values.iter().zip(expected.iter()) {
            assert_eq!(VerminGrade::parse(v.clone()).unwrap().0, *e);
        }
    }

    #[test]
    fn parse_bad() {
        let values = [
            Value::Number(0.into()), // only string-values accepted
            Value::String("5.10".into()), // doesn't match regex
            Value::String("V256".into()), // cannot be contained in a u8
        ];

        for v in values.iter() {
            // An error is an error, regardless of fields (for now)
            assert!(matches!(VerminGrade::parse(v.clone()), Err(InputValueError { .. })));
        }
    }

    #[test]
    fn to_value() {
        let grades = [VerminGrade(0)];
        let expected = ["V0"];

        for (g, e) in grades.iter().zip(expected.iter()) {
            assert_eq!(g.to_value(), Value::String(e.to_string()));
        }
    }
}
