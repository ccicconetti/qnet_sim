// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, PartialEq, Eq)]
pub enum MemoryCell {
    /// The memory cell is empty.
    Empty,
    /// The memory cell contains half of a valid EPR, with given identifier.
    Valid(u64),
}

#[derive(Debug)]
pub enum Role {
    Master,
    Slave,
}

/// Quantum network interface card associated with a single peer.
#[derive(Debug)]
pub struct Nic {
    /// Role of this NIC.
    role: Role,
    /// Quantum memory cells assigned to this NIC.
    memory_cells: Vec<MemoryCell>,
}

impl Nic {
    /// Create a NIC with a given role and number of quantum memory cells.
    pub fn new(role: Role, num_qubits: u32) -> Self {
        let mut memory_cells = vec![];
        for _ in 0..num_qubits {
            memory_cells.push(MemoryCell::Empty);
        }
        Self { role, memory_cells }
    }
}
