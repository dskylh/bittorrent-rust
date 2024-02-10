use std::{
    io::{Read, Write},
    net::TcpStream,
};

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
