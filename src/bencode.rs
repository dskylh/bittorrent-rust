use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::json;
use std;
use std::collections::HashMap;
use std::iter::Peekable;
use std::slice::Iter;
use std::str::from_utf8;
#[derive(Debug, Serialize, Deserialize)]
pub enum BencodeValue {
    ByteString(Vec<u8>),
    Integer(i64),
    List(Vec<BencodeValue>),
    Dictionary(HashMap<String, BencodeValue>),
}

impl<'a> BencodeValue {
    pub fn from_bencoded_string(chars: &mut Peekable<std::slice::Iter<u8>>) -> Option<Self> {
        let mut index = String::new();
        while let Some(cur) = chars.next() {
            if *cur != b':' {
                index.push(*cur as char);
            } else {
                break;
            }
        }
        let index = index.parse().unwrap();
        let string: Vec<u8> = chars.take(index).map(|&x| x).collect();
        Some(BencodeValue::ByteString(string))
    }

    pub fn from_bencoded_integer(chars: &mut Peekable<std::slice::Iter<u8>>) -> Option<Self> {
        chars.next();
        let mut number = String::new();
        while let Some(cur) = chars.next() {
            if *cur != b'e' {
                number.push(*cur as char);
            } else {
                break;
            }
        }
        Some(BencodeValue::Integer(number.parse().unwrap()))
    }

    pub fn from_bencoded_list(chars: &mut Peekable<Iter<u8>>) -> Option<Self> {
        chars.next();
        let mut values = Vec::new();
        while let Some(cur) = chars.peek() {
            if **cur != b'e' {
                values.push(decode_bencoded_value(chars).unwrap());
            } else {
                break;
            }
        }
        Some(BencodeValue::List(values))
    }

    pub fn from_bencoded_dictionary(chars: &mut Peekable<Iter<u8>>) -> Option<Self> {
        chars.next();
        let mut dict = HashMap::new();
        while let Some(cur) = chars.peek() {
            if **cur != b'e' {
                let mut k: String = String::new();
                if let Self::ByteString(value) = decode_bencoded_value(chars).unwrap() {
                    k = from_utf8(&value).unwrap().to_string();
                };
                let v = decode_bencoded_value(chars).unwrap();
                // let key = k.strip_prefix('"').unwrap();
                // let key = key.strip_suffix('"').unwrap();

                // println!("{:?}", dict);
                dict.insert(k, v);
                // print out the dict
            } else {
                break;
            }
        }

        Some(BencodeValue::Dictionary(dict))
    }

    pub fn into_json(&self) -> Option<serde_json::Value> {
        return Some(match self {
            Self::ByteString(bytes) => {
                json!(from_utf8(&bytes).unwrap())
            }

            BencodeValue::Integer(n) => json!(n),

            BencodeValue::List(arr) => {
                let collected: Vec<serde_json::Value> = arr
                    .into_iter()
                    .map(|item| item.into_json().unwrap())
                    .collect();

                serde_json::Value::Array(collected)
            }

            BencodeValue::Dictionary(dict) => {
                let mut map: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

                for (key, value) in dict.iter() {
                    map.insert(key.clone(), value.into_json().unwrap());
                }

                serde_json::Value::Object(map)
            }
        });
    }
}

pub fn decode_bencoded_value(chars: &mut Peekable<Iter<u8>>) -> Option<BencodeValue> {
    // If encoded_value starts with a digit, it's a number
    if chars.peek().unwrap().is_ascii_digit() {
        BencodeValue::from_bencoded_string(chars)
    } else if **chars.peek().unwrap() == b'i' {
        BencodeValue::from_bencoded_integer(chars)
    } else if **chars.peek().unwrap() == b'l' {
        BencodeValue::from_bencoded_list(chars)
    } else if **chars.peek().unwrap() == b'd' {
        BencodeValue::from_bencoded_dictionary(chars)
    } else {
        panic!("Unhandled encoded value")
    }
}
