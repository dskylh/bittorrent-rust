use super::decode_bencoded_value;
use crate::bencode;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha1::Digest;
use sha1::Sha1;
use std::io::{Read, Write};
use std::iter::Peekable;
use std::net::Ipv4Addr;
use std::net::TcpStream;
use std::slice::Iter;
use std::str::from_utf8;

pub(crate) struct TorrentFile {
    pub announce: String,
    pub info: TorrentFileInfo,
}
#[allow(dead_code)]
#[derive(Debug)]
pub struct TorrentResponse {
    pub interval: u64,
    pub peers: Vec<Peer>,
    pub complete: u64,
    pub incomplete: u64,
    pub min_interval: u64,
}
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct Peer {
    pub ip_addr: Ipv4Addr,
    pub port: u16,
}
impl Peer {
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.ip_addr, self.port)
    }
}
#[derive(Serialize, Deserialize)]
pub(crate) struct TorrentFileInfo {
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: u64,
    #[serde(with = "serde_bytes")]
    pub pieces: Vec<u8>,
    pub length: u64,
}

impl TorrentFileInfo {
    pub(crate) fn hash(&self) -> String {
        let bencoded_info_dictionary = serde_bencode::to_bytes(&self).unwrap();
        let mut hasher = Sha1::new();
        hasher.update(bencoded_info_dictionary);
        let hash = hasher.finalize();
        hex::encode(hash)
    }

    pub(crate) fn hash_nohex(&self) -> Vec<u8> {
        let bencoded_info_dictionary = serde_bencode::to_bytes(&self).unwrap();
        let mut hasher = Sha1::new();
        hasher.update(bencoded_info_dictionary);
        let hash = hasher.finalize();
        return hash.to_vec();
    }

    pub(crate) fn hash_pieces(&self) -> Vec<String> {
        let mut hashed_pieces = Vec::new();
        for piece in self.pieces.chunks(20) {
            hashed_pieces.push(hex::encode(piece));
        }
        hashed_pieces
    }
}

pub(crate) fn parse_torrent_file(chars: &mut Peekable<Iter<u8>>) -> TorrentFile {
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

pub fn tcp_handshake(peer: &str, info_hash: Vec<u8>) {
    let mut stream = TcpStream::connect(peer).unwrap();

    let mut message: Vec<u8> = Vec::new();
    message.push(19);
    message.extend_from_slice("BitTorrent protocol".as_bytes());
    message.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);
    message.extend(info_hash);
    message.extend_from_slice("00112233445566778899".as_bytes());

    stream.write(&message).unwrap();
    let mut buffer = [0; 68];
    let bytes_read = stream.read(&mut buffer[..]).unwrap();
    let response = buffer[..bytes_read][48..].to_vec();
    println!("Server response: {:?}", hex::encode(response));
}
