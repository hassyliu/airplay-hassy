use super::modified_md5::modified_md5;
use super::sap_hash::sap_hash;
use super::tables::*;

const INDEX_MANGLE: [u8; 11] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36, 0x6C];

const INITIAL_SESSION_KEY: [u8; 16] = [
    0xDC, 0xDC, 0xF3, 0xB9, 0x0B, 0x74, 0xDC, 0xFB,
    0x86, 0x7F, 0xF7, 0x60, 0x16, 0x72, 0x90, 0x51,
];

const STATIC_SOURCE_1: [u8; 17] = [
    0xFA, 0x9C, 0xAD, 0x4D, 0x4B, 0x68, 0x26, 0x8C,
    0x7F, 0xF3, 0x88, 0x99, 0xDE, 0x92, 0x2E, 0x95,
    0x1E,
];

const STATIC_SOURCE_2: [u8; 47] = [
    0xEC, 0x4E, 0x27, 0x5E, 0xFD, 0xF2, 0xE8, 0x30,
    0x97, 0xAE, 0x70, 0xFB, 0xE0, 0x00, 0x3F, 0x1C,
    0x39, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
    0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

pub const DEFAULT_SAP: [u8; 276] = [
    0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79,
    0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79,
    0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79,
    0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79, 0x79,
    0x79, 0x79, 0x79, 0x79, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x03, 0x02, 0x53,
    0x00, 0x01, 0xcc, 0x34, 0x2a, 0x5e, 0x5b, 0x1a, 0x67, 0x73, 0xc2, 0x0e, 0x21, 0xb8, 0x22, 0x4d,
    0xf8, 0x62, 0x48, 0x18, 0x64, 0xef, 0x81, 0x0a, 0xae, 0x2e, 0x37, 0x03, 0xc8, 0x81, 0x9c, 0x23,
    0x53, 0x9d, 0xe5, 0xf5, 0xd7, 0x49, 0xbc, 0x5b, 0x7a, 0x26, 0x6c, 0x49, 0x62, 0x83, 0xce, 0x7f,
    0x03, 0x93, 0x7a, 0xe1, 0xf6, 0x16, 0xde, 0x0c, 0x15, 0xff, 0x33, 0x8c, 0xca, 0xff, 0xb0, 0x9e,
    0xaa, 0xbb, 0xe4, 0x0f, 0x5d, 0x5f, 0x55, 0x8f, 0xb9, 0x7f, 0x17, 0x31, 0xf8, 0xf7, 0xda, 0x60,
    0xa0, 0xec, 0x65, 0x79, 0xc3, 0x3e, 0xa9, 0x83, 0x12, 0xc3, 0xb6, 0x71, 0x35, 0xa6, 0x69, 0x4f,
    0xf8, 0x23, 0x05, 0xd9, 0xba, 0x5c, 0x61, 0x5f, 0xa2, 0x54, 0xd2, 0xb1, 0x83, 0x45, 0x83, 0xce,
    0xe4, 0x2d, 0x44, 0x26, 0xc8, 0x35, 0xa7, 0xa5, 0xf6, 0xc8, 0x42, 0x1c, 0x0d, 0xa3, 0xf1, 0xc7,
    0x00, 0x50, 0xf2, 0xe5, 0x17, 0xf8, 0xd0, 0xfa, 0x77, 0x8d, 0xfb, 0x82, 0x8d, 0x40, 0xc7, 0x8e,
    0x94, 0x1e, 0x1e, 0x1e,
];

#[inline]
fn xor_blocks(a: &[u8], b: &[u8], out: &mut [u8]) {
    for i in 0..16 {
        out[i] = a[i] ^ b[i];
    }
}

pub fn z_xor(input: &[u8], output: &mut [u8], blocks: usize) {
    for j in 0..blocks {
        for i in 0..16 {
            output[j * 16 + i] = input[j * 16 + i] ^ Z_KEY[i];
        }
    }
}

pub fn x_xor(input: &[u8], output: &mut [u8], blocks: usize) {
    for j in 0..blocks {
        for i in 0..16 {
            output[j * 16 + i] = input[j * 16 + i] ^ X_KEY[i];
        }
    }
}

fn t_xor(input: &[u8], output: &mut [u8]) {
    for i in 0..16 {
        output[i] = input[i] ^ T_KEY[i];
    }
}

#[inline]
fn table_index(i: usize) -> &'static [u8] {
    let off = ((31 * i) % 0x28) << 8;
    &TABLE_S1[off..off + 256]
}

