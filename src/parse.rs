use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;

enum SubstitutingSegment {
    Plain(String),
    Variable(String),
}

pub struct SubstitutingUrl {
    segments: Vec<SubstitutingSegment>,
}

impl SubstitutingUrl {
    pub fn from_str(input: &str) -> Self {
        let mut result = SubstitutingUrl {
            segments: Vec::new(),
        };
        result.parse(input);

        result
    }

    fn parse(&mut self, remainder: &str) {
        if remainder.is_empty() {
            return;
        }

        if remainder.starts_with(r"\{") {
            let plain_start = r"\".len();
            if let Some(pos) = find_interest(&remainder[r"\{".len()..]) {
                let plain_end = plain_start + pos;

                let plain = &remainder[plain_start..plain_end];
                let remainder = &remainder[plain_end..];

                self.segments
                    .push(SubstitutingSegment::Plain(plain.to_owned()));
                self.parse(remainder);
                return; // url_replace_inner(result, to_add, remainder, values);
            }

            self.segments.push(SubstitutingSegment::Plain(
                remainder[plain_start..].to_owned(),
            ));
            return;
        }

        if remainder.starts_with("{") {
            if let Some(pos) = remainder.find("}") {
                let (name_start, name_end) = ("{".len(), pos);
                let rem_start = name_end + "}".len();

                let name = &remainder[name_start..name_end];
                let remainder = &remainder[rem_start..];

                self.segments
                    .push(SubstitutingSegment::Variable(name.to_owned()));
                self.parse(remainder);
                return; // url_replace_inner(result, val, remainder, values);
            }

            return; // Err("Unterminated variable tag");
        }

        if let Some(pos) = find_interest(remainder) {
            let (plain, remainder) = remainder.split_at(pos);

            self.segments
                .push(SubstitutingSegment::Plain(plain.to_owned()));
            self.parse(remainder);
            return;
        }

        self.segments
            .push(SubstitutingSegment::Plain(remainder.to_owned()));
        return;
    }

    pub fn sub_by_name(&self, values: &HashMap<String, String>) -> Cow<str> {
        let mut result = String::from("");

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
                            .expect(&format!("no value for variable: {}", name));
                    }
                }
            }
        }

        result.into()
    }

    pub fn sub_by_index(&self, values: &[String]) -> Cow<str> {
        let mut result = String::from("");
        let mut values = values.iter().rev();

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
                            .expect(&format!("not enough values for variable: {}", name));
                    }
                }
            }
        }

        result.into()
    }
}

fn find_interest(input: &str) -> Option<usize> {
    input.find(|x| x == '\\' || x == '{')
}

#[cfg(test)]
mod url_replace_tests {
    use super::*;

    #[test]
    fn test_plain() {
        let subber = SubstitutingUrl::from_str("test/something/blah");

        let result = subber.sub_by_name(&HashMap::new());
        assert_eq!(result, "test/something/blah".to_owned());
    }

    #[test]
    fn test_parse() {
        let subber = SubstitutingUrl::from_str("test/{val}/blah");

        let mut test_values = HashMap::new();
        test_values.insert("val".to_owned(), "something".to_owned());

        let result = subber.sub_by_name(&test_values);
        assert_eq!(result, "test/something/blah".to_owned());
    }

    #[test]
    fn test_multi_parse() {
        let subber = SubstitutingUrl::from_str("{v1}/{v2}{v3}");

        let mut test_values = HashMap::new();
        test_values.insert("v1".to_owned(), "la".to_owned());
        test_values.insert("v2".to_owned(), "de".to_owned());
        test_values.insert("v3".to_owned(), "dah".to_owned());

        let result = subber.sub_by_name(&test_values);
        assert_eq!(result, "la/dedah".to_owned());
    }
}
