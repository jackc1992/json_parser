use anyhow::Result;
use anyhow::anyhow;
use std::{collections::HashMap, str::Chars};

#[derive(Debug, PartialEq)]
enum Json {
    String(String),
    Number(f64),
    // not sure if this is real
    Integer(usize),
    Array(Vec<Json>),
    Object(HashMap<String, Json>),
    Boolean(bool),
    Null,
}

struct Parser<'a> {
    current_char: Option<char>,
    iterator: Chars<'a>,
    buffer: String,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let buffer = String::new();
        let current_char = None;
        let iterator = input.chars();

        Self {
            buffer,
            current_char,
            iterator,
        }
    }

    fn advance(&mut self) {
        self.current_char = self.iterator.next();
    }

    fn eat_whitespace(&mut self) {
        while let Some(c) = self.current_char {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn parse_expected_value(&mut self, expected: &str, json_type: Json) -> Result<Json> {
        if self.iterator.as_str().starts_with(expected) {
            for _ in 0..expected.len() {
                self.advance();
            }
            Ok(json_type)
        } else {
            Err(anyhow!(
                "invalid json value, probably wanted: {}{}",
                self.current_char.unwrap(),
                expected
            ))
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        self.advance();
        self.buffer.clear();

        while let Some(c) = self.current_char {
            if c == '"' {
                return Ok(self.buffer.clone());
            }
            self.buffer.push(c);
            self.advance();
        }

        Err(anyhow!("Json string never ends!"))
    }

    fn parse_object(&mut self) -> Result<Json> {
        let mut result = HashMap::new();
        loop {
            self.advance();
            self.eat_whitespace();

            match self.current_char {
                Some('"') => {}
                Some(_) => return Err(anyhow!("Invalid json object composition, no separator")),
                None => return Err(anyhow!("Invalid Json object composition")),
            }

            let key = self.parse_string()?;

            self.advance();
            self.eat_whitespace();

            match self.current_char {
                Some(':') => {}
                Some(_) => return Err(anyhow!("Invalid json object composition, no separator")),
                None => return Err(anyhow!("Invalid Json object composition")),
            }

            let value = self.parse_value()?;

            result.insert(key, value);

            self.advance();
            self.eat_whitespace();

            match self.current_char {
                Some('}') => break,
                Some(',') => continue,
                Some(c) => return Err(anyhow!("Invalid Json object composition, got: {}", c)),
                None => return Err(anyhow!("Invalid Json object composition, no closing }}")),
            }
        }

        Ok(Json::Object(result))
    }

    fn parse_array(&mut self) -> Result<Json> {
        let mut result = Vec::new();

        loop {
            let value = self.parse_value()?;
            result.push(value);

            self.advance();
            self.eat_whitespace();

            match self.current_char {
                Some(']') => break,
                Some(',') => continue,
                Some(c) => return Err(anyhow!("Invalid json array structure, got: {c}")),
                None => return Err(anyhow!("Invalid json array structure, no closing ]")),
            }
        }

        Ok(Json::Array(result))
    }

    fn parse_digits(&mut self) -> Result<Json> {
        let iter_clone = self.iterator.clone();
        let mut seen_dot = false;
        self.buffer.clear();

        self.buffer.push(self.current_char.unwrap());

        for c in iter_clone {
            if c == '.' && !seen_dot {
                seen_dot = true;
            } else if !c.is_ascii_digit() {
                break;
            }
            self.buffer.push(c);
            self.advance();
        }

        if seen_dot {
            let value = self.buffer.parse()?;
            // incredibly scuffed
            if let Some(c) = self.current_char {
                if c == '.' {
                    return Err(anyhow!("invalid json number structure"));
                }
            }
            Ok(Json::Number(value))
        } else {
            let value = self.buffer.parse()?;
            Ok(Json::Integer(value))
        }
    }

    fn parse_value(&mut self) -> Result<Json> {
        self.advance();
        self.eat_whitespace();

        match self.current_char {
            Some('[') => self.parse_array(),
            Some('{') => self.parse_object(),
            Some('"') => {
                let value = self.parse_string()?;
                Ok(Json::String(value))
            }
            Some('n') => self.parse_expected_value("ull", Json::Null),
            Some('f') => self.parse_expected_value("alse", Json::Boolean(false)),
            Some('t') => self.parse_expected_value("rue", Json::Boolean(true)),
            Some(c) if c.is_ascii_digit() => self.parse_digits(),
            Some(c) => Err(anyhow!("Unexpected: {} in json value", c)),
            None => Err(anyhow!("Invalid json format.")),
        }
    }

    pub fn parse(&mut self) -> Result<Json> {
        let result = self.parse_value()?;
        self.advance();
        self.eat_whitespace();
        match self.current_char {
            Some(_) => Err(anyhow!("Invalid json format.")),
            None => Ok(result),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_everything() {
        let json_value = r#"[123.1231,"abc", {"nested": {"object": 1}}, null, false, true, "weirldly huge amount of space    ", true]"#;
        let mut parser = Parser::new(json_value);
        let value = parser.parse().unwrap();

        let mut arr = Vec::new();

        let mut inner_object = HashMap::new();
        inner_object.insert(String::from("object"), Json::Integer(1));
        let inner_object = Json::Object(inner_object);

        let mut outer_object = HashMap::new();
        outer_object.insert(String::from("nested"), inner_object);
        let outer_object = Json::Object(outer_object);

        arr.push(Json::Number(123.1231));
        arr.push(Json::String("abc".into()));
        arr.push(outer_object);
        arr.push(Json::Null);
        arr.push(Json::Boolean(false));
        arr.push(Json::Boolean(true));
        arr.push(Json::String("weirldly huge amount of space    ".into()));
        arr.push(Json::Boolean(true));

        assert_eq!(value, Json::Array(arr));
    }

    #[test]
    fn test_everything_with_weird_spacing() {
        let json_value = r#"[123.1231,"abc", {"nested": {"object": 1}}, null, false, true      ,           "weirldly huge amount of space    ", true  ]"#;
        let mut parser = Parser::new(json_value);
        let value = parser.parse().unwrap();

        let mut arr = Vec::new();

        let mut inner_object = HashMap::new();
        inner_object.insert(String::from("object"), Json::Integer(1));
        let inner_object = Json::Object(inner_object);

        let mut outer_object = HashMap::new();
        outer_object.insert(String::from("nested"), inner_object);
        let outer_object = Json::Object(outer_object);

        arr.push(Json::Number(123.1231));
        arr.push(Json::String("abc".into()));
        arr.push(outer_object);
        arr.push(Json::Null);
        arr.push(Json::Boolean(false));
        arr.push(Json::Boolean(true));
        arr.push(Json::String("weirldly huge amount of space    ".into()));
        arr.push(Json::Boolean(true));

        assert_eq!(value, Json::Array(arr));
    }

    #[test]
    fn invalid_object() {
        let json_value = r#"{"#;
        let mut parser = Parser::new(json_value);
        let value = parser.parse();
        assert!(value.is_err());
    }

    #[test]
    fn verify_that_top_level_fails_if_extra_stuff_is_there() {
        let json_value = r#"1234 aihykuajnlsd"#;
        let mut parser = Parser::new(json_value);
        let value = parser.parse();
        assert!(value.is_err());
    }
}

fn main() {
    let json_value = r#"[123.1231,"abc", {"nested": {"object": 1}}, null, false, true      ,           "weirldly huge amount of space    ", true  ]"#;
    let mut parser = Parser::new(json_value);
    let value = parser.parse();

    println!("{:?}", value);
}
