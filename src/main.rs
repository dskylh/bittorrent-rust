use bittorrent_starter_rust::{torrent, utils::decode};
use std::env;
use std::fs::{self};

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
        torrent_file.perform_handshake();
    } else if command == "download_piece" {
        let output_file_path = &args[3];
        let file_path = &args[4];
        let piece_index = &args[5].parse().unwrap();
        let contents = fs::read(file_path).unwrap();
        let mut chars = contents.iter().peekable();
        let torrent_file = torrent::parse_torrent_file(&mut chars);
        torrent_file.download_piece(*piece_index, output_file_path)
    } else if command == "download" {
        let output_file_path = &args[3];
        let file_path = &args[4];
        let contents = fs::read(file_path).unwrap();
        let mut chars = contents.iter().peekable();
        let torrent_file = torrent::parse_torrent_file(&mut chars);
        torrent_file.download(output_file_path)
    } else {
        println!("unknown command: {}", args[1])
    }
}