#[inline]
fn message_table_index(i: usize) -> &'static [u8] {
    let off = ((97 * i) % 144) << 8;
    &TABLE_S2[off..off + 256]
}

#[inline]
fn permute_table_2(i: usize) -> &'static [u8] {
    let off = ((71 * i) % 144) << 8;
    &TABLE_S4[off..off + 256]
}

fn permute_block_1(block: &mut [u8; 16]) {
    block[0] = TABLE_S3[block[0] as usize];
    block[4] = TABLE_S3[0x400 + block[4] as usize];
    block[8] = TABLE_S3[0x800 + block[8] as usize];
    block[12] = TABLE_S3[0xc00 + block[12] as usize];

    let mut tmp = block[13];
    block[13] = TABLE_S3[0x100 + block[9] as usize];
    block[9] = TABLE_S3[0xd00 + block[5] as usize];
    block[5] = TABLE_S3[0x900 + block[1] as usize];
    block[1] = TABLE_S3[0x500 + tmp as usize];

    tmp = block[2];
    block[2] = TABLE_S3[0xa00 + block[10] as usize];
    block[10] = TABLE_S3[0x200 + tmp as usize];
    tmp = block[6];
    block[6] = TABLE_S3[0xe00 + block[14] as usize];
    block[14] = TABLE_S3[0x600 + tmp as usize];

    tmp = block[3];
    block[3] = TABLE_S3[0xf00 + block[7] as usize];
    block[7] = TABLE_S3[0x300 + block[11] as usize];
    block[11] = TABLE_S3[0x700 + block[15] as usize];
    block[15] = TABLE_S3[0xb00 + tmp as usize];
}

fn permute_block_2(block: &mut [u8; 16], round: usize) {
    block[0] = permute_table_2(round * 16 + 0)[block[0] as usize];
    block[4] = permute_table_2(round * 16 + 4)[block[4] as usize];
    block[8] = permute_table_2(round * 16 + 8)[block[8] as usize];
    block[12] = permute_table_2(round * 16 + 12)[block[12] as usize];

    let mut tmp = block[13];
    block[13] = permute_table_2(round * 16 + 13)[block[9] as usize];
    block[9] = permute_table_2(round * 16 + 9)[block[5] as usize];
    block[5] = permute_table_2(round * 16 + 5)[block[1] as usize];
    block[1] = permute_table_2(round * 16 + 1)[tmp as usize];

    tmp = block[2];
    block[2] = permute_table_2(round * 16 + 2)[block[10] as usize];
    block[10] = permute_table_2(round * 16 + 10)[tmp as usize];
    tmp = block[6];
    block[6] = permute_table_2(round * 16 + 6)[block[14] as usize];
    block[14] = permute_table_2(round * 16 + 14)[tmp as usize];

    tmp = block[3];
    block[3] = permute_table_2(round * 16 + 3)[block[7] as usize];
    block[7] = permute_table_2(round * 16 + 7)[block[11] as usize];
    block[11] = permute_table_2(round * 16 + 11)[block[15] as usize];
    block[15] = permute_table_2(round * 16 + 15)[tmp as usize];
}

#[inline]
fn bytes_to_words(b: &[u8; 16]) -> [u32; 4] {
    [
        u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        u32::from_le_bytes([b[4], b[5], b[6], b[7]]),
        u32::from_le_bytes([b[8], b[9], b[10], b[11]]),
        u32::from_le_bytes([b[12], b[13], b[14], b[15]]),
    ]
}

