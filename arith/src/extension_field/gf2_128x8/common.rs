// abcdefgh -> aacceegg
#[inline(always)]
pub fn duplicate_even_bits(byte: u8) -> u8 {
    let even_bits = byte & 0b10101010;
    let even_bits_shifted = even_bits >> 1;
    even_bits | even_bits_shifted
}

// abcdefgh -> bbddffhh
#[inline(always)]
pub fn duplicate_odd_bits(byte: u8) -> u8 {
    let odd_bits = byte & 0b01010101;
    let odd_bits_shifted = odd_bits << 1;
    odd_bits | odd_bits_shifted
}
