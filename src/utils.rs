use crate::bencode::decode_bencoded_value;

pub fn decode(encoded_value: &String) {
    let mut chars = encoded_value.as_bytes().iter().peekable();
    let decoded_value = decode_bencoded_value(&mut chars).unwrap();
    let result: serde_json::Value = decoded_value.into_json().unwrap();
    println!("{}", result.to_string());
}