#[inline]
fn words_to_bytes(w: &[u32; 4], out: &mut [u8; 16]) {
    for i in 0..4 {
        out[4 * i..4 * i + 4].copy_from_slice(&w[i].to_le_bytes());
    }
}

pub fn generate_key_schedule(key_material: &[u8; 16], key_schedule: &mut [[u32; 4]; 11]) {
    let mut buffer = [0u8; 16];
    t_xor(key_material, &mut buffer);
    let mut ti = 0usize;

    for round in 0..11 {
        let w0 = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        key_schedule[round][0] = w0;

        let table1 = table_index(ti);
        let table2 = table_index(ti + 1);
        let table3 = table_index(ti + 2);
        let table4 = table_index(ti + 3);
        ti += 4;

        buffer[0] ^= table1[buffer[0x0d] as usize] ^ INDEX_MANGLE[round];
        buffer[1] ^= table2[buffer[0x0e] as usize];
        buffer[2] ^= table3[buffer[0x0f] as usize];
        buffer[3] ^= table4[buffer[0x0c] as usize];

        let mut kd = bytes_to_words(&buffer);
        key_schedule[round][1] = kd[1];
        kd[1] ^= kd[0];
        key_schedule[round][2] = kd[2];
        kd[2] ^= kd[1];
        key_schedule[round][3] = kd[3];
        kd[3] ^= kd[2];
        words_to_bytes(&kd, &mut buffer);
    }
}

pub fn cycle(block: &mut [u8; 16], key_schedule: &[[u32; 4]; 11]) {
    let mut bw = bytes_to_words(block);
    for i in 0..4 {
        bw[i] ^= key_schedule[10][i];
    }
    words_to_bytes(&bw, block);
    permute_block_1(block);

    for round in 0..9 {
        let key0 = key_schedule[9 - round][0].to_le_bytes();
        let w0 = TABLE_S5[(block[3] ^ key0[3]) as usize]
            ^ TABLE_S6[(block[2] ^ key0[2]) as usize]
            ^ TABLE_S8[(block[0] ^ key0[0]) as usize]
            ^ TABLE_S7[(block[1] ^ key0[1]) as usize];
        block[0..4].copy_from_slice(&w0.to_le_bytes());

        let key1 = key_schedule[9 - round][1].to_le_bytes();
        let w1 = TABLE_S5[(block[7] ^ key1[3]) as usize]
            ^ TABLE_S6[(block[6] ^ key1[2]) as usize]
            ^ TABLE_S7[(block[5] ^ key1[1]) as usize]
            ^ TABLE_S8[(block[4] ^ key1[0]) as usize];
        block[4..8].copy_from_slice(&w1.to_le_bytes());

        let key2 = key_schedule[9 - round][2].to_le_bytes();
        let w2 = TABLE_S5[(block[11] ^ key2[3]) as usize]
            ^ TABLE_S6[(block[10] ^ key2[2]) as usize]
            ^ TABLE_S7[(block[9] ^ key2[1]) as usize]
            ^ TABLE_S8[(block[8] ^ key2[0]) as usize];
        block[8..12].copy_from_slice(&w2.to_le_bytes());

        let key3 = key_schedule[9 - round][3].to_le_bytes();
        let w3 = TABLE_S5[(block[15] ^ key3[3]) as usize]
            ^ TABLE_S6[(block[14] ^ key3[2]) as usize]
            ^ TABLE_S7[(block[13] ^ key3[1]) as usize]
            ^ TABLE_S8[(block[12] ^ key3[0]) as usize];
        block[12..16].copy_from_slice(&w3.to_le_bytes());

        permute_block_2(block, 8 - round);
    }

    let mut bw = bytes_to_words(block);
    for i in 0..4 {
        bw[i] ^= key_schedule[0][i];
    }
    words_to_bytes(&bw, block);
}

