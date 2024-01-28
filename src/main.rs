use serde_json;
use std::{env, iter::Peekable, str::Chars};

// Available if you need it!
// use serde_bencode

enum BencodeValue {
    ByteString(String),
    Integer(i64),
    List(Vec<BencodeValue>),
}

impl BencodeValue {
    fn from_bencoded_string(chars: &mut Peekable<Chars>) -> Option<Self> {
        let mut index = String::new();
        while let Some(cur) = chars.next() {
            if cur != ':' {
                index.push(cur);
            } else {
                break;
            }
        }
        let index = index.parse().unwrap();
        let string: String = chars.take(index).collect();

        Some(BencodeValue::ByteString(string))
    }

    fn from_bencoded_integer(chars: &mut Peekable<Chars>) -> Option<Self> {
        chars.next();
        let mut number = String::new();
        while let Some(cur) = chars.next() {
            if cur != 'e' {
                number.push(cur);
            } else {
                break;
            }
        }
        Some(BencodeValue::Integer(number.parse().unwrap()))
    }

    fn from_bencoded_list(chars: &mut Peekable<Chars>) -> Option<Self> {
        chars.next();
        let mut values = Vec::new();
        while let Some(cur) = chars.peek() {
            if *cur != 'e' {
                values.push(decode_bencoded_value(chars).unwrap());
            } else {
                break;
            }
        }

        Some(BencodeValue::List(values))
    }
}

impl Into<serde_json::Value> for BencodeValue {
    fn into(self) -> serde_json::Value {
        match self {
            BencodeValue::ByteString(string) => serde_json::Value::String(string),
            BencodeValue::Integer(number) => serde_json::Value::Number(number.into()),
            BencodeValue::List(list) => {
                let mut vec: Vec<serde_json::Value> = Vec::new();
                for val in list {
                    vec.push(val.into());
                }
                serde_json::Value::Array(vec)
            }
        }
    }
}

#[allow(dead_code)]
fn decode_bencoded_value(chars: &mut Peekable<Chars>) -> Option<BencodeValue> {
    // If encoded_value starts with a digit, it's a number
    if chars.peek().unwrap().is_digit(10) {
        BencodeValue::from_bencoded_string(chars)
    } else if *chars.peek().unwrap() == 'i' {
        BencodeValue::from_bencoded_integer(chars)
    } else if *chars.peek().unwrap() == 'l' {
        BencodeValue::from_bencoded_list(chars)
    } else {
        panic!("Unhandled encoded value")
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        // println!("Logs from your program will appear here!");

        // Uncomment this block to pass the first stage
        let encoded_value = &args[2];
        let mut chars: Peekable<Chars<'_>> = encoded_value.chars().peekable();
        let decoded_value = decode_bencoded_value(&mut chars).unwrap();
        let result: serde_json::Value = decoded_value.into();
        println!("{}", result.to_string());
    } else {
        println!("unknown command: {}", args[1])
    }
}
