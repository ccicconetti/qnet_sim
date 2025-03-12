// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, PartialEq, Eq)]
pub enum MemoryCell {
    /// The memory cell is empty.
    Empty,
    /// The memory cell contains half of a valid EPR, with given creation time
    /// and identifier.
    Valid(u64, u64),
}

impl PartialOrd for MemoryCell {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self {
            MemoryCell::Empty => match other {
                MemoryCell::Empty => Some(std::cmp::Ordering::Equal),
                MemoryCell::Valid(_other_time, _) => Some(std::cmp::Ordering::Less),
            },
            MemoryCell::Valid(self_time, _) => match other {
                MemoryCell::Empty => Some(std::cmp::Ordering::Greater),
                MemoryCell::Valid(other_time, _) => self_time.partial_cmp(other_time),
            },
        }
    }
}

impl Ord for MemoryCell {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Debug, PartialEq, Eq)]
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

    /// Add a fresh EPR pair to an empty memory cell or, if not available,
    /// overwrite the oldest non-empty memory cell.
    pub fn add_epr_pair(&mut self, now: u64, epr_pair_id: u64) {
        let index_of_oldest = self
            .memory_cells
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(index, _)| index);

        if let Some(index) = index_of_oldest {
            self.memory_cells[index] = MemoryCell::Valid(now, epr_pair_id);
        }
    }

    /// Return the occupancy of the NIC, i.e., the number of non-empty memory
    /// cells divided by the total number of cells.
    pub fn occupancy(&mut self) -> f64 {
        if self.memory_cells.len() == 0 {
            0.0
        } else {
            self.memory_cells
                .iter()
                .map(|cell| matches!(cell, MemoryCell::Valid(_, _)) as u32)
                .sum::<u32>() as f64
                / self.memory_cells.len() as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::nic::MemoryCell;

    use super::{Nic, Role};

    #[test]
    fn test_nic_add_epr_pair() {
        let mut nic = Nic::new(Role::Master, 10);

        for cell in &nic.memory_cells {
            assert!(matches!(cell, MemoryCell::Empty));
        }

        assert_float_eq::assert_f64_near!(0.0, nic.occupancy());

        for i in 0..10 {
            nic.add_epr_pair(i + 100, i);
            assert_float_eq::assert_f64_near!(0.1 * (i + 1) as f64, nic.occupancy());
        }

        assert_float_eq::assert_f64_near!(1.0, nic.occupancy());

        for cell in &nic.memory_cells {
            assert!(matches!(cell, MemoryCell::Valid(_, _)));
        }

        nic.add_epr_pair(999, 42);

        for cell in &nic.memory_cells {
            match cell {
                MemoryCell::Empty => panic!("invalid empty cell"),
                MemoryCell::Valid(created, identifier) => {
                    assert!((*created >= 101 && *created <= 109) || *created == 999);
                    assert!((*identifier >= 1 && *identifier <= 9) || *identifier == 42);
                }
            }
        }
    }

    #[test]
    fn test_nic_memory_cell_order() {
        assert!(MemoryCell::Empty == MemoryCell::Empty);
        assert!(MemoryCell::Empty <= MemoryCell::Empty);
        assert!(!(MemoryCell::Empty < MemoryCell::Empty));

        assert!(!(MemoryCell::Empty == MemoryCell::Valid(100, 0)));
        assert!(MemoryCell::Empty <= MemoryCell::Valid(100, 0));
        assert!(MemoryCell::Empty < MemoryCell::Valid(100, 0));

        assert!(!(MemoryCell::Valid(100, 0) == MemoryCell::Empty));
        assert!(!(MemoryCell::Valid(100, 0) <= MemoryCell::Empty));
        assert!(!(MemoryCell::Valid(100, 0) < MemoryCell::Empty));

        assert!(MemoryCell::Valid(100, 0) == MemoryCell::Valid(100, 0));
        assert!(MemoryCell::Valid(100, 0) <= MemoryCell::Valid(100, 0));
        assert!(!(MemoryCell::Valid(100, 0) < MemoryCell::Valid(100, 0)));

        assert!(!(MemoryCell::Valid(200, 0) == MemoryCell::Valid(100, 0)));
        assert!(!(MemoryCell::Valid(200, 0) <= MemoryCell::Valid(100, 0)));
        assert!(!(MemoryCell::Valid(200, 0) < MemoryCell::Valid(100, 0)));

        assert!(!(MemoryCell::Valid(100, 0) == MemoryCell::Valid(200, 0)));
        assert!(MemoryCell::Valid(100, 0) <= MemoryCell::Valid(200, 0));
        assert!(MemoryCell::Valid(100, 0) < MemoryCell::Valid(200, 0));
    }
}