fn decrypt_message(message_in: &[u8; 164], decrypted_message: &mut [u8; 128]) {
    let mode = message_in[12] as usize;
    let mut buffer = [0u8; 16];

    for i in 0..8usize {
        if mode == 3 {
            let start = 0x80 - 0x10 * i;
            buffer.copy_from_slice(&message_in[start..start + 16]);
        } else {
            let start = 0x10 * (i + 1);
            buffer.copy_from_slice(&message_in[start..start + 16]);
        }

        for j in 0..9usize {
            let base = 0x80 - 0x10 * j;
            let mk = &MESSAGE_KEY[mode];

            buffer[0x0] = message_table_index(base + 0x0)[buffer[0x0] as usize] ^ mk[base + 0x0];
            buffer[0x4] = message_table_index(base + 0x4)[buffer[0x4] as usize] ^ mk[base + 0x4];
            buffer[0x8] = message_table_index(base + 0x8)[buffer[0x8] as usize] ^ mk[base + 0x8];
            buffer[0xc] = message_table_index(base + 0xc)[buffer[0xc] as usize] ^ mk[base + 0xc];

            let mut tmp = buffer[0x0d];
            buffer[0xd] = message_table_index(base + 0xd)[buffer[0x9] as usize] ^ mk[base + 0xd];
            buffer[0x9] = message_table_index(base + 0x9)[buffer[0x5] as usize] ^ mk[base + 0x9];
            buffer[0x5] = message_table_index(base + 0x5)[buffer[0x1] as usize] ^ mk[base + 0x5];
            buffer[0x1] = message_table_index(base + 0x1)[tmp as usize] ^ mk[base + 0x1];

            tmp = buffer[0x02];
            buffer[0x2] = message_table_index(base + 0x2)[buffer[0xa] as usize] ^ mk[base + 0x2];
            buffer[0xa] = message_table_index(base + 0xa)[tmp as usize] ^ mk[base + 0xa];
            tmp = buffer[0x06];
            buffer[0x6] = message_table_index(base + 0x6)[buffer[0xe] as usize] ^ mk[base + 0x6];
            buffer[0xe] = message_table_index(base + 0xe)[tmp as usize] ^ mk[base + 0xe];

            tmp = buffer[0x3];
            buffer[0x3] = message_table_index(base + 0x3)[buffer[0x7] as usize] ^ mk[base + 0x3];
            buffer[0x7] = message_table_index(base + 0x7)[buffer[0xb] as usize] ^ mk[base + 0x7];
            buffer[0xb] = message_table_index(base + 0xb)[buffer[0xf] as usize] ^ mk[base + 0xb];
            buffer[0xf] = message_table_index(base + 0xf)[tmp as usize] ^ mk[base + 0xf];

            let w0 = TABLE_S9[0x000 + buffer[0x0] as usize]
                ^ TABLE_S9[0x100 + buffer[0x1] as usize]
                ^ TABLE_S9[0x200 + buffer[0x2] as usize]
                ^ TABLE_S9[0x300 + buffer[0x3] as usize];
            let w1 = TABLE_S9[0x000 + buffer[0x4] as usize]
                ^ TABLE_S9[0x100 + buffer[0x5] as usize]
                ^ TABLE_S9[0x200 + buffer[0x6] as usize]
                ^ TABLE_S9[0x300 + buffer[0x7] as usize];
            let w2 = TABLE_S9[0x000 + buffer[0x8] as usize]
                ^ TABLE_S9[0x100 + buffer[0x9] as usize]
                ^ TABLE_S9[0x200 + buffer[0xa] as usize]
                ^ TABLE_S9[0x300 + buffer[0xb] as usize];
            let w3 = TABLE_S9[0x000 + buffer[0xc] as usize]
                ^ TABLE_S9[0x100 + buffer[0xd] as usize]
                ^ TABLE_S9[0x200 + buffer[0xe] as usize]
                ^ TABLE_S9[0x300 + buffer[0xf] as usize];
            words_to_bytes(&[w0, w1, w2, w3], &mut buffer);
        }

        buffer[0x0] = TABLE_S10[(0x0 << 8) + buffer[0x0] as usize];
        buffer[0x4] = TABLE_S10[(0x4 << 8) + buffer[0x4] as usize];
        buffer[0x8] = TABLE_S10[(0x8 << 8) + buffer[0x8] as usize];
        buffer[0xc] = TABLE_S10[(0xc << 8) + buffer[0xc] as usize];

        let mut tmp = buffer[0x0d];
        buffer[0xd] = TABLE_S10[(0xd << 8) + buffer[0x9] as usize];
        buffer[0x9] = TABLE_S10[(0x9 << 8) + buffer[0x5] as usize];
        buffer[0x5] = TABLE_S10[(0x5 << 8) + buffer[0x1] as usize];
        buffer[0x1] = TABLE_S10[(0x1 << 8) + tmp as usize];

        tmp = buffer[0x02];
        buffer[0x2] = TABLE_S10[(0x2 << 8) + buffer[0xa] as usize];
        buffer[0xa] = TABLE_S10[(0xa << 8) + tmp as usize];
        tmp = buffer[0x06];
        buffer[0x6] = TABLE_S10[(0x6 << 8) + buffer[0xe] as usize];
        buffer[0xe] = TABLE_S10[(0xe << 8) + tmp as usize];

        tmp = buffer[0x3];
        buffer[0x3] = TABLE_S10[(0x3 << 8) + buffer[0x7] as usize];
        buffer[0x7] = TABLE_S10[(0x7 << 8) + buffer[0xb] as usize];
        buffer[0xb] = TABLE_S10[(0xb << 8) + buffer[0xf] as usize];
        buffer[0xf] = TABLE_S10[(0xf << 8) + tmp as usize];

        if mode == 2 || mode == 1 || mode == 0 {
            let off = 0x10 * i;
            if i > 0 {
                xor_blocks(&buffer, &message_in[off..off + 16], &mut decrypted_message[off..off + 16]);
            } else {
                xor_blocks(&buffer, &MESSAGE_IV[mode], &mut decrypted_message[off..off + 16]);
            }
        } else {
            let off = 0x70 - 0x10 * i;
            if i < 7 {
                xor_blocks(&buffer, &message_in[off..off + 16], &mut decrypted_message[off..off + 16]);
            } else {
                xor_blocks(&buffer, &MESSAGE_IV[mode], &mut decrypted_message[off..off + 16]);
            }
        }
    }
}

