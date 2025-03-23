use std::{fmt::Display, str::FromStr};

use async_graphql::{Scalar, ScalarType, Value, InputValueError, InputValueResult};
use postgres_types::{FromSql, ToSql};
use regex::Regex;

#[derive(Clone, Debug, PartialEq)]
pub struct YosemiteDecimalGrade {
    pub grade: u8,
    pub letter: Option<YosemiteDecimalLetter>,
}

#[derive(Clone, Copy, PartialEq, Eq, FromSql, ToSql, Debug)]
#[postgres(name = "yds_letter")]
pub enum YosemiteDecimalLetter {
    #[postgres(name = "a")]
    A,
    #[postgres(name = "b")]
    B,
    #[postgres(name = "c")]
    C,
    #[postgres(name = "d")]
    D,
}

impl Display for YosemiteDecimalGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut grade_str = format!("5.{}", self.grade);

        if let Some(letter) = &self.letter {
            grade_str.push(match letter {
                YosemiteDecimalLetter::A => 'a',
                YosemiteDecimalLetter::B => 'b',
                YosemiteDecimalLetter::C => 'c',
                YosemiteDecimalLetter::D => 'd',
            });
        }

        write!(f, "{}", grade_str)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseYosemiteDecimalGradeError;

impl FromStr for YosemiteDecimalGrade {
    type Err = ParseYosemiteDecimalGradeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(r"^(5\.)?(\d{1,2})([abcd]?)$").unwrap();
        if let Some(captures) = re.captures(s) {
            let grade = captures.get(2).unwrap().as_str().parse::<u8>().unwrap();

            if grade < 1 {
                return Err(ParseYosemiteDecimalGradeError);
            }

            let letter = match captures.get(3).map(|m| m.as_str()) {
                Some("a") if grade >= 10 => Some(YosemiteDecimalLetter::A),
                Some("b") if grade >= 10 => Some(YosemiteDecimalLetter::B),
                Some("c") if grade >= 10 => Some(YosemiteDecimalLetter::C),
                Some("d") if grade >= 10 => Some(YosemiteDecimalLetter::D),
                Some("") if grade < 10 => None,
                _ => return Err(ParseYosemiteDecimalGradeError),
            };

            return Ok(YosemiteDecimalGrade { grade, letter });
        }

        Err(ParseYosemiteDecimalGradeError)
    }
}

#[Scalar]
impl ScalarType for YosemiteDecimalGrade {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(s) = &value {
            s.parse::<YosemiteDecimalGrade>().map_err(|_| InputValueError::custom("Invalid format"))
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
    fn parse_good() {
        let values = [
            Value::String("5.9".into()),
            Value::String("5.10a".into()),
            Value::String("5.10b".into()),
            Value::String("5.10c".into()),
            Value::String("5.10d".into()),
            Value::String("6".into()),
            Value::String("11b".into()),
        ];

        let expected = [
            YosemiteDecimalGrade {
                grade: 9,
                letter: None,
            },
            YosemiteDecimalGrade {
                grade: 10,
                letter: Some(YosemiteDecimalLetter::A),
            },
            YosemiteDecimalGrade {
                grade: 10,
                letter: Some(YosemiteDecimalLetter::B),
            },
            YosemiteDecimalGrade {
                grade: 10,
                letter: Some(YosemiteDecimalLetter::C),
            },
            YosemiteDecimalGrade {
                grade: 10,
                letter: Some(YosemiteDecimalLetter::D),
            },
            YosemiteDecimalGrade {
                grade: 6,
                letter: None,
            },
            YosemiteDecimalGrade {
                grade: 11,
                letter: Some(YosemiteDecimalLetter::B),
            },
        ];

        for (v, e) in values.iter().zip(expected.iter()) {
            assert_eq!(YosemiteDecimalGrade::parse(v.clone()).unwrap(), *e);
        }
    }

    #[test]
    fn parse_bad() {
        let values = [
            Value::Number(0.into()), // only string-values accepted
            Value::String("f8a".into()), // invalid format
            Value::String("5.0".into()), // grades start a 1
            Value::String("5.9a".into()), // grades 9 and below must not have a letter
            Value::String("5.10".into()), // grades 10 and above must have a letter
        ];

        for v in values.iter() {
            // An error is an error, regardless of fields (for now)
            assert!(
                matches!(YosemiteDecimalGrade::parse(v.clone()), Err(InputValueError { .. })),
                "{} expected to fail parsing but didn't", v
                );
        }
    }

    #[test]
    fn to_value() {
        let grades = [
            YosemiteDecimalGrade { grade: 9, letter: None },
            YosemiteDecimalGrade { grade: 10, letter: Some(YosemiteDecimalLetter::A) },
            YosemiteDecimalGrade { grade: 10, letter: Some(YosemiteDecimalLetter::B) },
            YosemiteDecimalGrade { grade: 10, letter: Some(YosemiteDecimalLetter::C) },
            YosemiteDecimalGrade { grade: 10, letter: Some(YosemiteDecimalLetter::D) },
        ];

        let expected = [
            "5.9",
            "5.10a",
            "5.10b",
            "5.10c",
            "5.10d",
        ];

        for (g, e) in grades.iter().zip(expected.iter()) {
            assert_eq!(g.to_value(), Value::String(e.to_string()));
        }
    }
}
