use async_graphql::{Scalar, ScalarType, Value, InputValueError, InputValueResult};
use postgres_types::{FromSql, ToSql};
use regex::Regex;

#[derive(Clone, Debug)]
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

#[Scalar]
impl ScalarType for FontainebleauGrade {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(s) = &value {
            let re = Regex::new(r"^(?:[Ff](?:[Bb])?)?([1-9])([a-cA-C])?(\+)?$").unwrap();
            if let Some(captures) = re.captures(s) {
                let number = captures.get(1).unwrap().as_str().parse::<u8>().unwrap();

                let letter = match captures.get(2).map(|m| m.as_str()) {
                    Some("A") | Some("a") => Some(FontainebleauLetter::A),
                    Some("B") | Some("b") => Some(FontainebleauLetter::B),
                    Some("C") | Some("c") => Some(FontainebleauLetter::C),
                    None if number >= 6 => return Err(InputValueError::custom("Grades 6 and above require A, B, or C")),
                    _ => None,
                };

                let plus = captures.get(3).is_some();

                return Ok(FontainebleauGrade { number, letter, plus });
            }
        }
        Err(InputValueError::custom("Invalid format"))
    }

    fn to_value(&self) -> Value {
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
        Value::String(format!("F{}", grade))
    }
}

