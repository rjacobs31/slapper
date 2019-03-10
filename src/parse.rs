use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;
use std::str::FromStr;

enum SubstitutingSegment {
    Plain(String),
    Variable(String),
}

pub struct SubstitutingUrl {
    segments: Vec<SubstitutingSegment>,
}

impl SubstitutingUrl {
    fn parse(&mut self, remainder: &str) -> Result<(), ParseError> {
        let special_chars = |x: char| x == '\\' || x == '{';

        if remainder.is_empty() {
            return Ok(());
        }

        if remainder.starts_with(r"\{") {
            let plain_start = r"\".len();
            if let Some(pos) = &remainder[r"\{".len()..].find(special_chars) {
                let plain_end = plain_start + pos;

                let plain = &remainder[plain_start..plain_end];
                let remainder = &remainder[plain_end..];

                self.segments
                    .push(SubstitutingSegment::Plain(plain.to_owned()));
                return self.parse(remainder);
            }

            self.segments.push(SubstitutingSegment::Plain(
                remainder[plain_start..].to_owned(),
            ));
            return Ok(());
        }

        if remainder.starts_with('{') {
            if let Some(pos) = remainder.find('}') {
                let (name_start, name_end) = ("{".len(), pos);
                let rem_start = name_end + "}".len();

                let name = &remainder[name_start..name_end];
                if name.is_empty() {
                    return Err(ParseError::EmptyName);
                }
                let remainder = &remainder[rem_start..];

                self.segments
                    .push(SubstitutingSegment::Variable(name.to_owned()));
                return self.parse(remainder);
            }

            return Err(ParseError::UnterminatedVariableTag);
        }

        if let Some(pos) = remainder.find(special_chars) {
            let (plain, remainder) = remainder.split_at(pos);

            self.segments
                .push(SubstitutingSegment::Plain(plain.to_owned()));
            return self.parse(remainder);
        }

        self.segments
            .push(SubstitutingSegment::Plain(remainder.to_owned()));
        Ok(())
    }

    pub fn sub_by_name(
        &self,
        values: &HashMap<String, String>,
    ) -> Result<Cow<str>, SubstitutionError> {
        let mut result = String::from("");

        let mut position = 0usize;
        for segment in &self.segments {
            match segment {
                SubstitutingSegment::Plain(plain) => {
                    result
                        .write_str(&plain)
                        .expect("unknown error when writing parsed URL");
                }
                SubstitutingSegment::Variable(name) => {
                    if let Some(val) = values.get(name) {
                        result
                            .write_str(&val)
                            .expect("unknown error when writing parsed URL");
                    } else {
                        return Err(SubstitutionError::MissingParameter {
                            name: name.clone(),
                            position,
                        });
                    }
                    position += 1;
                }
            }
        }

        Ok(result.into())
    }

    pub fn sub_by_index<I>(&self, values: I) -> Result<Cow<str>, SubstitutionError>
    where
        I: IntoIterator<Item = String>,
    {
        use std::iter::FromIterator;

        let var_count = self
            .segments
            .iter()
            .filter(|x| match x {
                SubstitutingSegment::Variable(_) => true,
                _ => false,
            })
            .count();
        let values = Vec::from_iter(values);
        let val_count = values.len();
        if val_count < var_count {
            return Err(SubstitutionError::WrongNumberVariables {
                expected: var_count,
                found: val_count,
            });
        }

        let mut result = String::from("");
        let mut values = values.iter().rev();

        let mut position = 0usize;
        for segment in &self.segments {
            match segment {
                SubstitutingSegment::Plain(plain) => {
                    result
                        .write_str(&plain)
                        .expect("unknown error when writing parsed URL");
                }
                SubstitutingSegment::Variable(name) => {
                    if let Some(val) = values.next() {
                        result
                            .write_str(&val)
                            .expect("unknown error when writing parsed URL");
                    } else {
                        return Err(SubstitutionError::MissingParameter {
                            name: name.clone(),
                            position,
                        });
                    }
                    position += 1;
                }
            }
        }

        Ok(result.into())
    }
}

impl FromStr for SubstitutingUrl {
    type Err = ParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut subber = SubstitutingUrl {
            segments: Vec::new(),
        };
        subber.parse(input)?;

        Ok(subber)
    }
}

#[cfg(test)]
mod url_replace_tests {
    use super::*;

    #[test]
    fn test_plain() {
        let subber = SubstitutingUrl::from_str("test/something/blah").unwrap();

        let result = subber.sub_by_name(&HashMap::new()).unwrap();
        assert_eq!(result, "test/something/blah".to_owned());
    }

    #[test]
    fn test_parse() {
        let subber = SubstitutingUrl::from_str("test/{val}/blah").unwrap();

        let mut test_values = HashMap::new();
        test_values.insert("val".to_owned(), "something".to_owned());

        let result = subber.sub_by_name(&test_values).unwrap();
        assert_eq!(result, "test/something/blah".to_owned());
    }

    #[test]
    fn test_multi_parse() {
        let subber = SubstitutingUrl::from_str("{v1}/{v2}{v3}").unwrap();

        let mut test_values = HashMap::new();
        test_values.insert("v1".to_owned(), "la".to_owned());
        test_values.insert("v2".to_owned(), "de".to_owned());
        test_values.insert("v3".to_owned(), "dah".to_owned());

        let result = subber.sub_by_name(&test_values).unwrap();
        assert_eq!(result, "la/dedah".to_owned());
    }
}

#[derive(Debug)]
pub enum ParseError {
    EmptyName,
    UnterminatedVariableTag,
}

#[derive(Debug)]
pub enum SubstitutionError {
    MissingParameter { name: String, position: usize },
    WrongNumberVariables { expected: usize, found: usize },
}
