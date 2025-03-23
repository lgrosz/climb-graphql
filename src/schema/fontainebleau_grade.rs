use std::{fmt::Display, str::FromStr};

use async_graphql::{Scalar, ScalarType, Value, InputValueError, InputValueResult};
use postgres_types::{FromSql, ToSql};
use regex::Regex;

#[derive(Clone, Debug, PartialEq)]
pub struct FontainebleauGrade {
    pub number: u8,
    pub letter: Option<FontainebleauLetter>,
    pub plus: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, FromSql, ToSql, Debug)]
#[postgres(name = "font_letter")]
pub enum FontainebleauLetter {
    #[postgres(name = "A")]
    A,
    #[postgres(name = "B")]
    B,
    #[postgres(name = "C")]
    C,
}

impl Display for FontainebleauGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut grade = format!("{}", self.number);

        if let Some(letter) = &self.letter {
            grade.push(match letter {
                FontainebleauLetter::A => 'A',
                FontainebleauLetter::B => 'B',
                FontainebleauLetter::C => 'C',
            });
        }

        if self.plus {
            grade.push('+');
        }

        write!(f, "F{}", grade)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseFontainebleauGradeError;

impl FromStr for FontainebleauGrade {
    type Err = ParseFontainebleauGradeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(r"^(?:[Ff](?:[Bb])?)?([1-9])([a-cA-C])?(\+)?$").unwrap();
        if let Some(captures) = re.captures(s) {
            let number = captures.get(1).unwrap().as_str().parse::<u8>().unwrap();

            let letter = match captures.get(2).map(|m| m.as_str()) {
                Some(_) if number < 6 => return Err(ParseFontainebleauGradeError),
                Some("A") | Some("a") => Some(FontainebleauLetter::A),
                Some("B") | Some("b") => Some(FontainebleauLetter::B),
                Some("C") | Some("c") => Some(FontainebleauLetter::C),
                None if number >= 6 => return Err(ParseFontainebleauGradeError),
                _ => None,
            };

            let plus = captures.get(3).is_some();

            return Ok(FontainebleauGrade { number, letter, plus });
        }

        Err(ParseFontainebleauGradeError)
    }
}

#[Scalar]
impl ScalarType for FontainebleauGrade {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(s) = &value {
            s.parse::<FontainebleauGrade>().map_err(|_| InputValueError::custom("Invalid format"))
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
            Value::String("F5".into()),
            Value::String("F5+".into()),
            Value::String("F6A".into()),
            Value::String("F6B".into()),
            Value::String("F6C".into()),
            Value::String("F7A+".into()),
        ];

        let expected = [
            FontainebleauGrade {
                number: 5,
                letter: None,
                plus: false,
            },
            FontainebleauGrade {
                number: 5,
                letter: None,
                plus: true,
            },
            FontainebleauGrade {
                number: 6,
                letter: Some(FontainebleauLetter::A),
                plus: false,
            },
            FontainebleauGrade {
                number: 6,
                letter: Some(FontainebleauLetter::B),
                plus: false,
            },
            FontainebleauGrade {
                number: 6,
                letter: Some(FontainebleauLetter::C),
                plus: false,
            },
            FontainebleauGrade {
                number: 7,
                letter: Some(FontainebleauLetter::A),
                plus: true,
            },
        ];

        for (v, e) in values.iter().zip(expected.iter()) {
            assert_eq!(FontainebleauGrade::parse(v.clone()).unwrap(), *e);
        }
    }

    #[test]
    fn parse_bad() {
        let values = [
            Value::Number(0.into()), // only string-values accepted
            Value::String("5.10".into()), // doesn't match regex
            Value::String("F5A".into()), // modifiers only valid for grades >5
            Value::String("F6".into()), // modifiers required for grades >5
        ];

        for v in values.iter() {
            // An error is an error, regardless of fields (for now)
            assert!(
                matches!(FontainebleauGrade::parse(v.clone()), Err(InputValueError { .. })),
                "{} expected to fail parsing but didn't", v
                );
        }
    }

    #[test]
    fn to_value() {
        let grades = [
            FontainebleauGrade { number: 5, letter: None, plus: false },
            FontainebleauGrade { number: 5, letter: None, plus: true },
            FontainebleauGrade { number: 6, letter: Some(FontainebleauLetter::A), plus: false },
            FontainebleauGrade { number: 6, letter: Some(FontainebleauLetter::B), plus: false },
            FontainebleauGrade { number: 6, letter: Some(FontainebleauLetter::C), plus: false },
        ];

        let expected = [
            "F5",
            "F5+",
            "F6A",
            "F6B",
            "F6C",
        ];

        for (g, e) in grades.iter().zip(expected.iter()) {
            assert_eq!(g.to_value(), Value::String(e.to_string()));
        }
    }
}
