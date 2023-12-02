use aes::{
    cipher::{generic_array::GenericArray, BlockDecrypt, KeyInit},
    Aes256,
};

pub(crate) fn decrypt_content(bytes: &[u8], key_str: &str) -> String {
    let key = GenericArray::clone_from_slice(key_str.as_bytes());

    let mut blocks = Vec::new();
    (0..bytes.len()).step_by(16).for_each(|x| {
        blocks.push(GenericArray::clone_from_slice(&bytes[x..x + 16]));
    });

    let cipher = Aes256::new(&key);
    cipher.decrypt_blocks(&mut blocks);

    let decrypted_bytes: Vec<u8> = blocks.iter().flatten().cloned().collect();

    let last_byte = decrypted_bytes.last().cloned().unwrap_or(0) as usize;
    let unpadded_bytes = &decrypted_bytes[..decrypted_bytes.len() - last_byte - 1usize];

    let result = String::from_utf8(unpadded_bytes.to_vec()).unwrap_or("[]".to_string());

    result
}
