use bittorrent_starter_rust::bencode::decode_bencoded_value;
use bittorrent_starter_rust::message::{Message, MessageId};
use bittorrent_starter_rust::peer::{download_all, download_piece, send_message, wait_message};
use bittorrent_starter_rust::utils::decode;
use bittorrent_starter_rust::{handshake, torrent};
use handshake::tcp_handshake;
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
        let encoded_value = &args[2];
        decode(encoded_value);
    } else if command == "info" {
        let file_path = &args[2];
        let contents = fs::read(file_path).unwrap();
        let mut chars = contents.iter().peekable();
        let torrent_file = torrent::parse_torrent_file(&mut chars);
        torrent_file.show_info();
    } else if command == "peers" {
        let file_path = &args[2];
        let contents = fs::read(file_path).unwrap();
        let mut chars = contents.iter().peekable();
        let torrent_file = torrent::parse_torrent_file(&mut chars);
        let peers = torrent_file.peers();
        for peer in peers {
            println!("Peer: {}", peer.ip_addr.to_string())
        }
    } else if command == "handshake" {
        let file_path = &args[2];
        let contents = fs::read(file_path).unwrap();
        let mut chars = contents.iter().peekable();
        let torrent_file = torrent::parse_torrent_file(&mut chars);
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
        let piece = download_piece(torrent_file, &mut stream, *piece_index).unwrap();
        let _ = fs::write(&output_file_path, piece);
    } else if command == "download" {
        let output_file_path = &args[3];
        let file_path = &args[4];
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
        let piece = download_all(torrent_file, &mut stream);
        let _ = fs::write(&output_file_path, piece);
    } else {
        println!("unknown command: {}", args[1])
    }
}
