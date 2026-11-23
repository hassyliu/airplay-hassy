#![allow(non_snake_case, unused_assignments, unused_variables, unused_parens, clippy::all)]
use std::ops::*;
use super::sap_hash::{rol8, rol8x};

#[derive(Copy, Clone, Debug)]
struct W(u32);

impl Add for W { type Output = W; #[inline] fn add(self, r: W) -> W { W(self.0.wrapping_add(r.0)) } }
impl Sub for W { type Output = W; #[inline] fn sub(self, r: W) -> W { W(self.0.wrapping_sub(r.0)) } }
impl Mul for W { type Output = W; #[inline] fn mul(self, r: W) -> W { W(self.0.wrapping_mul(r.0)) } }
impl Div for W { type Output = W; #[inline] fn div(self, r: W) -> W { W(self.0 / r.0) } }
impl Rem for W { type Output = W; #[inline] fn rem(self, r: W) -> W { W(self.0 % r.0) } }
impl BitAnd for W { type Output = W; #[inline] fn bitand(self, r: W) -> W { W(self.0 & r.0) } }
impl BitOr for W { type Output = W; #[inline] fn bitor(self, r: W) -> W { W(self.0 | r.0) } }
impl BitXor for W { type Output = W; #[inline] fn bitxor(self, r: W) -> W { W(self.0 ^ r.0) } }
impl Shl for W { type Output = W; #[inline] fn shl(self, r: W) -> W { W(self.0.wrapping_shl(r.0)) } }
impl Shr for W { type Output = W; #[inline] fn shr(self, r: W) -> W { W(self.0.wrapping_shr(r.0)) } }
impl Not for W { type Output = W; #[inline] fn not(self) -> W { W(!self.0) } }
impl Neg for W { type Output = W; #[inline] fn neg(self) -> W { W(self.0.wrapping_neg()) } }

#[inline]
fn weird_ror8(input: W, count: W) -> W {
    let c = count.0;
    if c == 0 { return W(0); }
    let x = (input.0 & 0xff) as u32;
    W(((x >> c) & 0xff) | (x << (8 - c)))
}

#[inline]
fn weird_rol8(input: W, count: W) -> W {
    let c = count.0;
    if c == 0 { return W(0); }
    let x = (input.0 & 0xff) as u32;
    W(((x << c) & 0xff) | (x >> (8 - c)))
}

#[inline]
fn weird_rol32(input: W, count: W) -> W {
    let c = count.0;
    if c == 0 { return W(0); }
    let x = (input.0 & 0xff) as u32;
    W((x << c) ^ (x >> (8 - c)))
}

#[inline]
fn w_rol8(input: W, count: W) -> W {
    W(rol8(input.0 as u8, count.0) as u32)
}

#[inline]
fn w_rol8x(input: W, count: W) -> W {
    W(rol8x(input.0 as u8, count.0))
}

#[inline]
fn gb(buf: &[u8], idx: W) -> W {
    W(buf[idx.0 as usize] as u32)
}

pub fn garble(
    buffer0: &mut [u8; 20],
    buffer1: &mut [u8; 210],
    buffer2: &mut [u8; 35],
    buffer3: &mut [u8; 132],
    buffer4: &mut [u8; 21],
) {
    let (mut tmp, mut tmp2, mut tmp3) = (W(0), W(0), W(0));
    let (mut A, mut B, mut C, mut D, mut E, mut M, mut J, mut G, mut F, mut H, mut K, mut R, mut S, mut T, mut U, mut V, mut W_var, mut X, mut Y, mut Z) =
        (W(0), W(0), W(0), W(0), W(0), W(0), W(0), W(0), W(0), W(0), W(0), W(0), W(0), W(0), W(0), W(0), W(0), W(0), W(0), W(0));

    buffer2[12] = (W(0x14) + (((gb(buffer1, W(64)) & W(92)) | ((gb(buffer1, W(99)) / W(3)) & W(35))) & gb(buffer4, w_rol8x(gb(buffer4, (gb(buffer1, W(206)) % W(21))),W(4)) % W(21)))).0 as u8;
    buffer1[4] = ((gb(buffer1, W(99)) / W(5)) * (gb(buffer1, W(99)) / W(5)) * W(2)).0 as u8;
    buffer2[34] = (W(0xb8)).0 as u8;
    buffer1[153] = (W(buffer1[153] as u32) ^ ((gb(buffer2, gb(buffer1, W(203)) % W(35)) * gb(buffer2, gb(buffer1, W(203)) % W(35)) * gb(buffer1, W(190))))).0 as u8;
    buffer0[3] = (W(buffer0[3] as u32) - ((((gb(buffer4, gb(buffer1, W(205)) % W(21))>>W(1)) & W(80)) | W(0xe6440)))).0 as u8;
    buffer0[16] = (W(0x93)).0 as u8;
    buffer0[13] = (W(0x62)).0 as u8;
    buffer1[33] = (W(buffer1[33] as u32) - ((gb(buffer4, gb(buffer1, W(36)) % W(21)) & W(0xf6)))).0 as u8;
    tmp2 = gb(buffer2, gb(buffer1, W(67)) % W(35));
    buffer2[12] = (W(0x07)).0 as u8;
    tmp = gb(buffer0, gb(buffer1, W(181)) % W(20));
    buffer1[2] = (W(buffer1[2] as u32) - (W(3136))).0 as u8;
    buffer0[19] = (gb(buffer4, gb(buffer1, W(58)) % W(21))).0 as u8;
    buffer3[0] = (W(92) - gb(buffer2, gb(buffer1, W(32)) % W(35))).0 as u8;
    buffer3[4] = (gb(buffer2, gb(buffer1, W(15)) % W(35)) + W(0x9e)).0 as u8;
    buffer1[34] = (W(buffer1[34] as u32) + ((gb(buffer4, ((gb(buffer2, gb(buffer1, W(15)) % W(35)) + W(0x9e)) & W(0xff)) % W(21)) / W(5)))).0 as u8;
    buffer0[19] = (W(buffer0[19] as u32) + (W(0xfffffee6) - ((gb(buffer0, gb(buffer3, W(4)) % W(20))>>W(1)) & W(102)))).0 as u8;
    buffer1[15] = ((W(3)*(((gb(buffer1, W(72)) >> (gb(buffer4, gb(buffer1, W(190)) % W(21)) & W(7))) ^ (gb(buffer1, W(72)) << ((W(7) - (gb(buffer4, gb(buffer1, W(190)) % W(21))-W(1))&W(7))))) - (W(3)*gb(buffer4, gb(buffer1, W(126)) % W(21))))) ^ gb(buffer1, W(15))).0 as u8;
    buffer0[15] = (W(buffer0[15] as u32) ^ (gb(buffer2, gb(buffer1, W(181)) % W(35)) * gb(buffer2, gb(buffer1, W(181)) % W(35)) * gb(buffer2, gb(buffer1, W(181)) % W(35)))).0 as u8;
    buffer2[4] = (W(buffer2[4] as u32) ^ (gb(buffer1, W(202))/W(3))).0 as u8;
    A = W(92) - gb(buffer0, gb(buffer3, W(0)) % W(20));
    E = (A & W(0xc6)) | (!gb(buffer1, W(105)) & W(0xc6)) | (A & (!gb(buffer1, W(105))));
    buffer2[1] = (W(buffer2[1] as u32) + ((E*E*E))).0 as u8;
    buffer0[19] = (W(buffer0[19] as u32) ^ (((W(224) | (gb(buffer4, gb(buffer1, W(92)) % W(21)) & W(27))) * gb(buffer2, gb(buffer1, W(41)) % W(35))) / W(3))).0 as u8;
    buffer1[140] = (W(buffer1[140] as u32) + (weird_ror8(W(92), gb(buffer1, W(5)) & W(7)))).0 as u8;
    buffer2[12] = (W(buffer2[12] as u32) + (((((!gb(buffer1, W(4))) ^ gb(buffer2, gb(buffer1, W(12)) % W(35))) | gb(buffer1, W(182))) & W(192)) | (((!gb(buffer1, W(4))) ^ gb(buffer2, gb(buffer1, W(12)) % W(35))) & gb(buffer1, W(182))))).0 as u8;
    buffer1[36] = (W(buffer1[36] as u32) + (W(125))).0 as u8;
    buffer1[124] = (w_rol8x((((W(74) & gb(buffer1, W(138))) | ((W(74) | gb(buffer1, W(138))) & gb(buffer0, W(15)))) & gb(buffer0, gb(buffer1, W(43)) % W(20))) | (((W(74) & gb(buffer1, W(138))) | ((W(74) | gb(buffer1, W(138))) & gb(buffer0, W(15))) | gb(buffer0, gb(buffer1, W(43)) % W(20))) & W(95)), W(4))).0 as u8;
    buffer3[8] = (((((gb(buffer0, gb(buffer3, W(4)) % W(20)) & W(95))) & ((gb(buffer4, gb(buffer1, W(68)) % W(21)) & W(46)) << W(1))) | W(16)) ^ W(92)).0 as u8;
    A = gb(buffer1, W(177)) + gb(buffer4, gb(buffer1, W(79)) % W(21));
    D = (((A >> W(1)) | ((W(3) * gb(buffer1, W(148))) / W(5))) & gb(buffer2, W(1))) | ((A >> W(1)) & ((W(3) * gb(buffer1, W(148)))/W(5)));
    buffer3[12] = (((-W(34) - D))).0 as u8;
    A = W(8) - ((gb(buffer2, W(22)) & W(7)));
    B = (gb(buffer1, W(33)) >> (A & W(7)));
    C = gb(buffer1, W(33)) << (gb(buffer2, W(22)) & W(7));
    buffer2[16] = (W(buffer2[16] as u32) + (((gb(buffer2, gb(buffer3, W(0)) % W(35)) & W(159)) | gb(buffer0, gb(buffer3, W(4)) % W(20)) | W(8)) - ((B^C) | W(128)))).0 as u8;
    buffer0[14] = (W(buffer0[14] as u32) ^ (gb(buffer2, gb(buffer3, W(12)) % W(35)))).0 as u8;
    A = weird_rol8(gb(buffer4, gb(buffer0, gb(buffer1, W(201)) % W(20)) % W(21)), ((gb(buffer2, gb(buffer1, W(112)) % W(35)) << W(1)) & W(7)));
    D = (gb(buffer0, gb(buffer1, W(208)) % W(20)) & W(131)) | (gb(buffer0, gb(buffer1, W(164)) % W(20)) & W(124));
    buffer1[19] = (W(buffer1[19] as u32) + ((A & (D/W(5))) | ((A | (D/W(5))) & W(37)))).0 as u8;
    buffer2[8] = (weird_ror8(W(140), ((gb(buffer4, gb(buffer1, W(45)) % W(21)) + W(92)) * (gb(buffer4, gb(buffer1, W(45)) % W(21)) + W(92))) & W(7))).0 as u8;
    buffer1[190] = (W(56)).0 as u8;
    buffer2[8] = (W(buffer2[8] as u32) ^ (gb(buffer3, W(0)))).0 as u8;
    buffer1[53] = (!((gb(buffer0, gb(buffer1, W(83)) % W(20)) | W(204))/W(5))).0 as u8;
    buffer0[13] = (W(buffer0[13] as u32) + (gb(buffer0, gb(buffer1, W(41)) % W(20)))).0 as u8;
    buffer0[10] = (((gb(buffer2, gb(buffer3, W(0)) % W(35)) & gb(buffer1, W(2))) | ((gb(buffer2, gb(buffer3, W(0)) % W(35)) | gb(buffer1, W(2))) & gb(buffer3, W(12)))) / W(15)).0 as u8;
    A = (((W(56) | (gb(buffer4, gb(buffer1, W(2)) % W(21)) & W(68))) | gb(buffer2, gb(buffer3, W(8)) % W(35))) & W(42)) | (((gb(buffer4, gb(buffer1, W(2)) % W(21)) & W(68)) | W(56)) & gb(buffer2, gb(buffer3, W(8)) % W(35)));
    buffer3[16] = ((A*A) + W(110)).0 as u8;
    buffer3[20] = (W(202) - gb(buffer3, W(16))).0 as u8;
    buffer3[24] = (gb(buffer1, W(151))).0 as u8;
    buffer2[13] = (W(buffer2[13] as u32) ^ (gb(buffer4, gb(buffer3, W(0)) % W(21)))).0 as u8;
    B = ((gb(buffer2, gb(buffer1, W(179)) % W(35)) - W(38)) & W(177)) | (gb(buffer3, W(12)) & W(177));
    C = ((gb(buffer2, gb(buffer1, W(179)) % W(35)) - W(38))) & gb(buffer3, W(12));
    buffer3[28] = (W(30) + ((B | C) * (B | C))).0 as u8;
    buffer3[32] = (gb(buffer3, W(28)) + W(62)).0 as u8;
    A = ((gb(buffer3, W(20)) + (gb(buffer3, W(0)) & W(74))) | !gb(buffer4, gb(buffer3, W(0)) % W(21))) & W(121);
    B = ((gb(buffer3, W(20)) + (gb(buffer3, W(0)) & W(74))) & !gb(buffer4, gb(buffer3, W(0)) % W(21)));
    tmp3 = (A|B);
    C = ((((A|B) ^ W(0xffffffa6)) | gb(buffer3, W(0))) & W(4)) | (((A|B) ^ W(0xffffffa6)) & gb(buffer3, W(0)));
    buffer1[47] = ((gb(buffer2, gb(buffer1, W(89)) % W(35)) + C) ^ gb(buffer1, W(47))).0 as u8;
    buffer3[36] = (((w_rol8((tmp & W(179)) + W(68), W(2)) & gb(buffer0, W(3))) | (tmp2 & !gb(buffer0, W(3)))) - W(15)).0 as u8;
    buffer1[123] = (W(buffer1[123] as u32) ^ (W(221))).0 as u8;
    A = ((gb(buffer4, gb(buffer3, W(0)) % W(21))) / W(3)) - gb(buffer2, gb(buffer3, W(4)) % W(35));
    C = (((gb(buffer3, W(0)) & W(163)) + W(92)) & W(246)) | (gb(buffer3, W(0)) & W(92));
    E = ((C | gb(buffer3, W(24))) & W(54)) | (C & gb(buffer3, W(24)));
    buffer3[40] = (A - E).0 as u8;
    buffer3[44] = (tmp3 ^ W(81) ^ (((gb(buffer3, W(0)) >> W(1)) & W(101)) + W(26))).0 as u8;
    buffer3[48] = (gb(buffer2, gb(buffer3, W(4)) % W(35)) & W(27)).0 as u8;
    buffer3[52] = (W(27)).0 as u8;
    buffer3[56] = (W(199)).0 as u8;
    buffer3[64] = (gb(buffer3, W(4)) + (((((((gb(buffer3, W(40)) | gb(buffer3, W(24))) & W(177)) | (gb(buffer3, W(40)) & gb(buffer3, W(24)))) & ((((gb(buffer4, gb(buffer3, W(0)) % W(20)) & W(177)) | W(176))) | ((gb(buffer4, gb(buffer3, W(0)) % W(21))) & !W(3)))) | ((((gb(buffer3, W(40)) & gb(buffer3, W(24))) | ((gb(buffer3, W(40)) | gb(buffer3, W(24))) & W(177))) & W(199)) | ((((gb(buffer4, gb(buffer3, W(0)) % W(21)) & W(1)) + W(176)) | (gb(buffer4, gb(buffer3, W(0)) % W(21)) & !W(3))) & gb(buffer3, W(56))))) & (!gb(buffer3, W(52)))) | gb(buffer3, W(48)))).0 as u8;
    buffer2[33] = (W(buffer2[33] as u32) ^ (gb(buffer1, W(26)))).0 as u8;
    buffer1[106] = (W(buffer1[106] as u32) ^ (gb(buffer3, W(20)) ^ W(133))).0 as u8;
    buffer2[30] = (((gb(buffer3, W(64)) / W(3)) - (W(275) | (gb(buffer3, W(0)) & W(247)))) ^ gb(buffer0, gb(buffer1, W(122)) % W(20))).0 as u8;
    buffer1[22] = ((gb(buffer2, gb(buffer1, W(90)) % W(35)) & W(95)) | W(68)).0 as u8;
    A = (gb(buffer4, gb(buffer3, W(36)) % W(21)) & W(184)) | (gb(buffer2, gb(buffer3, W(44)) % W(35)) & !W(184));
    buffer2[18] = (W(buffer2[18] as u32) + (((A*A*A) >> W(1)))).0 as u8;
    buffer2[5] = (W(buffer2[5] as u32) - (gb(buffer4, gb(buffer1, W(92)) % W(21)))).0 as u8;
    A = (((gb(buffer1, W(41)) & !W(24))|(gb(buffer2, gb(buffer1, W(183)) % W(35)) & W(24))) & (gb(buffer3, W(16)) + W(53))) | (gb(buffer3, W(20)) & gb(buffer2, gb(buffer3, W(20)) % W(35)));
    B = (gb(buffer1, W(17)) & (!gb(buffer3, W(44)))) | (gb(buffer0, gb(buffer1, W(59)) % W(20)) & gb(buffer3, W(44)));
    buffer2[18] = (W(buffer2[18] as u32) ^ ((A*B))).0 as u8;
    A = weird_ror8(gb(buffer1, W(11)), gb(buffer2, gb(buffer1, W(28)) % W(35)) & W(7)) & W(7);
    B = (((gb(buffer0, gb(buffer1, W(93)) % W(20)) & !gb(buffer0, W(14))) | (gb(buffer0, W(14)) & W(150))) & !W(28)) | (gb(buffer1, W(7)) & W(28));
    buffer2[22] = ((((((B | weird_rol8(gb(buffer2, gb(buffer3, W(0)) % W(35)), A)) & gb(buffer2, W(33))) | (B & weird_rol8(gb(buffer2, gb(buffer3, W(0)) % W(35)), A))) + W(74)) & W(0xff))).0 as u8;
    A = gb(buffer4, (gb(buffer0, gb(buffer1, W(39)) % W(20)) ^ W(217)) % W(21));
    buffer0[15] = (W(buffer0[15] as u32) - (((((gb(buffer3, W(20)) | gb(buffer3, W(0))) & W(214)) | (gb(buffer3, W(20)) & gb(buffer3, W(0)))) & A) | ((((gb(buffer3, W(20)) | gb(buffer3, W(0))) & W(214)) | (gb(buffer3, W(20)) & gb(buffer3, W(0))) | A) & gb(buffer3, W(32))))).0 as u8;
    B = (((gb(buffer2, gb(buffer1, W(57)) % W(35)) & gb(buffer0, gb(buffer3, W(64)) % W(20))) | ((gb(buffer0, gb(buffer3, W(64)) % W(20)) | gb(buffer2, gb(buffer1, W(57)) % W(35))) & W(95)) | (gb(buffer3, W(64)) & W(45)) | W(82)) & W(32));
    C = ((gb(buffer2, gb(buffer1, W(57)) % W(35)) & gb(buffer0, gb(buffer3, W(64)) % W(20))) | ((gb(buffer2, gb(buffer1, W(57)) % W(35)) | gb(buffer0, gb(buffer3, W(64)) % W(20))) & W(95))) & ((gb(buffer3, W(64)) & W(45)) | W(82));
    D = ((((gb(buffer3, W(0))/W(3)) - (gb(buffer3, W(64))|gb(buffer1, W(22))))) ^ (gb(buffer3, W(28)) + W(62)) ^ ((B|C)));
    T = gb(buffer0, (D & W(0xff)) % W(20));
    buffer3[68] = ((gb(buffer0, gb(buffer1, W(99)) % W(20)) * gb(buffer0, gb(buffer1, W(99)) % W(20)) * gb(buffer0, gb(buffer1, W(99)) % W(20)) * gb(buffer0, gb(buffer1, W(99)) % W(20))) | gb(buffer2, gb(buffer3, W(64)) % W(35))).0 as u8;
    U = gb(buffer0, gb(buffer1, W(50)) % W(20));
    W_var = gb(buffer2, gb(buffer1, W(138)) % W(35));
    X = gb(buffer4, gb(buffer1, W(39)) % W(21));
    Y = gb(buffer0, gb(buffer1, W(4)) % W(20));
    Z = gb(buffer4, gb(buffer1, W(202)) % W(21));
    V = gb(buffer0, gb(buffer1, W(151)) % W(20));
    S = gb(buffer2, gb(buffer1, W(14)) % W(35));
    R = gb(buffer0, gb(buffer1, W(145)) % W(20));
    A = (gb(buffer2, gb(buffer3, W(68)) % W(35)) & gb(buffer0, gb(buffer1, W(209)) % W(20))) | ((gb(buffer2, gb(buffer3, W(68)) % W(35)) | gb(buffer0, gb(buffer1, W(209)) % W(20))) & W(24));
    B = weird_rol8(gb(buffer4, gb(buffer1, W(127)) % W(21)), gb(buffer2, gb(buffer3, W(68)) % W(35)) & W(7));
    C = (A & gb(buffer0, W(10))) | (B & !gb(buffer0, W(10)));
    D = W(7) ^ (gb(buffer4, gb(buffer2, gb(buffer3, W(36)) % W(35)) % W(21)) << W(1));
    buffer3[72] = ((C & W(71)) | (D & !W(71))).0 as u8;
    buffer2[2] = (W(buffer2[2] as u32) + ((((gb(buffer0, gb(buffer3, W(20)) % W(20)) << W(1)) & W(159)) | (gb(buffer4, gb(buffer1, W(190)) % W(21)) & !W(159))) & ((((gb(buffer4, gb(buffer3, W(64)) % W(21)) & W(110)) | (gb(buffer0, gb(buffer1, W(25)) % W(20)) & !W(110))) & !W(150)) | (gb(buffer1, W(25)) & W(150))))).0 as u8;
    buffer2[14] = (W(buffer2[14] as u32) - (((gb(buffer2, gb(buffer3, W(20)) % W(35)) & (gb(buffer3, W(72)) ^ gb(buffer2, gb(buffer1, W(100)) % W(35)))) & !W(34)) | (gb(buffer1, W(97)) & W(34)))).0 as u8;
    buffer0[17] = (W(115)).0 as u8;
    buffer1[23] = (W(buffer1[23] as u32) ^ (((((((gb(buffer4, gb(buffer1, W(17)) % W(21)) | gb(buffer0, gb(buffer3, W(20)) % W(20))) & gb(buffer3, W(72))) | (gb(buffer4, gb(buffer1, W(17)) % W(21)) & gb(buffer0, gb(buffer3, W(20)) % W(20)))) & (gb(buffer1, W(50))/W(3))) | ((((gb(buffer4, gb(buffer1, W(17)) % W(21)) | gb(buffer0, gb(buffer3, W(20)) % W(20))) & gb(buffer3, W(72))) | (gb(buffer4, gb(buffer1, W(17)) % W(21)) & gb(buffer0, gb(buffer3, W(20)) % W(20))) | (gb(buffer1, W(50)) / W(3))) & W(246))) << W(1)))).0 as u8;
    buffer0[13] = (((((((gb(buffer0, gb(buffer3, W(40)) % W(20)) | gb(buffer1, W(10))) & W(82)) | (gb(buffer0, gb(buffer3, W(40)) % W(20)) & gb(buffer1, W(10)))) & W(209)) | ((gb(buffer0, gb(buffer1, W(39)) % W(20)) << W(1)) & W(46))) >> W(1))).0 as u8;
    buffer2[33] = (W(buffer2[33] as u32) - (gb(buffer1, W(113)) & W(9))).0 as u8;
    buffer2[28] = (W(buffer2[28] as u32) - (((((W(2) | (gb(buffer1, W(110)) & W(222))) >> W(1)) & !W(223)) | (gb(buffer3, W(20)) & W(223))))).0 as u8;
    J = weird_rol8((V | Z), (U & W(7)));
    A = (gb(buffer2, W(16)) & T) | (W_var & (!gb(buffer2, W(16))));
    B = (gb(buffer1, W(33)) & W(17)) | (X & !W(17));
    E = ((Y | ((A+B) / W(5))) & W(147)) | (Y & ((A+B) / W(5)));
    M = (gb(buffer3, W(40)) & gb(buffer4, ((gb(buffer3, W(8)) + J + E) & W(0xff)) % W(21))) | ((gb(buffer3, W(40)) | gb(buffer4, ((gb(buffer3, W(8)) + J + E) & W(0xff)) % W(21))) & gb(buffer2, W(23)));
    buffer0[15] = ((((gb(buffer4, gb(buffer3, W(20)) % W(21)) - W(48)) & (!gb(buffer1, W(184)))) | ((gb(buffer4, gb(buffer3, W(20)) % W(21)) - W(48)) & W(189)) | (W(189) & !gb(buffer1, W(184)))) & (M*M*M)).0 as u8;
    buffer2[22] = (W(buffer2[22] as u32) + (gb(buffer1, W(183)))).0 as u8;
    buffer3[76] = ((W(3) * gb(buffer4, gb(buffer1, W(1)) % W(21))) ^ gb(buffer3, W(0))).0 as u8;
    A = gb(buffer2, ((gb(buffer3, W(8)) + (J + E)) & W(0xff)) % W(35));
    F = (((gb(buffer4, gb(buffer1, W(178)) % W(21)) & A) | ((gb(buffer4, gb(buffer1, W(178)) % W(21)) | A) & W(209))) * gb(buffer0, gb(buffer1, W(13)) % W(20))) * (gb(buffer4, gb(buffer1, W(26)) % W(21)) >> W(1));
    G = (F + W(0x733ffff9)) * W(198) - (((F + W(0x733ffff9)) * W(396) + W(212)) & W(212)) + W(85);
    buffer3[80] = (gb(buffer3, W(36)) + (G ^ W(148)) + ((G ^ W(107)) << W(1)) - W(127)).0 as u8;
    buffer3[84] = (((gb(buffer2, gb(buffer3, W(64)) % W(35))) & W(245)) | (gb(buffer2, gb(buffer3, W(20)) % W(35)) & W(10))).0 as u8;
    A = gb(buffer0, gb(buffer3, W(68)) % W(20)) | W(81);
    buffer2[18] = (W(buffer2[18] as u32) - (((A*A*A) & !gb(buffer0, W(15))) | ((gb(buffer3, W(80)) / W(15)) & gb(buffer0, W(15))))).0 as u8;
    buffer3[88] = (gb(buffer3, W(8)) + J + E - gb(buffer0, gb(buffer1, W(160)) % W(20)) + (gb(buffer4, gb(buffer0, ((gb(buffer3, W(8)) + J + E) & W(255)) % W(20)) % W(21)) / W(3))).0 as u8;
    B = ((R ^ gb(buffer3, W(72))) & !W(198)) | ((S * S) & W(198));
    F = (gb(buffer4, gb(buffer1, W(69)) % W(21)) & gb(buffer1, W(172))) | ((gb(buffer4, gb(buffer1, W(69)) % W(21)) | gb(buffer1, W(172)) ) & ((gb(buffer3, W(12)) - B) + W(77)));
    buffer0[16] = (W(147) - ((gb(buffer3, W(72)) & ((F & W(251)) | W(1))) | (((F & W(250)) | gb(buffer3, W(72))) & W(198)))).0 as u8;
    C = (gb(buffer4, gb(buffer1, W(168)) % W(21)) & gb(buffer0, gb(buffer1, W(29)) % W(20)) & W(7)) | ((gb(buffer4, gb(buffer1, W(168)) % W(21)) | gb(buffer0, gb(buffer1, W(29)) % W(20))) & W(6));
    F = (gb(buffer4, gb(buffer1, W(155)) % W(21)) & gb(buffer1, W(105))) | ((gb(buffer4, gb(buffer1, W(155)) % W(21)) | gb(buffer1, W(105))) & W(141));
    buffer0[3] = (W(buffer0[3] as u32) - (gb(buffer4, weird_rol32(F, C) % W(21)))).0 as u8;
    buffer1[5] = (weird_ror8(gb(buffer0, W(12)), ((gb(buffer0, gb(buffer1, W(61)) % W(20)) / W(5)) & W(7))) ^ (((!gb(buffer2, gb(buffer3, W(84)) % W(35))) & W(0xffffffff)) / W(5))).0 as u8;
    buffer1[198] = (W(buffer1[198] as u32) + (gb(buffer1, W(3)))).0 as u8;
    A = (W(162) | gb(buffer2, gb(buffer3, W(64)) % W(35)));
    buffer1[164] = (W(buffer1[164] as u32) + (((A*A)/W(5)))).0 as u8;
    G = weird_ror8(W(139), (gb(buffer3, W(80)) & W(7)));
    C = ((gb(buffer4, gb(buffer3, W(64)) % W(21)) * gb(buffer4, gb(buffer3, W(64)) % W(21)) * gb(buffer4, gb(buffer3, W(64)) % W(21))) & W(95)) | (gb(buffer0, gb(buffer3, W(40)) % W(20)) & !W(95));
    buffer3[92] = ((G & W(12)) | (gb(buffer0, gb(buffer3, W(20)) % W(20)) & W(12)) | (G & gb(buffer0, gb(buffer3, W(20)) % W(20))) | C).0 as u8;
    buffer2[12] = (W(buffer2[12] as u32) + (((gb(buffer1, W(103)) & W(32)) | (gb(buffer3, W(92)) & ((gb(buffer1, W(103)) | W(60)))) | W(16))/W(3))).0 as u8;
    buffer3[96] = (gb(buffer1, W(143))).0 as u8;
    buffer3[100] = (W(27)).0 as u8;
    buffer3[104] = ((((gb(buffer3, W(40)) & !gb(buffer2, W(8))) | (gb(buffer1, W(35)) & gb(buffer2, W(8)))) & gb(buffer3, W(64))) ^ W(119)).0 as u8;
    buffer3[108] = (W(238) & ((((gb(buffer3, W(40)) & !gb(buffer2, W(8))) | (gb(buffer1, W(35)) & gb(buffer2, W(8)))) & gb(buffer3, W(64))) << W(1))).0 as u8;
    buffer3[112] = ((!gb(buffer3, W(64)) & (gb(buffer3, W(84)) / W(3))) ^ W(49)).0 as u8;
    buffer3[116] = (W(98) & ((!gb(buffer3, W(64)) & (gb(buffer3, W(84)) / W(3))) << W(1))).0 as u8;
    A = (gb(buffer1, W(35)) & gb(buffer2, W(8))) | (gb(buffer3, W(40)) & !gb(buffer2, W(8)));
    B = (A & gb(buffer3, W(64))) | (((gb(buffer3, W(84)) / W(3)) & !gb(buffer3, W(64))));
    buffer1[143] = (gb(buffer3, W(96)) - ((B & (W(86) + ((gb(buffer1, W(172)) & W(64)) >> W(1)))) | (((((gb(buffer1, W(172)) & W(65)) >> W(1)) ^ W(86)) | ((!gb(buffer3, W(64)) & (gb(buffer3, W(84)) / W(3))) | (((gb(buffer3, W(40)) & !gb(buffer2, W(8))) | (gb(buffer1, W(35)) & gb(buffer2, W(8)))) & gb(buffer3, W(64))))) & gb(buffer3, W(100))))).0 as u8;
    buffer2[29] = (W(162)).0 as u8;
    A = ((((gb(buffer4, gb(buffer3, W(88)) % W(21))) & W(160)) | (gb(buffer0, gb(buffer1, W(125)) % W(20)) & W(95))) >> W(1));
    B = gb(buffer2, gb(buffer1, W(149)) % W(35)) ^ (gb(buffer1, W(43)) * gb(buffer1, W(43)));
    buffer0[15] = (W(buffer0[15] as u32) + ((B&A) | ((A|B) & W(115)))).0 as u8;
    buffer3[120] = (gb(buffer3, W(64)) - gb(buffer0, gb(buffer3, W(40)) % W(20))).0 as u8;
    buffer1[95] = (gb(buffer4, gb(buffer3, W(20)) % W(21))).0 as u8;
    A = weird_ror8(gb(buffer2, gb(buffer3, W(80)) % W(35)), (gb(buffer2, gb(buffer1, W(17)) % W(35)) * gb(buffer2, gb(buffer1, W(17)) % W(35)) * gb(buffer2, gb(buffer1, W(17)) % W(35))) & W(7));
    buffer0[7] = (W(buffer0[7] as u32) - ((A*A))).0 as u8;
    buffer2[8] = (gb(buffer2, W(8)) - gb(buffer1, W(184)) + (gb(buffer4, gb(buffer1, W(202)) % W(21)) * gb(buffer4, gb(buffer1, W(202)) % W(21)) * gb(buffer4, gb(buffer1, W(202)) % W(21)))).0 as u8;
    buffer0[16] = ((gb(buffer2, gb(buffer1, W(102)) % W(35)) << W(1)) & W(132)).0 as u8;
    buffer3[124] = ((gb(buffer4, gb(buffer3, W(40)) % W(21)) >> W(1)) ^ gb(buffer3, W(68))).0 as u8;
    buffer0[7] = (W(buffer0[7] as u32) - ((gb(buffer0, gb(buffer1, W(191)) % W(20)) - (((gb(buffer4, gb(buffer1, W(80)) % W(21)) << W(1)) & !W(177)) | (gb(buffer4, gb(buffer4, gb(buffer3, W(88)) % W(21)) % W(21)) & W(177)))))).0 as u8;
    buffer0[6] = (gb(buffer0, gb(buffer1, W(119)) % W(20))).0 as u8;
    A = (gb(buffer4, gb(buffer1, W(190)) % W(21)) & !W(209)) | (gb(buffer1, W(118)) & W(209));
    B = gb(buffer0, gb(buffer3, W(120)) % W(20)) * gb(buffer0, gb(buffer3, W(120)) % W(20));
    buffer0[12] = ((gb(buffer0, gb(buffer3, W(84)) % W(20)) ^ (gb(buffer2, gb(buffer1, W(71)) % W(35)) + gb(buffer2, gb(buffer1, W(15)) % W(35)))) & ((A & B) | ((A | B) & W(27)))).0 as u8;
    B = (gb(buffer1, W(32)) & gb(buffer2, gb(buffer3, W(88)) % W(35))) | ((gb(buffer1, W(32)) | gb(buffer2, gb(buffer3, W(88)) % W(35))) & W(23));
    D = (((gb(buffer4, gb(buffer1, W(57)) % W(21)) * W(231)) & W(169)) | (B & W(86)));
    F = (((gb(buffer0, gb(buffer1, W(82)) % W(20)) & !W(29)) | (gb(buffer4, gb(buffer3, W(124)) % W(21)) & W(29))) & W(190)) | (gb(buffer4, (D/W(5)) % W(21)) & !W(190));
    H = gb(buffer0, gb(buffer3, W(40)) % W(20)) * gb(buffer0, gb(buffer3, W(40)) % W(20)) * gb(buffer0, gb(buffer3, W(40)) % W(20));
    K = (H & gb(buffer1, W(82))) | (H & W(92)) | (gb(buffer1, W(82)) & W(92));
    buffer3[128] = (((F & K) | ((F | K) & W(192))) ^ (D/W(5))).0 as u8;
    buffer2[25] = (W(buffer2[25] as u32) ^ (((gb(buffer0, gb(buffer3, W(120)) % W(20)) << W(1)) * gb(buffer1, W(5))) - (weird_rol8(gb(buffer3, W(76)), (gb(buffer4, gb(buffer3, W(124)) % W(21)) & W(7))) & (gb(buffer3, W(20)) + W(110))))).0 as u8;
}
