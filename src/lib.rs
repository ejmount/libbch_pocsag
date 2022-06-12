#[cfg(test)]
mod test;

pub const fn low_bits_mask(n: u32) -> u32 {
    (1u32 << n) - 1
}

const HIGHEST_BIT: u32 = u32::BITS - 1;

const PAYLOAD_BITS: u32 = 21;
const ECC_BITS: u32 = 10;
const PARITY_BITS: u32 = 1;

const ECC_MASK: u32 = low_bits_mask(ECC_BITS);
const PAYLOAD_MASK: u32 = !low_bits_mask(ECC_BITS + PARITY_BITS);

const fn bit_set(word: u32, n: u32) -> bool {
    word & (1 << n) > 0
}

pub fn bits_ms(word: u32) -> impl Iterator<Item = bool> {
    (0..32).rev().map(move |n| bit_set(word, n))
}

pub fn from_bits(bits: impl Iterator<Item = bool>) -> u32 {
    bits.fold(0, |t, n| (t << 1) | (n as u32))
}
pub fn bch_encode(cw: u32) -> u32 {
    let mut local_cw = cw & PAYLOAD_MASK; // mask off BCH parity and even parity
    let mut cw_e = local_cw;

    // Calculate BCH bits
    for _ in 0..PAYLOAD_BITS {
        if bit_set(cw_e, HIGHEST_BIT) {
            cw_e ^= 0xED_20_00_00;
        }
        cw_e <<= 1;
    }
    local_cw |= cw_e >> PAYLOAD_BITS;

    // At this point local_cw contains a codeword with BCH but no parity

    // Calculate parity bit
    let parity = local_cw.count_ones();

    // apply parity bit
    local_cw | (parity % 2)
}

// Debug options for error correction
// -- Enable debug
//#define BCH_REPAIR_DEBUG
// -- Enable printing the output of the ECC process step-by-step
//#define BCH_REPAIR_DEBUG_STEPBYSTEP

pub fn bch_repair(cw: u32) -> Result<u32, ()> {
    // calculate syndrome
    // We do this by recalculating the BCH parity bits and XORing them against the received ones

    // mask off data bits and parity, leaving the error syndrome in the LSB
    let mut syndrome = ((bch_encode(cw) ^ cw) >> 1) & ECC_MASK;

    if syndrome == 0 {
        // Syndrome of zero indicates no repair required
        return Ok(cw);
    }

    #[cfg(test)]
    println!("cw:{:08X}  syndrome:{:08X}", cw, syndrome);

    // --- Meggitt decoder ---
    // Calculate BCH bits
    let result = bits_ms(cw)
        .take((PAYLOAD_BITS + ECC_BITS) as usize)
        .map(|bit| {
            println!("    xbit:{}  synd:{:08X}", bit, syndrome);

            // produce the next corrected bit in the high bit of the result
            let output;
            if (syndrome == 0x3B4) ||		// 0x3B4: Syndrome when a single error is detected in the MSB
			(syndrome == 0x26E)	||		// 0x26E: Two adjacent errors
			(syndrome == 0x359) ||		// 0x359: Two errors, one OK bit between
			(syndrome == 0x076) ||		// 0x076: Two errors, two OK bits between
			(syndrome == 0x255) ||		// 0x255: Two errors, three OK bits between
			(syndrome == 0x0F0) ||		// 0x0F0: Two errors, four OK bits between
			(syndrome == 0x216) ||		// ... and so on
			(syndrome == 0x365) ||
			(syndrome == 0x068) ||
			(syndrome == 0x25A) ||
			(syndrome == 0x343) ||
			(syndrome == 0x07B) ||
			(syndrome == 0x1E7) ||
			(syndrome == 0x129) ||
			(syndrome == 0x14E) ||
			(syndrome == 0x2C9) ||
			(syndrome == 0x0BE) ||
			(syndrome == 0x231) ||
			(syndrome == 0x0C2) ||
			(syndrome == 0x20F) ||
			(syndrome == 0x0DD) ||
			(syndrome == 0x1B4) ||
			(syndrome == 0x2B4) ||
			(syndrome == 0x334) ||
			(syndrome == 0x3F4) ||
			(syndrome == 0x394) ||
			(syndrome == 0x3A4) ||
			(syndrome == 0x3BC) ||
			(syndrome == 0x3B0) ||
			(syndrome == 0x3B6) ||
			(syndrome == 0x3B5)
            {
                // Syndrome matches an error in the MSB
                // Correct that error and adjust the syndrome to account for it
                syndrome ^= 0x3B4;

                output = !bit;

                println!("  E"); // indicate that an error was corrected in this bit
            } else {
                // no error
                output = bit;

                println!("   \n");
            }

            // Handle Syndrome shift register feedback
            if bit_set(syndrome, ECC_BITS - PARITY_BITS) {
                syndrome <<= 1;
                syndrome ^= 0x769; // 0x769 = POCSAG generator polynomial -- x^10 + x^9 + x^8 + x^6 + x^5 + x^3 + 1
            } else {
                syndrome <<= 1;
            }
            // mask off bits which fall off the end of the syndrome shift register
            syndrome &= low_bits_mask(ECC_BITS + PARITY_BITS);

            output
            // XXX Possible optimisation: Can we exit early if the syndrome is zero? (no more errors to correct)
        });

    let result = from_bits(result) << PARITY_BITS;

    println!(
        "  orig:{:08X}  fixed:{:08X}  {}",
        cw,                                       /* original codeword */
        result,                                   /* corrected codeword sans parity bit */
        if syndrome == 0 { "OK" } else { "ERR" }  /* syndrome == 0 if error was corrected */
    );

    // Check if error correction was successful
    if syndrome != 0 {
        // Syndrome nonzero at end indicates uncorrectable errors
        println!("nonzero syndrome at end");
        return Err(());
    }

    // Syndrome is zero -- that means we must have succeeded!
    Ok(result)
}
