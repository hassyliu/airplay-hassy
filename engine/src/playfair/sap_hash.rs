use super::hand_garble::garble;

#[inline]
pub fn rol8(input: u8, count: u32) -> u8 {
    input.rotate_left(count)
}

#[inline]
pub fn rol8x(input: u8, count: u32) -> u32 {
    let x = input as u32;
    (x << count) | (x >> (8 - count))
}

pub fn sap_hash(block_in: &[u8; 64], key_out: &mut [u8; 16]) {
    let mut block_words = [0u32; 16];
    for i in 0..16 {
        block_words[i] = u32::from_le_bytes([
            block_in[4 * i],
            block_in[4 * i + 1],
            block_in[4 * i + 2],
            block_in[4 * i + 3],
        ]);
    }

    let mut buffer0: [u8; 20] = [
        0x96, 0x5F, 0xC6, 0x53, 0xF8, 0x46, 0xCC, 0x18, 0xDF, 0xBE,
        0xB2, 0xF8, 0x38, 0xD7, 0xEC, 0x22, 0x03, 0xD1, 0x20, 0x8F,
    ];
    let mut buffer1 = [0u8; 210];
    let mut buffer2: [u8; 35] = [
        0x43, 0x54, 0x62, 0x7A, 0x18, 0xC3, 0xD6, 0xB3, 0x9A, 0x56,
        0xF6, 0x1C, 0x14, 0x3F, 0x0C, 0x1D, 0x3B, 0x36, 0x83, 0xB1,
        0x39, 0x51, 0x4A, 0xAA, 0x09, 0x3E, 0xFE, 0x44, 0xAF, 0xDE,
        0xC3, 0x20, 0x9D, 0x42, 0x3A,
    ];
    let mut buffer3 = [0u8; 132];
    let mut buffer4: [u8; 21] = [
        0xED, 0x25, 0xD1, 0xBB, 0xBC, 0x27, 0x9F, 0x02, 0xA2, 0xA9,
        0x11, 0x00, 0x0C, 0xB3, 0x52, 0xC0, 0xBD, 0xE3, 0x1B, 0x49,
        0xC7,
    ];
    let i0_index: [usize; 11] = [18, 22, 23, 0, 5, 19, 32, 31, 10, 21, 30];

    for i in 0..210usize {
        let in_word = block_words[(i % 64) >> 2];
        let in_byte = (in_word >> ((3 - (i % 4)) << 3)) & 0xff;
        buffer1[i] = in_byte as u8;
    }

    for i in 0..840u32 {
        let x = buffer1[(i.wrapping_sub(155) % 210) as usize];
        let y = buffer1[(i.wrapping_sub(57) % 210) as usize];
        let z = buffer1[(i.wrapping_sub(13) % 210) as usize];
        let w = buffer1[(i % 210) as usize];
        buffer1[(i % 210) as usize] = rol8(y, 5)
            .wrapping_add(rol8(z, 3) ^ w)
            .wrapping_sub(rol8(x, 7));
    }

    garble(&mut buffer0, &mut buffer1, &mut buffer2, &mut buffer3, &mut buffer4);

    for b in key_out.iter_mut() {
        *b = 0xE1;
    }

    for i in 0..11 {
        if i == 3 {
            key_out[i] = 0x3d;
        } else {
            key_out[i] = key_out[i].wrapping_add(buffer3[i0_index[i] * 4]);
        }
    }

    for i in 0..20 {
        key_out[i % 16] ^= buffer0[i];
    }
    for i in 0..35 {
        key_out[i % 16] ^= buffer2[i];
    }
    for i in 0..210 {
        key_out[i % 16] ^= buffer1[i];
    }

    for _j in 0..16 {
        for i in 0..16u32 {
            let x = key_out[(i.wrapping_sub(7) % 16) as usize];
            let y = key_out[(i % 16) as usize];
            let z = key_out[(i.wrapping_sub(37) % 16) as usize];
            let w = key_out[(i.wrapping_sub(177) % 16) as usize];
            key_out[i as usize] = rol8(x, 1) ^ y ^ rol8(z, 6) ^ rol8(w, 5);
        }
    }
}
