use std::str::FromStr;

use async_graphql::{OneofObject, SimpleObject, Union};

use super::{
    fontainebleau_grade::FontainebleauGrade, vermin_grade::VerminGrade,
    yosemite_decimal_grade::YosemiteDecimalGrade,
};

#[derive(SimpleObject)]
pub struct Fontainebleau {
    pub value: FontainebleauGrade,
}

#[derive(SimpleObject)]
pub struct YosemiteDecimal {
    pub value: YosemiteDecimalGrade,
}

#[derive(SimpleObject)]
pub struct Vermin {
    pub value: VerminGrade,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseClimbGradeError;

#[derive(Union)]
pub enum Grade {
    Vermin(Vermin),
    Fontainebleau(Fontainebleau),
    YosemiteDecimal(YosemiteDecimal),
}

impl FromStr for Grade {
    type Err = ParseClimbGradeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        VerminGrade::from_str(s)
            .map(|v| Grade::Vermin(Vermin { value: v }))
            .or_else(|_| {
                FontainebleauGrade::from_str(s)
                    .map(|f| Grade::Fontainebleau(Fontainebleau { value: f }))
            })
            .or_else(|_| {
                YosemiteDecimalGrade::from_str(s)
                    .map(|y| Grade::YosemiteDecimal(YosemiteDecimal { value: y }))
            })
            .map_err(|_| ParseClimbGradeError)
    }
}

#[derive(Debug, OneofObject)]
pub enum GradeInput {
    Vermin(VerminGrade),
    Fontainebleau(FontainebleauGrade),
    YosemiteDecimal(YosemiteDecimalGrade),
}
