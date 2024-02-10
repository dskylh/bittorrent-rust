use crate::bencode;
use crate::bencode::decode_bencoded_value;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha1::Digest;
use sha1::Sha1;
use std::iter::Peekable;
use std::net::Ipv4Addr;
use std::slice::Iter;
use std::str::from_utf8;

pub struct TorrentFile {
    pub announce: String,
    pub info: TorrentFileInfo,
}

#[derive(Debug)]
pub struct TorrentResponse {
    pub interval: u64,
    pub peers: Vec<Peer>,
    pub complete: u64,
    pub incomplete: u64,
    pub min_interval: u64,
}

#[derive(Debug)]
pub struct Peer {
    pub ip_addr: Ipv4Addr,
    pub port: u16,
}

impl Peer {
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.ip_addr, self.port)
    }
}

#[derive(Serialize, Deserialize)]
pub struct TorrentFileInfo {
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: u64,
    #[serde(with = "serde_bytes")]
    pub pieces: Vec<u8>,
    pub length: u64,
}

impl TorrentFileInfo {
    pub fn hash(&self) -> String {
        let bencoded_info_dictionary = serde_bencode::to_bytes(&self).unwrap();
        let mut hasher = Sha1::new();
        hasher.update(bencoded_info_dictionary);
        let hash = hasher.finalize();
        hex::encode(hash)
    }

    pub fn hash_nohex(&self) -> Vec<u8> {
        let bencoded_info_dictionary = serde_bencode::to_bytes(&self).unwrap();
        let mut hasher = Sha1::new();
        hasher.update(bencoded_info_dictionary);
        let hash = hasher.finalize();
        return hash.to_vec();
    }

    pub fn hash_pieces(&self) -> Vec<String> {
        let mut hashed_pieces = Vec::new();
        for piece in self.pieces.chunks(20) {
            hashed_pieces.push(hex::encode(piece));
        }
        hashed_pieces
    }
}

pub fn parse_torrent_file(chars: &mut Peekable<Iter<u8>>) -> TorrentFile {
    let decoded_value = decode_bencoded_value(chars);
    let mut announce: Option<String> = None;
    let mut length: Option<u64> = None;
    let mut name: Option<String> = None;
    let mut piece_length: Option<u64> = None;
    let mut pieces: Option<Vec<u8>> = None;

    if let bencode::BencodeValue::Dictionary(dict) = decoded_value.unwrap() {
        if let bencode::BencodeValue::ByteString(s) = dict.get("announce").unwrap() {
            announce = Some(from_utf8(s).unwrap().to_owned());
        };

        if let bencode::BencodeValue::Dictionary(info) = dict.get("info").unwrap() {
            if let bencode::BencodeValue::Integer(n) = info.get("length").unwrap() {
                length = Some(n.to_owned() as u64);
            }

            if let bencode::BencodeValue::ByteString(s) = info.get("name").unwrap() {
                name = Some(from_utf8(&s).unwrap().to_owned());
            }

            if let bencode::BencodeValue::Integer(n) = info.get("piece length").unwrap() {
                piece_length = Some(n.to_owned() as u64);
            }

            if let bencode::BencodeValue::ByteString(s) = info.get("pieces").unwrap() {
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

pub fn tracker_get(torrent_file: TorrentFile) -> Result<Bytes, reqwest::Error> {
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

pub fn hash_encode(t: &str) -> String {
    let bytes = hex::decode(t).unwrap_or_default();
    let encoded: String = bytes.iter().map(|b| format!("%{:02x}", b)).collect();
    encoded
}

pub fn parse_response(bencode: bencode::BencodeValue) -> TorrentResponse {
    let mut interval: Option<u64> = None;
    let mut complete: Option<u64> = None;
    let mut incomplete: Option<u64> = None;
    let mut min_interval: Option<u64> = None;
    let mut peers: Option<Vec<Peer>> = None;

    if let bencode::BencodeValue::Dictionary(dict) = bencode {
        if let bencode::BencodeValue::Integer(n) = dict.get("interval").unwrap() {
            interval = Some(n.to_owned() as u64);
        }

        if let bencode::BencodeValue::Integer(n) = dict.get("complete").unwrap() {
            complete = Some(n.to_owned() as u64);
        }

        if let bencode::BencodeValue::Integer(n) = dict.get("incomplete").unwrap() {
            incomplete = Some(n.to_owned() as u64);
        }

        if let bencode::BencodeValue::Integer(n) = dict.get("min interval").unwrap() {
            min_interval = Some(n.to_owned() as u64);
        }

        if let bencode::BencodeValue::ByteString(p) = dict.get("peers").unwrap() {
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