pub fn generate_session_key(old_sap: &[u8; 276], message_in: &[u8; 164], session_key: &mut [u8; 16]) {
    let mut decrypted_message = [0u8; 128];
    let mut new_sap = [0u8; 320];
    let mut md5 = [0u8; 16];

    decrypt_message(message_in, &mut decrypted_message);

    new_sap[0x000..0x011].copy_from_slice(&STATIC_SOURCE_1);
    new_sap[0x011..0x091].copy_from_slice(&decrypted_message);
    new_sap[0x091..0x111].copy_from_slice(&old_sap[0x80..0x100]);
    new_sap[0x111..0x140].copy_from_slice(&STATIC_SOURCE_2);
    session_key.copy_from_slice(&INITIAL_SESSION_KEY);

    for round in 0..5 {
        let mut base = [0u8; 64];
        base.copy_from_slice(&new_sap[round * 64..(round + 1) * 64]);
        modified_md5(&base, session_key, &mut md5);
        sap_hash(&base, session_key);

        let mut sk_words = bytes_to_words(session_key);
        let md5_words = bytes_to_words(&md5);
        for i in 0..4 {
            sk_words[i] = sk_words[i].wrapping_add(md5_words[i]);
        }
        words_to_bytes(&sk_words, session_key);
    }

    for i in (0..16).step_by(4) {
        session_key.swap(i, i + 3);
        session_key.swap(i + 1, i + 2);
    }

    for b in session_key.iter_mut() {
        *b ^= 121;
    }
}
