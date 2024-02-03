use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{self, json};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fs::{self};
use std::net::Ipv4Addr;
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

    fn into_json(&self) -> Option<serde_json::Value> {
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

struct TorrentFile {
    pub announce: String,
    pub info: TorrentFileInfo,
}

#[derive(Serialize, Deserialize)]
struct TorrentFileInfo {
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: u64,
    #[serde(with = "serde_bytes")]
    pub pieces: Vec<u8>,
    pub length: u64,
}

impl TorrentFileInfo {
    fn hash(&self) -> String {
        let bencoded_info_dictionary = serde_bencode::to_bytes(&self).unwrap();
        let mut hasher = Sha1::new();
        hasher.update(bencoded_info_dictionary);
        let hash = hasher.finalize();
        hex::encode(hash)
    }

    fn hash_pieces(&self) -> Vec<String> {
        let mut hashed_pieces = Vec::new();
        for piece in self.pieces.chunks(20) {
            hashed_pieces.push(hex::encode(piece));
        }
        hashed_pieces
    }
}

fn parse_torrent_file(chars: &mut Peekable<Iter<u8>>) -> TorrentFile {
    let decoded_value = decode_bencoded_value(chars);
    let mut announce: Option<String> = None;
    let mut length: Option<u64> = None;
    let mut name: Option<String> = None;
    let mut piece_length: Option<u64> = None;
    let mut pieces: Option<Vec<u8>> = None;

    if let BencodeValue::Dictionary(dict) = decoded_value.unwrap() {
        if let BencodeValue::ByteString(s) = dict.get("announce").unwrap() {
            announce = Some(from_utf8(s).unwrap().to_owned());
        };

        if let BencodeValue::Dictionary(info) = dict.get("info").unwrap() {
            if let BencodeValue::Integer(n) = info.get("length").unwrap() {
                length = Some(n.to_owned() as u64);
            }

            if let BencodeValue::ByteString(s) = info.get("name").unwrap() {
                name = Some(from_utf8(&s).unwrap().to_owned());
            }

            if let BencodeValue::Integer(n) = info.get("piece length").unwrap() {
                piece_length = Some(n.to_owned() as u64);
            }

            if let BencodeValue::ByteString(s) = info.get("pieces").unwrap() {
                pieces = Some(s.to_vec());
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
// fn hash_encode(t: String) -> String {
//     let encoded: String = t.chars().map(|b| format!("%{:02x}", b)).collect();
//     encoded
// }

fn hash_encode(t: &str) -> String {
    let bytes = hex::decode(t).unwrap_or_default();
    let encoded: String = bytes.iter().map(|b| format!("%{:02x}", b)).collect();
    encoded
}

#[allow(dead_code)]
#[derive(Debug)]
struct Peer {
    ip_addr: Ipv4Addr,
    port: u16,
}

impl Peer {
    fn to_string(&self) -> String {
        format!("{}:{}", self.ip_addr, self.port)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct TorrentResponse {
    interval: u64,
    peers: Vec<Peer>,
    complete: u64,
    incomplete: u64,
    min_interval: u64,
}

fn parse_response(bencode: BencodeValue) -> TorrentResponse {
    let mut interval: Option<u64> = None;
    let mut complete: Option<u64> = None;
    let mut incomplete: Option<u64> = None;
    let mut min_interval: Option<u64> = None;
    let mut peers: Option<Vec<Peer>> = None;

    if let BencodeValue::Dictionary(dict) = bencode {
        if let BencodeValue::Integer(n) = dict.get("interval").unwrap() {
            interval = Some(n.to_owned() as u64);
        }

        if let BencodeValue::Integer(n) = dict.get("complete").unwrap() {
            complete = Some(n.to_owned() as u64);
        }

        if let BencodeValue::Integer(n) = dict.get("incomplete").unwrap() {
            incomplete = Some(n.to_owned() as u64);
        }

        if let BencodeValue::Integer(n) = dict.get("min interval").unwrap() {
            min_interval = Some(n.to_owned() as u64);
        }

        if let BencodeValue::ByteString(p) = dict.get("peers").unwrap() {
            let mut vec = vec![];
            for chunk in p.chunks(6) {
                vec.push(Peer {
                    ip_addr: Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]),
                    port: u16::from_be_bytes([chunk[4], chunk[5]]),
                })
            }
            peers = Some(vec);
        }
    }
    TorrentResponse {
        interval: interval.unwrap(),
        min_interval: min_interval.unwrap(),
        incomplete: incomplete.unwrap(),
        complete: complete.unwrap(),
        peers: peers.unwrap(),
    }
}

fn tracker_get(torrent_file: TorrentFile) -> Result<Bytes, reqwest::Error> {
    let mut url = torrent_file.announce;
    let left = torrent_file.info.length.to_string();
    let info_hash = hash_encode(&torrent_file.info.hash());

    url.push_str(&format!("?info_hash={}", info_hash));
    url.push_str("&peer_id=00112233445566778899");
    url.push_str("&port=6881");
    url.push_str("&uploaded=0");
    url.push_str("&downloaded=0");
    url.push_str(&format!("&left={}", left));
    url.push_str("&compact=1");

    let response = reqwest::blocking::get(url).unwrap().bytes().unwrap();
    // println!("{:?}", response.to_string());

    // let mut response = binding.as_bytes().iter().peekable();
    // let response = decode_bencoded_value(&mut response).unwrap();

    Ok(response)
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
        let result: serde_json::Value = decoded_value.into_json().unwrap();
        println!("{}", result.to_string());
    } else if command == "info" {
        let file_path = &args[2];
        let contents = fs::read(file_path).unwrap();
        let mut chars = contents.iter().peekable();
        // let decoded_value = decode_bencoded_value(&mut chars).unwrap();
        // let result: serde_json::Value = decoded_value.into();
        let torrent_file = parse_torrent_file(&mut chars);
        let hashed_pieces = torrent_file.info.hash_pieces();
        println!("Tracker Url: {}", torrent_file.announce);
        println!("Length: {}", torrent_file.info.length);
        println!("Info Hash: {}", torrent_file.info.hash());
        println!("Piece Length: {}", torrent_file.info.piece_length);
        println!("Piece Hashes:");
        for hashed_piece in &hashed_pieces {
            println!("{:?}", hashed_piece);
        }
    } else if command == "peers" {
        let file_path = &args[2];
        let contents = fs::read(file_path).unwrap();
        let mut chars = contents.iter().peekable();
        // let decoded_value = decode_bencoded_value(&mut chars).unwrap();
        // let result: serde_json::Value = decoded_value.into();
        let torrent_file = parse_torrent_file(&mut chars);
        let tracker = tracker_get(torrent_file).unwrap();
        // println!("{:?}", tracker);
        let bencode_tracker = decode_bencoded_value(&mut tracker.iter().peekable());
        let parsed_response = parse_response(bencode_tracker.unwrap());
        for peer in parsed_response.peers {
            println!("{}", peer.to_string());
        }
    } else {
        println!("unknown command: {}", args[1])
    }
}
