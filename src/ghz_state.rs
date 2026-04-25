use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct GHZState {
    real_data: Vec<u8>,
    pointer: usize,
    n_qubits: usize,
}

impl GHZState {
    /// Initializes the GHZ state exclusively using real IBM Brisbane hardware data.
    pub fn new<P: AsRef<Path>>(data_path: P, n_qubits: usize) -> Self {
        let mut file = File::open(data_path)
            .expect("CRITICAL: Could not find brisbane_raw.bin. Real quantum data is required.");

        let mut real_data = Vec::new();
        file.read_to_end(&mut real_data)
            .expect("Failed to read quantum data");

        GHZState {
            real_data,
            pointer: 0,
            n_qubits,
        }
    }

    /// Returns the configured number of GHZ qubits.
    pub fn n_qubits(&self) -> usize {
        self.n_qubits
    }

    /// Measures bits directly from the IBM hardware payload
    pub fn measure(&mut self, bits_needed: usize) -> Vec<u8> {
        let mut output = Vec::with_capacity(bits_needed);
        let max_bits = self.real_data.len() * 8;

        for _ in 0..bits_needed {
            // Cycle the hardware entropy pool if we request more bits than available
            if self.pointer >= max_bits {
                self.pointer = 0;
            }

            // Extract the specific bit from the byte array (MSB first)
            let byte_idx = self.pointer / 8;
            let bit_idx = 7 - (self.pointer % 8);
            let bit = (self.real_data[byte_idx] >> bit_idx) & 1;

            output.push(bit);
            self.pointer += 1;
        }

        output
    }
}
