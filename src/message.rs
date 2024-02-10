enum MessageId {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    BitField = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
}

impl From<u8> for MessageId {
    fn from(value: u8) -> Self {
        match value {
            0 => MessageId::Choke,
            1 => MessageId::Unchoke,
            2 => MessageId::Interested,
            3 => MessageId::NotInterested,
            4 => MessageId::Have,
            5 => MessageId::BitField,
            6 => MessageId::Request,
            7 => MessageId::Piece,
            8 => MessageId::Cancel,
            _ => unreachable!(),
        }
    }
}
pub struct Message {
    MessageId: MessageId,
    Payload: Vec<u8>,
}
