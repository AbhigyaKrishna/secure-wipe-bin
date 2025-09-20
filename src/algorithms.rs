use crate::args::WipeAlgorithm;

#[derive(Debug)]
pub enum WipePattern {
    Fixed(u8),
    Random,
    Gutmann(Vec<Vec<u8>>),
}

// Gutmann method patterns
pub const GUTMANN_PATTERNS: &[&[u8]] = &[
    &[0x00],
    &[0xFF],
    &[0x55],
    &[0xAA],
    &[0x92, 0x49, 0x24],
    &[0x49, 0x24, 0x92],
    &[0x24, 0x92, 0x49],
    &[0x00, 0x00, 0x00],
    &[0x11, 0x11, 0x11],
    &[0x22, 0x22, 0x22],
    &[0x33, 0x33, 0x33],
    &[0x44, 0x44, 0x44],
    &[0x55, 0x55, 0x55],
    &[0x66, 0x66, 0x66],
    &[0x77, 0x77, 0x77],
    &[0x88, 0x88, 0x88],
    &[0x99, 0x99, 0x99],
    &[0xAA, 0xAA, 0xAA],
    &[0xBB, 0xBB, 0xBB],
    &[0xCC, 0xCC, 0xCC],
    &[0xDD, 0xDD, 0xDD],
    &[0xEE, 0xEE, 0xEE],
    &[0xFF, 0xFF, 0xFF],
    &[0x92, 0x49, 0x24],
    &[0x49, 0x24, 0x92],
    &[0x24, 0x92, 0x49],
    &[0x6D, 0xB6, 0xDB],
    &[0xB6, 0xDB, 0x6D],
    &[0xDB, 0x6D, 0xB6],
];

pub fn get_algorithm_pass_count(algorithm: &WipeAlgorithm, custom_passes: usize) -> usize {
    match algorithm {
        WipeAlgorithm::Zero | WipeAlgorithm::Random => 1,
        WipeAlgorithm::Dod5220 => 3,
        WipeAlgorithm::Gutmann => 35,
        WipeAlgorithm::Custom => custom_passes,
    }
}

pub fn get_pass_pattern(algorithm: &WipeAlgorithm, pass: usize) -> WipePattern {
    match algorithm {
        WipeAlgorithm::Zero => WipePattern::Fixed(0x00),
        WipeAlgorithm::Random => WipePattern::Random,
        WipeAlgorithm::Dod5220 => match pass {
            1 => WipePattern::Fixed(0x00),
            2 => WipePattern::Fixed(0xFF),
            3 => WipePattern::Random,
            _ => unreachable!(),
        },
        WipeAlgorithm::Gutmann => {
            let patterns: Vec<Vec<u8>> = GUTMANN_PATTERNS.iter().map(|p| p.to_vec()).collect();
            WipePattern::Gutmann(patterns)
        }
        WipeAlgorithm::Custom => WipePattern::Random,
    }
}

pub fn get_pattern_name(algorithm: &WipeAlgorithm, pass: usize) -> &'static str {
    match algorithm {
        WipeAlgorithm::Zero => "0x00",
        WipeAlgorithm::Random => "RAND",
        WipeAlgorithm::Dod5220 => match pass {
            1 => "0x00",
            2 => "0xFF",
            3 => "RAND",
            _ => "????",
        },
        WipeAlgorithm::Gutmann => "GUTM",
        WipeAlgorithm::Custom => "RAND",
    }
}
