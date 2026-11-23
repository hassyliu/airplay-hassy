const SHIFT: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

const K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
    0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
    0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
    0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
    0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
    0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
    0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
    0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
    0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

#[inline]
fn f_fn(b: u32, c: u32, d: u32) -> u32 {
    (b & c) | (!b & d)
}

#[inline]
fn g_fn(b: u32, c: u32, d: u32) -> u32 {
    (b & d) | (c & !d)
}

#[inline]
fn h_fn(b: u32, c: u32, d: u32) -> u32 {
    b ^ c ^ d
}

#[inline]
fn i_fn(b: u32, c: u32, d: u32) -> u32 {
    c ^ (b | !d)
}

pub fn modified_md5(original_block_in: &[u8; 64], key_in: &[u8; 16], key_out: &mut [u8; 16]) {
    let mut block_in = *original_block_in;
    let mut key_words = [0u32; 4];
    for idx in 0..4 {
        key_words[idx] = u32::from_le_bytes([
            key_in[4 * idx],
            key_in[4 * idx + 1],
            key_in[4 * idx + 2],
            key_in[4 * idx + 3],
        ]);
    }

    let mut a = key_words[0];
    let mut b = key_words[1];
    let mut c = key_words[2];
    let mut d = key_words[3];

    for i in 0..64usize {
        let j = if i < 16 {
            i
        } else if i < 32 {
            (5 * i + 1) % 16
        } else if i < 48 {
            (3 * i + 5) % 16
        } else {
            (7 * i) % 16
        };

        let input = ((block_in[4 * j] as u32) << 24)
            | ((block_in[4 * j + 1] as u32) << 16)
            | ((block_in[4 * j + 2] as u32) << 8)
            | (block_in[4 * j + 3] as u32);

        let mut z = a.wrapping_add(input).wrapping_add(K[i]);
        let func = if i < 16 {
            f_fn(b, c, d)
        } else if i < 32 {
            g_fn(b, c, d)
        } else if i < 48 {
            h_fn(b, c, d)
        } else {
            i_fn(b, c, d)
        };
        z = z.wrapping_add(func).rotate_left(SHIFT[i]);
        z = z.wrapping_add(b);

        let tmp = d;
        d = c;
        c = b;
        b = z;
        a = tmp;

        if i == 31 {
            let mut words = [0u32; 16];
            for k in 0..16 {
                words[k] = u32::from_le_bytes([
                    block_in[4 * k],
                    block_in[4 * k + 1],
                    block_in[4 * k + 2],
                    block_in[4 * k + 3],
                ]);
            }
            words.swap((a & 15) as usize, (b & 15) as usize);
            words.swap((c & 15) as usize, (d & 15) as usize);
            words.swap(((a & (15 << 4)) >> 4) as usize, ((b & (15 << 4)) >> 4) as usize);
            words.swap(((a & (15 << 8)) >> 8) as usize, ((b & (15 << 8)) >> 8) as usize);
            words.swap(((a & (15 << 12)) >> 12) as usize, ((b & (15 << 12)) >> 12) as usize);
            for k in 0..16 {
                block_in[4 * k..4 * k + 4].copy_from_slice(&words[k].to_le_bytes());
            }
        }
    }

    let out_words = [
        key_words[0].wrapping_add(a),
        key_words[1].wrapping_add(b),
        key_words[2].wrapping_add(c),
        key_words[3].wrapping_add(d),
    ];
    for idx in 0..4 {
        key_out[4 * idx..4 * idx + 4].copy_from_slice(&out_words[idx].to_le_bytes());
    }
}
