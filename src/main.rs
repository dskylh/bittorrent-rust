use base64::encode;
use serde::{Deserialize, Serialize};
use serde_bencode::to_bytes;
use serde_json::{self};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::{self};
use std::slice::Iter;
use std::u8;
use std::{env, iter::Peekable};

#[derive(Deserialize, Serialize)]
enum BencodeValue {
    ByteString(Vec<u8>),
    Integer(i64),
    List(Vec<BencodeValue>),
    Dictionary(HashMap<String, BencodeValue>),
}

impl Display for BencodeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BencodeValue::ByteString(str) => {
                write!(f, "({:?})", str)
            }
            BencodeValue::Integer(int) => {
                write!(f, "({})", int)
            }
            BencodeValue::List(list) => {
                write!(f, "[")?;
                for (index, item) in list.iter().enumerate() {
                    if index != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            BencodeValue::Dictionary(dict) => {
                write!(f, "{{")?;
                for (index, (key, value)) in dict.iter().enumerate() {
                    if index != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", key, value)?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl Into<serde_json::Value> for BencodeValue {
    fn into(self) -> serde_json::Value {
        match self {
            BencodeValue::ByteString(string) => {
                let base64_string = encode(&string);
                serde_json::Value::String(base64_string)
            }
            BencodeValue::Integer(number) => serde_json::Value::Number(number.into()),
            BencodeValue::List(list) => {
                let mut vec: Vec<serde_json::Value> = Vec::new();
                for val in list {
                    vec.push(val.into());
                }
                serde_json::Value::Array(vec)
            }
            BencodeValue::Dictionary(dict) => {
                let mut map = serde_json::Map::new();
                for (key, val) in dict {
                    map.insert(key, val.into());
                }
                serde_json::Value::Object(map)
            }
        }
    }
}

impl BencodeValue {
    fn from_bencoded_string(chars: &mut Peekable<std::slice::Iter<u8>>) -> Option<Self> {
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

    fn from_bencoded_integer(chars: &mut Peekable<std::slice::Iter<u8>>) -> Option<Self> {
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

    fn from_bencoded_list(chars: &mut Peekable<Iter<u8>>) -> Option<Self> {
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

    fn from_bencoded_dictionary(chars: &mut Peekable<Iter<u8>>) -> Option<Self> {
        chars.next();
        let mut dict = HashMap::new();
        while let Some(cur) = chars.peek() {
            if **cur != b'e' {
                let k: serde_json::Value = decode_bencoded_value(chars).unwrap().into();
                let v = decode_bencoded_value(chars).unwrap();
                let key: String = k.to_string();
                let key = key.strip_prefix('"').unwrap();
                let key = key.strip_suffix('"').unwrap();

                println!("key: {}, value: {}", key, v);
                dict.insert(key.to_string(), v);
                // print out the dict
            } else {
                break;
            }
        }

        Some(BencodeValue::Dictionary(dict))
    }

    fn get_info(&self) -> Option<(String, String, Vec<u8>)> {
        match self {
            BencodeValue::Dictionary(dict) => {
                let announce: String = dict.get("announce").unwrap().to_string();
                let length = dict.get("info").unwrap();
                let info = dict.get("info").unwrap();
                println!("{}", info);
                let mut hasher: Sha1 = Sha1::new();
                let bencoded_info = to_bytes(&info).unwrap();
                println!("{:?}", bencoded_info);
                hasher.update(bencoded_info.clone());
                let hashed_info = hasher.finalize();
                return Some((announce, length.to_string(), hashed_info.to_vec()));
            }
            _ => return None,
        }
    }
}

#[allow(dead_code)]
fn decode_bencoded_value(chars: &mut Peekable<Iter<u8>>) -> Option<BencodeValue> {
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

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        // println!("Logs from your program will appear here!");

        // Uncomment this block to pass the first stage
        let encoded_value = &args[2];

        let mut chars = encoded_value.as_bytes().iter().peekable();
        let decoded_value = decode_bencoded_value(&mut chars).unwrap();
        let result: serde_json::Value = decoded_value.into();
        println!("{}", result.to_string());
    } else if command == "info" {
        let file_path = &args[2];
        let contents = fs::read(file_path).unwrap();
        let mut chars = contents.iter().peekable();
        let decoded_value = decode_bencoded_value(&mut chars).unwrap();
        // let result: serde_json::Value = decoded_value.into();
        let (announce, length, info) = decoded_value.get_info().unwrap();
        println!("Tracker URL: {}", announce);
        println!("Length: {}", length);
        println!("Info: {:?}", hex::encode(info));
    } else {
        println!("unknown command: {}", args[1])
    }
}
