use anyhow::Context;
use serde::de::Error;
use serde::{Deserialize, Serialize};
use serde_bencode::to_bytes;
use serde_json::{self};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fs::{self};
use std::slice::Iter;
use std::str::from_utf8;
use std::u8;
use std::{env, iter::Peekable};

#[derive(Debug, Serialize, Deserialize)]
enum BencodeValue {
    ByteString(Vec<u8>),
    Integer(i64),
    List(Vec<BencodeValue>),
    Dictionary(HashMap<String, BencodeValue>),
}

impl<'a> BencodeValue {
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
                let mut k = String::new();
                if let Self::ByteString(value) = decode_bencoded_value(chars).unwrap() {
                    k = from_utf8(&value).unwrap().to_string();
                };
                let v = decode_bencoded_value(chars).unwrap();
                let key = k.strip_prefix('"').unwrap();
                let key = key.strip_suffix('"').unwrap();

                println!("{:?}", dict);
                dict.insert(key.to_string(), v);
                // print out the dict
            } else {
                break;
            }
        }

        Some(BencodeValue::Dictionary(dict))
    }

    // fn get_info(&self) -> Option<(BencodeValue, BencodeValue, Vec<u8>)> {
    //     match self {
    //         BencodeValue::Dictionary(dict) => {
    //             let announce = dict.get("announce").unwrap();
    //             let length = dict.get("info").unwrap();
    //             let info = dict.get("info").unwrap();
    //             println!("{:?}", info);
    //             let mut hasher: Sha1 = Sha1::new();
    //             let bencoded_info = to_bytes(&info).unwrap();
    //             println!("{:?}", bencoded_info);
    //             hasher.update(bencoded_info.clone());
    //             let hashed_info = hasher.finalize();
    //             return Some((announce, length, hashed_info.to_vec()));
    //         }
    //         _ => return None,
    //     }
    // }
}

fn parse_torrent_file<'a>(chars: &mut Peekable<Iter<u8>>) -> TorrentFile<'a> {
    let decoded_value = decode_bencoded_value(chars);
    let mut announce: Option<&str> = None;
    let mut length: Option<u64> = None;
    let mut name: Option<&str> = None;
    let mut piece_length: Option<u64> = None;
    let mut pieces: Option<&[u8]> = None;

    if let BencodeValue::Dictionary(dict) = decoded_value.unwrap() {
        if let BencodeValue::ByteString(s) = dict.get("announce").unwrap() {
            announce = Some(from_utf8(&s).unwrap());
        };

        if let BencodeValue::Dictionary(info) = dict.get("info").unwrap() {
            if let BencodeValue::Integer(n) = info.get("length").unwrap() {
                length = Some(n.to_owned() as u64);
            }

            if let BencodeValue::ByteString(s) = info.get("name").unwrap() {
                name = Some(from_utf8(&s).unwrap());
            }

            if let BencodeValue::Integer(n) = info.get("piece length").unwrap() {
                piece_length = Some(n.to_owned() as u64);
            }

            if let BencodeValue::ByteString(s) = info.get("pieces").unwrap() {
                pieces = Some(s);
            }
        }
    }
    TorrentFile {
        announce: announce.unwrap(),
        info: TorrentFileInfo {
            length: length.unwrap(),
            name: name.unwrap(),
            piece_length: piece_length.unwrap(),
            pieces: pieces.unwrap(),
        },
    }
}

//     fn get_info(&self) -> Option<(BencodeValue, BencodeValue, Vec<u8>)> {
//         match self {
//             BencodeValue::Dictionary(dict) => {
//                 let announce = dict.get("announce").unwrap();
//                 let length = dict.get("info").unwrap();
//                 let info = dict.get("info").unwrap();
//                 println!("{:?}", info);
//                 let mut hasher: Sha1 = Sha1::new();
//                 let bencoded_info = to_bytes(&info).unwrap();
//                 println!("{:?}", bencoded_info);
//                 hasher.update(bencoded_info.clone());
//                 let hashed_info = hasher.finalize();
//                 return Some((announce, length, hashed_info.to_vec()));
//             }
//             _ => return None,
//         }
//     }
// }

pub struct TorrentFile<'input> {
    pub announce: &'input str,
    pub info: TorrentFileInfo<'input>,
}

#[derive(Serialize)]

pub struct TorrentFileInfo<'input> {
    pub name: &'input str,
    #[serde(rename = "piece length")]
    pub piece_length: u64,
    #[serde(with = "serde_bytes")]
    pub pieces: &'input [u8],
    pub length: u64,
}

impl<'input> TorrentFileInfo<'input> {
    pub fn hash(&self) -> String {
        let bencoded_info_dictionary = serde_bencode::to_bytes(&self).unwrap();
        let mut hasher = Sha1::new();
        hasher.update(bencoded_info_dictionary);
        let hash = hasher.finalize();
        hex::encode(hash)
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
