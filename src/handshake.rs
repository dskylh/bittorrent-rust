use std::{
    io::{Read, Write},
    net::TcpStream,
};

#[repr(C)]
#[repr(packed)]
pub struct Handshake {
    pub protocol_length: u8,
    pub protocol: [u8; 19],
    pub reserved_bytes: [u8; 8],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
}

const HANDSHAKE_SIZE: usize = std::mem::size_of::<Handshake>();

impl Handshake {
    pub fn new(info_hash: [u8; 20]) -> Self {
        Self {
            protocol_length: 19,
            protocol: *b"BitTorrent protocol",
            reserved_bytes: [0; 8],
            info_hash,
            peer_id: *b"00112233445566778899",
        }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let bytes = self as *mut Self as *mut [u8; HANDSHAKE_SIZE];
        // Safety: Self is a POD with repr(c) and repr(packed)
        let bytes: &mut [u8; HANDSHAKE_SIZE] = unsafe { &mut *bytes };
        bytes
    }
}

pub fn tcp_handshake(peer_addr: &str, info_hash: Vec<u8>) -> TcpStream {
    let mut stream = TcpStream::connect(peer_addr).unwrap();
    let mut handshake = Handshake::new(info_hash.try_into().unwrap());
    stream.write(handshake.as_bytes_mut()).unwrap();
    let mut buffer = [0; 68];
    let bytes_read = stream.read(&mut buffer[..]).unwrap();
    let response = buffer[..bytes_read][48..].to_vec();
    println!("Server response: {:?}", hex::encode(response));
    stream
}
