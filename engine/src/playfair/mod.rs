mod hand_garble;
mod modified_md5;
mod omg_hax;
mod sap_hash;
mod tables;

/// FairPlay SAP v3 key decryption (ported from PlayFair / UxPlay).
/// `message3` is the 164-byte stage-2 `/fp-setup` client message.
/// `cipher_text` is the 72-byte `ekey` from RTSP `SETUP`.
/// Returns the 16-byte decrypted AES key.
pub fn playfair_decrypt(message3: &[u8; 164], cipher_text: &[u8; 72]) -> [u8; 16] {
    let chunk1 = &cipher_text[16..32];
    let chunk2 = &cipher_text[56..72];

    let mut sap_key = [0u8; 16];
    omg_hax::generate_session_key(&omg_hax::DEFAULT_SAP, message3, &mut sap_key);

    let mut key_schedule = [[0u32; 4]; 11];
    omg_hax::generate_key_schedule(&sap_key, &mut key_schedule);

    let mut block_in = [0u8; 16];
    omg_hax::z_xor(chunk2, &mut block_in, 1);
    omg_hax::cycle(&mut block_in, &key_schedule);

    let mut key_out = [0u8; 16];
    for i in 0..16 {
        key_out[i] = block_in[i] ^ chunk1[i];
    }
    let tmp = key_out;
    omg_hax::x_xor(&tmp, &mut key_out, 1);
    let tmp2 = key_out;
    omg_hax::z_xor(&tmp2, &mut key_out, 1);
    key_out
}
