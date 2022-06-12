use crate::*;

// BCH codewords to test with -- these are the POCSAG Idle Codeword and Sync Codeword
const TEST_CWS: [u32; 2] = [0x7A89C197u32, 0x7CD215D8u32];

#[test]
fn test_bch_sanity() {
    // Idle Codeword and Sync Codeword have valid BCH and parity and can be error-corrected
    // Start by making sure this assumption is true
    for n in 0..TEST_CWS.len() {
        assert_eq!(
            bch_encode(TEST_CWS[n]),
            TEST_CWS[n],
            "testCW index {}: {:00x}",
            n,
            TEST_CWS[n]
        );
    }
}

#[test]
fn test_bch_single_bit_errors() {
    // Make sure all possible single-bit errors are correctable using the test codewords
    for n in 0..TEST_CWS.len() {
        let original_cw: u32 = TEST_CWS[n];

        // try all possible single bit errors
        let mut mask = 0x80000000;

        while mask > 0 {
            // add damage to the CW
            let damaged_cw = original_cw ^ mask;

            let result = bch_repair(damaged_cw);
            assert!(
                result.is_ok(),
                "origCW:{:08X}, errormask:{:08X}",
                original_cw,
                mask
            );
            let repaired_cw = result.unwrap();

            // BCH doesn't repair the parity bit. We're only concerned about repairing errors in the message or BCH parity bits.
            // (So mask off the parity in the LSB)
            assert_eq!(
                original_cw & 0xFFFFFFFE,
                repaired_cw & 0xFFFFFFFE,
                "origCW:{:08X}, errormask:{:08X}",
                original_cw,
                mask
            );

            // test the next bit
            mask >>= 1;
        }
    }
}

#[test]
fn test_bch_double_bit_errors() {
    // message buffer for unity
    //char message[100];
    // subtest count
    let mut num_tests = 0;

    // Make sure all possible double-bit errors are correctable using the test codewords
    for n in 0..TEST_CWS.len() {
        let original_cw = TEST_CWS[n];

        // try all possible single bit errors
        let mut mask1 = 0x80000000;

        while mask1 > 1
        // LSB is even parity, don't waste time on it
        {
            let mut mask2 = mask1 >> 1;
            while mask2 > 1
            // LSB is even parity, don't waste time on it
            {
                // add damage to the CW
                let damaged_cw = original_cw ^ mask1 ^ mask2;

                let result = bch_repair(damaged_cw);
                assert!(
                    result.is_ok(),
                    "origCW:{:08X}, errormask:{:08X}",
                    original_cw,
                    mask1 ^ mask2
                );
                let repaired_cw = result.unwrap();

                // BCH doesn't repair the parity bit. We're only concerned about repairing errors in the message or BCH parity bits.
                // (So mask1 off the parity in the LSB)
                assert_eq!(
                    original_cw & 0xFFFFFFFE,
                    repaired_cw & 0xFFFFFFFE,
                    "origCW:{:08X}, errormask:{:08X}",
                    original_cw,
                    mask1 ^ mask2
                );

                // increment test counter
                num_tests += 1;

                // test the next bit
                mask2 >>= 1;
            }

            mask1 >>= 1;
        }
    }

    println!("\t{} tests finished\n", num_tests);
}

// TODO: confirm that all three-bit errors are detected
