use async_graphql::{Scalar, ScalarType, Value, InputValueError, InputValueResult};
use postgres_types::{FromSql, ToSql};
use regex::Regex;

#[derive(Clone, Debug)]
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

#[Scalar]
impl ScalarType for YosemiteDecimalGrade {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(s) = &value {
            let re = Regex::new(r"^(5\.)?(\d{1,2})([abcd]?)$").unwrap();
            if let Some(captures) = re.captures(s) {
                let grade = captures.get(2).unwrap().as_str().parse::<u8>().unwrap();

                if grade < 1 {
                    return Err(InputValueError::custom("Grade must be at least 5.1"));
                }

                let letter = match captures.get(3).map(|m| m.as_str()) {
                    Some("a") if grade >= 10 => Some(YosemiteDecimalLetter::A),
                    Some("b") if grade >= 10 => Some(YosemiteDecimalLetter::B),
                    Some("c") if grade >= 10 => Some(YosemiteDecimalLetter::C),
                    Some("d") if grade >= 10 => Some(YosemiteDecimalLetter::D),
                    Some("") if grade < 10 => None,
                    _ => return Err(InputValueError::custom("Letters a-d are must, and only, be used for 5.10 and above")),
                };

                return Ok(YosemiteDecimalGrade { grade, letter });
            }
        }
        Err(InputValueError::custom("Invalid format"))
    }

    fn to_value(&self) -> Value {
        let mut grade_str = format!("5.{}", self.grade);
        if let Some(letter) = &self.letter {
            grade_str.push(match letter {
                YosemiteDecimalLetter::A => 'a',
                YosemiteDecimalLetter::B => 'b',
                YosemiteDecimalLetter::C => 'c',
                YosemiteDecimalLetter::D => 'd',
            });
        }
        Value::String(grade_str)
    }
}

