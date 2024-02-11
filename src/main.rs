use bittorrent_starter_rust::bencode::decode_bencoded_value;
use bittorrent_starter_rust::message::{Message, MessageId};
use bittorrent_starter_rust::peer::{download, send_message, wait_message};
use bittorrent_starter_rust::{handshake, torrent};
use handshake::tcp_handshake;
use serde_json::{self};
use std::env;
use std::fs::{self};
use torrent::{parse_response, tracker_get};

// fn hash_encode(t: String) -> String {
//     let encoded: String = t.chars().map(|b| format!("%{:02x}", b)).collect();
//     encoded
// }

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
        let torrent_file = torrent::parse_torrent_file(&mut chars);
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
        let torrent_file = torrent::parse_torrent_file(&mut chars);
        let tracker = tracker_get(torrent_file).unwrap();
        // println!("{:?}", tracker);
        let bencode_tracker = decode_bencoded_value(&mut tracker.iter().peekable());
        let parsed_response = parse_response(bencode_tracker.unwrap());
        for peer in parsed_response.peers {
            println!("{}", peer.to_string());
        }
    } else if command == "handshake" {
        let file_path = &args[2];
        let contents = fs::read(file_path).unwrap();
        let mut chars = contents.iter().peekable();
        let torrent_file = torrent::parse_torrent_file(&mut chars);
        let tracker = tracker_get(torrent_file.clone()).unwrap();
        let bencode_tracker = decode_bencoded_value(&mut tracker.iter().peekable());
        let parsed_response = parse_response(bencode_tracker.unwrap());
        let peer = parsed_response.peers[0].to_string();
        tcp_handshake(&peer, torrent_file.info.hash_nohex());
    } else if command == "download_piece" {
        let output_file_path = &args[3];
        let file_path = &args[4];
        let piece_index = &args[5].parse().unwrap();
        let contents = fs::read(file_path).unwrap();
        let mut chars = contents.iter().peekable();
        let torrent_file = torrent::parse_torrent_file(&mut chars);
        let tracker = tracker_get(torrent_file.clone()).unwrap();
        let bencode_tracker = decode_bencoded_value(&mut tracker.iter().peekable());
        let parsed_response = parse_response(bencode_tracker.unwrap());
        let peer = parsed_response.peers[0].to_string();
        let mut stream = tcp_handshake(&peer, torrent_file.info.hash_nohex());
        wait_message(&mut stream, MessageId::BitField).unwrap();
        let interested_message = Message {
            message_id: MessageId::Interested,
            payload: Vec::new(),
        };
        send_message(&mut stream, interested_message);
        wait_message(&mut stream, MessageId::Unchoke).unwrap();
        let piece = download(torrent_file, &mut stream, *piece_index).unwrap();
        let _ = fs::write(&output_file_path, piece);
    } else if command == "download" {
        // let output_file_path = &args[3];
        // let file_path = &args[4];
        // let piece_index = &args[5].parse().unwrap();
        // let contents = fs::read(file_path).unwrap();
        // let mut chars = contents.iter().peekable();
        // let torrent_file = torrent::parse_torrent_file(&mut chars);
        // let tracker = tracker_get(torrent_file.clone()).unwrap();
        // let bencode_tracker = decode_bencoded_value(&mut tracker.iter().peekable());
        // let parsed_response = parse_response(bencode_tracker.unwrap());
        // let peer = parsed_response.peers[0].to_string();
        // let mut stream = tcp_handshake(&peer, torrent_file.info.hash_nohex());
        // wait_message(&mut stream, MessageId::BitField).unwrap();
        // let interested_message = Message {
        //     message_id: MessageId::Interested,
        //     payload: Vec::new(),
        // };
        // send_message(&mut stream, interested_message);
        // wait_message(&mut stream, MessageId::Unchoke).unwrap();
        // let piece_count = torrent_file.info.
        // let piece = download(torrent_file, &mut stream, *piece_index).unwrap();
        // let _ = fs::write(&output_file_path, piece);
    } else {
        println!("unknown command: {}", args[1])
    }
}
