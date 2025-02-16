use async_graphql::{Scalar, ScalarType, Value, InputValueError, InputValueResult};
use regex::Regex;

#[derive(Clone, Debug)]
pub struct VerminGrade(pub u8);

#[Scalar]
impl ScalarType for VerminGrade {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(s) = &value {
            let re = Regex::new(r"^(v|V)?([0-9]+)$").unwrap();
            if let Some(captures) = re.captures(s) {
                if let Some(grade_match) = captures.get(2) {
                    if let Ok(grade) = grade_match.as_str().parse::<u8>() {
                        return Ok(VerminGrade(grade));
                    }
                }
            }
        }
        Err(InputValueError::custom("Invalid format"))
    }

    fn to_value(&self) -> Value {
        Value::String(format!("V{}", self.0))
    }
}

