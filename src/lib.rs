#[cfg(test)]
mod test;

const HIGHEST_BIT: u32 = u32::BITS - 1;

const PAYLOAD_BITS: u32 = 21;
const ECC_BITS: u32 = 10;
const PARITY_BITS: u32 = 1;

const ECC_MASK: u32 = low_bits_mask(ECC_BITS);
const PAYLOAD_MASK: u32 = !low_bits_mask(ECC_BITS + PARITY_BITS);

const SYNDROME_ERRORS: [(u32, u32, Option<u32>); count_bit_errors()] = enumerate_syndromes();

// Create a mask to extract the N lowest bits
const fn low_bits_mask(n: u32) -> u32 {
    (1 << n) - 1
}

// Is the given bit set
const fn is_bit_set(word: u32, n: u32) -> bool {
    word & (1 << n) > 0
}

const fn high_bit(n: u32) -> u32 {
    1 << n
}

// Calculate BCH checksum code, ignoring any existing ECC bits
const fn get_bch_code(cw: u32) -> u32 {
    let mut local_cw = cw & PAYLOAD_MASK; // mask off BCH parity and even parity

    let mut count = 0; // Can't do for loops in a const fn

    // Calculate BCH bits
    while count < PAYLOAD_BITS {
        if is_bit_set(local_cw, HIGHEST_BIT) {
            local_cw ^= 0xED_20_00_00;
        }
        local_cw <<= 1;
        count += 1;
    }
    local_cw >> PAYLOAD_BITS
}

// Calculate "leftover" syndrome value from possibly-corrupt codeword
// We do this by recalculating the BCH parity bits and XORing them against the received ones
const fn calculate_syndrome(cw: u32) -> u32 {
    ((bch_encode(cw) ^ cw) >> 1) & ECC_MASK
}

// How many possible error situations there are with any combination of 1 or 2 bit errors
const fn count_bit_errors() -> usize {
    let single_bits = u32::BITS as usize;
    let double_bits = (((u32::BITS - 1) * u32::BITS) / 2) as usize;
    double_bits + single_bits
}

// Produce a list of every possible syndrome value we might encounter, along with the indices of the bits needing correction
// Single bit errors are represented by (value, None)
const fn enumerate_syndromes() -> [(u32, u32, Option<u32>); count_bit_errors()] {
    const EXAMPLE_WORLD: u32 = bch_encode(0x12340000); // Doesn't matter what this is

    let mut output = [(0, 0, None); count_bit_errors()];
    let mut items = 0;

    let mut single_bit = 0;
    while single_bit < u32::BITS {
        let error = high_bit(single_bit);
        let corrupted = EXAMPLE_WORLD ^ error;
        let syndrome = calculate_syndrome(corrupted);
        output[items] = (syndrome, single_bit, None);
        items += 1;
        single_bit += 1;
    }

    let mut first_bit = 0;
    while first_bit < u32::BITS {
        let mut second_bit = 0;
        while second_bit < first_bit {
            let error = high_bit(first_bit) | high_bit(second_bit);
            let corrupted = EXAMPLE_WORLD ^ error;
            let syndrome = calculate_syndrome(corrupted);
            output[items] = (syndrome, first_bit, Some(second_bit));
            items += 1;
            second_bit += 1;
        }
        first_bit += 1;
    }

    output
}

pub const fn bch_encode(cw: u32) -> u32 {
    // At this point local_cw contains a codeword with BCH but no parity
    let local_cw = (cw & PAYLOAD_MASK) | get_bch_code(cw);

    // Calculate parity bit
    let parity = local_cw.count_ones();

    // apply parity bit
    local_cw | (parity % 2)
}

// Attempt to repair the codeword, returning Err if recovery not possible
pub fn bch_repair(cw: u32) -> Result<u32, ()> {
    // Get "leftover" syndrome
    let syndrome = calculate_syndrome(cw);

    if syndrome == 0 {
        // Syndrome of zero indicates no repair required
        return Ok(cw);
    }

    // Lookup the syndrome
    if let Some((_, b, c)) = SYNDROME_ERRORS.iter().find(|(s, _, _)| *s == syndrome) {
        // Retrieve indicces to repair, defaulting to no-op of cw^0
        let firstbit = 1u32 << b;
        let secondbit = c.map(|n| 1u32 << n).unwrap_or(0);
        let corrected = cw ^ firstbit ^ secondbit;
        Ok(bch_encode(corrected)) // Recalculate ECC bits based on the corrected payload
    } else {
        // Did not recognise syndrome value to recover
        dbg!("nonzero syndrome at end");
        Err(())
    }
}
