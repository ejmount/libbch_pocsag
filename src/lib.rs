#[cfg(test)]
mod test;

const SYNDROME_ERRORS: &[u32] = &[
    0x3B4, 0x26E, 0x359, 0x076, 0x255, 0x0F0, 0x216, 0x365, 0x068, 0x25A, 0x343, 0x07B, 0x1E7,
    0x129, 0x14E, 0x2C9, 0x0BE, 0x231, 0x0C2, 0x20F, 0x0DD, 0x1B4, 0x2B4, 0x334, 0x3F4, 0x394,
    0x3A4, 0x3BC, 0x3B0, 0x3B6, 0x3B5,
];

const HIGHEST_BIT: u32 = u32::BITS - 1;

const PAYLOAD_BITS: u32 = 21;
const ECC_BITS: u32 = 10;
const PARITY_BITS: u32 = 1;

pub const fn low_bits_mask(n: u32) -> u32 {
    (1 << n) - 1
}

const ECC_MASK: u32 = low_bits_mask(ECC_BITS);
const PAYLOAD_MASK: u32 = !low_bits_mask(ECC_BITS + PARITY_BITS);

const fn bit_set(word: u32, n: u32) -> bool {
    word & (1 << n) > 0
}

fn bits_ms(word: u32) -> impl Iterator<Item = bool> {
    (0..u32::BITS).rev().map(move |n| bit_set(word, n))
}

fn from_bits(bits: impl Iterator<Item = bool>) -> u32 {
    bits.fold(0, |t, n| (t << 1) | (n as u32))
}

const fn get_bch_code(cw: u32) -> u32 {
    let mut local_cw = cw & PAYLOAD_MASK; // mask off BCH parity and even parity

    let mut count = 0; // Can't do for loops in a const fn

    // Calculate BCH bits
    while count < PAYLOAD_BITS {
        if bit_set(local_cw, HIGHEST_BIT) {
            local_cw ^= 0xED_20_00_00;
        }
        local_cw <<= 1;
        count += 1;
    }
    local_cw >> PAYLOAD_BITS
}

pub fn bch_encode(cw: u32) -> u32 {
    let local_cw = (cw & PAYLOAD_MASK) | get_bch_code(cw);

    // At this point local_cw contains a codeword with BCH but no parity

    // Calculate parity bit
    let parity = local_cw.count_ones();

    // apply parity bit
    local_cw | (parity % 2)
}

fn bit_decoder(syndrome: &mut u32) -> impl FnMut(bool) -> bool + '_ {
    move |bit: bool| {
        dbg!("    xbit:{}  synd:{:08X}", bit, &syndrome);

        let output = if SYNDROME_ERRORS.iter().find(|&&s| s == *syndrome).is_some() {
            // Syndrome matches an error in the MSB
            // Correct that error and adjust the syndrome to account for it
            *syndrome ^= 0x3B4;
            dbg!("  E"); // indicate that an error was corrected in this bit
            !bit
        } else {
            // no error
            dbg!("   \n");
            bit
        };

        // Handle Syndrome shift register feedback
        if bit_set(*syndrome, ECC_BITS - PARITY_BITS) {
            *syndrome <<= 1;
            *syndrome ^= 0x769; // 0x769 = POCSAG generator polynomial -- x^10 + x^9 + x^8 + x^6 + x^5 + x^3 + 1
        } else {
            *syndrome <<= 1;
        }
        // mask off bits which fall off the end of the syndrome shift register
        *syndrome &= low_bits_mask(ECC_BITS + PARITY_BITS);

        output
    }
    // XXX Possible optimisation: Can we exit early if the syndrome is zero? (no more errors to correct)
}

pub fn bch_repair(cw: u32) -> Result<u32, ()> {
    // calculate syndrome
    // We do this by recalculating the BCH parity bits and XORing them against the received ones

    // mask off data bits and parity, leaving the error syndrome in the LSB
    let mut syndrome = ((bch_encode(cw) ^ cw) >> 1) & ECC_MASK;

    if syndrome == 0 {
        // Syndrome of zero indicates no repair required
        return Ok(cw);
    }

    dbg!("cw:{:08X}  syndrome:{:08X}", cw, syndrome);

    // --- Meggitt decoder ---
    // Calculate repaired codeword
    let result_bits = bits_ms(cw)
        .take((PAYLOAD_BITS + ECC_BITS) as usize)
        .map(bit_decoder(&mut syndrome));

    let result = from_bits(result_bits) << PARITY_BITS;

    dbg!(
        "  orig:{cw:08X}  fixed:{result:08X}  {}",
        if syndrome == 0 { "OK" } else { "ERR" }
    );

    // Check if error correction was successful
    if syndrome == 0 {
        Ok(result)
    } else {
        // Syndrome nonzero at end indicates uncorrectable errors
        dbg!("nonzero syndrome at end");
        Err(())
    }
}
