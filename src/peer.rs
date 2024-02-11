use crate::message::{Message, MessageId};
use crate::torrent::TorrentFile;
use bytes::{BufMut, BytesMut};
use std::{io::Read, io::Write, net::TcpStream};

#[repr(C)]
#[repr(packed)]
pub struct Request {
    pub index: [u8; 4],
    pub begin: [u8; 4],
    pub length: [u8; 4],
}

impl Request {
    pub fn new(index: u32, begin: u32, length: u32) -> Self {
        Self {
            index: index.to_be_bytes(),
            begin: begin.to_be_bytes(),
            length: length.to_be_bytes(),
        }
    }
    pub fn index(&self) -> u32 {
        u32::from_be_bytes(self.index)
    }
    pub fn begin(&self) -> u32 {
        u32::from_be_bytes(self.begin)
    }
    pub fn length(&self) -> u32 {
        u32::from_be_bytes(self.length)
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let bytes = self as *mut Self as *mut [u8; std::mem::size_of::<Self>()];
        // Safety: Self is a POD with repr(c) and repr(packed)
        let bytes: &mut [u8; std::mem::size_of::<Self>()] = unsafe { &mut *bytes };
        bytes
    }
}

#[repr(C)]
pub struct Piece<T: ?Sized = [u8]> {
    index: [u8; 4],
    begin: [u8; 4],
    block: T,
}

impl Piece {
    pub fn index(&self) -> u32 {
        u32::from_be_bytes(self.index)
    }
    pub fn begin(&self) -> u32 {
        u32::from_be_bytes(self.begin)
    }
    pub fn block(&self) -> &[u8] {
        &self.block
    }
    const PIECE_LEAD: usize = std::mem::size_of::<Piece<()>>();
    pub fn ref_from_bytes(data: &[u8]) -> Option<&Self> {
        if data.len() < Self::PIECE_LEAD {
            return None;
        }
        let n = data.len();
        // NOTE: The slicing here looks really weird. The reason we do it is because we need the
        // length part of the fat pointer to Piece to hold the length of _just_ the `block` field.
        // And the only way we can change the length of the fat pointer to Piece is by changing the
        // length of the fat pointer to the slice, which we do by slicing it. We can't slice it at
        // the front (as it would invalidate the ptr part of the fat pointer), so we slice it at
        // the back!
        let piece = &data[..n - Self::PIECE_LEAD] as *const [u8] as *const Piece;
        // Safety: Piece is a POD with repr(c) and repr(packed), _and_ the fat pointer data length
        // is the length of the trailing DST field (thanks to the PIECE_LEAD offset).
        Some(unsafe { &*piece })
    }
}

pub fn wait_message(stream: &mut TcpStream, message_id: MessageId) -> Option<Message> {
    let mut length_bytes = [0; 4];
    stream.read_exact(&mut length_bytes).unwrap();
    let length = u32::from_be_bytes(length_bytes) as usize;
    let mut id_bytes = [0; 1];
    stream.read_exact(&mut id_bytes).unwrap();
    let id: MessageId = id_bytes[0].into();
    assert_eq!(id, message_id);
    if length == 1 {
        return Some(Message {
            message_id: id,
            payload: Vec::new(),
        });
    }

    let mut payload_bytes = Vec::with_capacity(length - 1);
    payload_bytes.resize_with(length - 1, || 0);
    stream.read_exact(&mut payload_bytes).unwrap();
    Some(Message {
        message_id: id,
        payload: payload_bytes,
    })
}

pub fn send_message(stream: &mut TcpStream, message: Message) {
    let mut buf = BytesMut::with_capacity(4 /* length */ + 1 /* tag */ + message.payload.len());
    buf.put_u32(1 + message.payload.len() as u32);
    buf.put_u8(message.message_id as u8);
    buf.put(&message.payload[..]);
    let _ = stream.write(&buf);
}

pub fn download(
    torrent_file: TorrentFile,
    stream: &mut TcpStream,
    piece_index: u32,
) -> Option<Vec<u8>> {
    let mut piece_length = torrent_file.info.piece_length as u32;
    if (piece_index + 1) as u64 * piece_length as u64 > torrent_file.info.length {
        piece_length = (torrent_file.info.length % piece_length as u64) as u32
    }
    let mut all_blocks: Vec<u8> = Vec::with_capacity(piece_length as usize);
    let block_size = 1 << 14;
    let mut block_idx = 0;
    let mut remaining = piece_length;
    while remaining > 0 {
        // Calculate begin and length
        let begin = block_idx * block_size;
        let length: u32;
        if remaining > block_size {
            length = block_size;
            remaining -= block_size;
        } else {
            length = remaining;
            remaining = 0;
        }
        // Prepare message
        let mut request = Request::new(piece_index, begin, length);
        let request_bytes = Vec::from(request.as_bytes_mut());
        let message = Message {
            message_id: MessageId::Request,
            payload: request_bytes,
        };
        // Collect a single block
        send_message(stream, message);
        let res = wait_message(stream, MessageId::Piece).unwrap();
        let piece = Piece::ref_from_bytes(&res.payload[..])
            .expect("always get all Piece response fields from peer");
        assert_eq!(piece.index() as u32, piece_index);
        assert_eq!(piece.begin() as u32, begin);
        assert_eq!(piece.block().len() as u32, length);
        all_blocks.extend(piece.block());
        block_idx += 1;
    }
    assert_eq!(all_blocks.len() as u32, piece_length);
    return Some(all_blocks);
}
