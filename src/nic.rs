// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MemoryCellData {
    pub created: u64,
    pub identifier: u64,
}

impl PartialOrd for MemoryCellData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.created.partial_cmp(&other.created)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MemoryCell {
    /// The memory cell is empty.
    Empty,
    /// The memory cell contains half of a valid EPR.
    /// It can be overwritten.
    Valid(MemoryCellData),
    /// The memory cell is currently in use and cannot be overwritten.
    Used(MemoryCellData),
}

impl MemoryCell {
    /// Return a new valid memory cell.
    pub fn new(created: u64, identifier: u64) -> Self {
        MemoryCell::Valid(MemoryCellData {
            created,
            identifier,
        })
    }

    /// Return the memory cell data, unless it is empty.
    /// Make the cell empty after the call.
    pub fn take_data(&mut self) -> Option<MemoryCellData> {
        let old_value = std::mem::take(self);
        match old_value {
            MemoryCell::Empty => None,
            MemoryCell::Valid(data) | MemoryCell::Used(data) => Some(data),
        }
    }

    /// Return the memory cell data, unless it is empty.
    pub fn data(&mut self) -> Option<MemoryCellData> {
        match self {
            MemoryCell::Empty => None,
            MemoryCell::Valid(data) | MemoryCell::Used(data) => Some(data.clone()),
        }
    }

    /// Return true if the cell is empty.
    pub fn is_empty(&self) -> bool {
        matches!(self, MemoryCell::Empty)
    }

    /// Return true if the cell is valid.
    pub fn is_valid(&self) -> bool {
        matches!(self, MemoryCell::Valid(_data))
    }

    /// Return true if the cell is used.
    pub fn is_used(&self) -> bool {
        matches!(self, MemoryCell::Used(_data))
    }

    /// Mark the cell as used if it was valid, otherwise do nothing.
    pub fn used(&mut self) {
        match self {
            MemoryCell::Valid(data) => *self = MemoryCell::Used(data.clone()),
            _ => {}
        }
    }
}

impl Default for MemoryCell {
    /// Return an empty memory cell.
    fn default() -> Self {
        MemoryCell::Empty
    }
}

impl PartialOrd for MemoryCell {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self {
            MemoryCell::Empty => match other {
                MemoryCell::Empty => Some(std::cmp::Ordering::Equal),
                MemoryCell::Valid(_data) | MemoryCell::Used(_data) => {
                    Some(std::cmp::Ordering::Less)
                }
            },
            MemoryCell::Valid(self_data) | MemoryCell::Used(self_data) => match other {
                MemoryCell::Empty => Some(std::cmp::Ordering::Greater),
                MemoryCell::Valid(other_data) | MemoryCell::Used(other_data) => {
                    self_data.partial_cmp(other_data)
                }
            },
        }
    }
}

impl Ord for MemoryCell {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
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
    /// Never overwrites currently in-use memory cells.
    /// Do nothing if all the memory cells are current in-use and return false.
    pub fn add_epr_pair(&mut self, now: u64, epr_pair_id: u64) -> bool {
        let first_empty = self
            .memory_cells
            .iter()
            .enumerate()
            .find(|(_, cell)| cell.is_empty())
            .map(|(index, _)| index);

        if let Some(index) = first_empty {
            self.memory_cells[index] = MemoryCell::new(now, epr_pair_id);
            return true;
        }

        if let Some(index) = self.oldest_valid() {
            self.memory_cells[index] = MemoryCell::new(now, epr_pair_id);
            return true;
        }

        return false;
    }

    /// Consume an EPR pair. Return None if the index is invalid or the memory
    /// cell is empty, otherwise return the memory cell data.
    pub fn consume(&mut self, index: usize) -> Option<MemoryCellData> {
        if index >= self.memory_cells.len() {
            return None;
        }

        self.memory_cells[index].take_data()
    }

    /// Return the occupancy of the NIC, i.e., the number of non-empty memory
    /// cells divided by the total number of cells.
    pub fn occupancy(&mut self) -> f64 {
        if self.memory_cells.len() == 0 {
            0.0
        } else {
            self.memory_cells
                .iter()
                .map(|cell| !cell.is_empty() as u32)
                .sum::<u32>() as f64
                / self.memory_cells.len() as f64
        }
    }

    /// Flag a memory cell as used. Return the memory cell data if successful,
    /// otherwise (the cell does not exist or was not valid) None.
    pub fn used(&mut self, index: usize) -> Option<MemoryCellData> {
        if index >= self.memory_cells.len() {
            return None;
        }

        let cell = &mut self.memory_cells[index];
        cell.used();
        if cell.is_used() {
            cell.data()
        } else {
            None
        }
    }

    /// Return the index of the oldest valid memory cell, if any.
    pub fn oldest_valid(&self) -> Option<usize> {
        self.memory_cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| cell.is_valid())
            .min_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(index, _)| index)
    }

    /// Return the index of the newest valid memory cell, if any.
    pub fn newest_valid(&self) -> Option<usize> {
        self.memory_cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| cell.is_valid())
            .min_by(|(_, a), (_, b)| b.cmp(a))
            .map(|(index, _)| index)
    }
}

#[cfg(test)]
mod tests {
    use crate::nic::MemoryCell;

    use super::{Nic, Role};

    #[test]
    fn test_nic_add_consume_epr_pairs() {
        let mut nic = Nic::new(Role::Master, 10);

        for cell in &nic.memory_cells {
            assert!(cell.is_empty());
        }

        assert_float_eq::assert_f64_near!(0.0, nic.occupancy());

        assert!(nic.oldest_valid().is_none());
        assert!(nic.newest_valid().is_none());

        for i in 0..10 {
            nic.add_epr_pair(i + 100, i);
            assert_eq!(0, nic.oldest_valid().unwrap());
            assert_eq!(i as usize, nic.newest_valid().unwrap());
            assert_float_eq::assert_f64_near!(0.1 * (i + 1) as f64, nic.occupancy());
        }

        assert_float_eq::assert_f64_near!(1.0, nic.occupancy());

        for cell in &nic.memory_cells {
            assert!(!cell.is_empty());
        }

        // Consume all the EPR pairs.
        assert!(nic.consume(10).is_none());
        for i in 0..10_u64 {
            let data = nic.consume(i as usize).unwrap();

            assert_eq!(i + 100, data.created);
            assert_eq!(i, data.identifier);

            assert_float_eq::assert_f64_near!(1.0 - 0.1 * (i + 1) as f64, nic.occupancy());
        }

        for cell in &nic.memory_cells {
            assert!(cell.is_empty());
        }

        // Re-add them all.
        for i in 0..10 {
            nic.add_epr_pair(i + 100, i);
        }

        // Plus a new one.
        nic.add_epr_pair(999, 42);

        for cell in &nic.memory_cells {
            match cell {
                MemoryCell::Empty => panic!("invalid empty cell"),
                MemoryCell::Valid(data) => {
                    assert!((data.created >= 101 && data.created <= 109) || data.created == 999);
                    assert!(
                        (data.identifier >= 1 && data.identifier <= 9) || data.identifier == 42
                    );
                }
                MemoryCell::Used(_data) => panic!("invalid used cell"),
            }
        }
    }

    #[test]
    fn test_nic_use_consume_epr_pairs() {
        let mut nic = Nic::new(Role::Master, 10);

        // Make sure all the cells are empty.
        for cell in &nic.memory_cells {
            assert!(cell.is_empty());
        }

        assert_float_eq::assert_f64_near!(0.0, nic.occupancy());

        // Try to use empty cells.
        for i in 0..10 {
            assert!(nic.used(i).is_none());
        }

        // Make all the cells valid.
        for i in 0..10 {
            nic.add_epr_pair(i + 100, i);
            assert_float_eq::assert_f64_near!(0.1 * (i + 1) as f64, nic.occupancy());
        }

        assert_float_eq::assert_f64_near!(1.0, nic.occupancy());

        // None of the cells are empty or used.
        for cell in &nic.memory_cells {
            assert!(cell.is_valid());
            assert!(!cell.is_empty());
            assert!(!cell.is_used());
        }

        // Try to use an invalid pair.
        assert!(nic.used(99).is_none());

        // Use all the EPR pairs.
        for i in 0..10 {
            assert!(nic.used(i).is_some());
        }
        assert!(nic.oldest_valid().is_none());
        assert!(nic.newest_valid().is_none());

        // Make sure all the cells are used.
        for cell in &nic.memory_cells {
            assert!(cell.is_used());
        }

        assert_float_eq::assert_f64_near!(1.0, nic.occupancy());

        // Try to add a new pair.
        assert!(!nic.add_epr_pair(999, 999));

        // Using cells already used is OK.
        for i in 0..10 {
            assert!(nic.used(i).is_some());
        }

        // Consume one pair.
        assert!(nic.consume(7).is_some());

        // New pairs can be added, they will overwrite the only valid one.
        for i in 0..100 {
            assert!(nic.add_epr_pair(1000 + i, 2000 + i));
        }

        // Consume all the EPR pairs.
        assert!(nic.consume(10).is_none());
        for i in 0..10_u64 {
            let data = nic.consume(i as usize).unwrap();

            if i == 7 {
                assert_eq!(1099, data.created);
                assert_eq!(2099, data.identifier);
            } else {
                assert_eq!(i + 100, data.created);
                assert_eq!(i, data.identifier);
            }

            assert_float_eq::assert_f64_near!(1.0 - 0.1 * (i + 1) as f64, nic.occupancy());
        }

        for cell in &nic.memory_cells {
            assert!(cell.is_empty());
        }
    }

    #[test]
    fn test_nic_memory_cell_order() {
        assert!(MemoryCell::Empty == MemoryCell::Empty);
        assert!(MemoryCell::Empty <= MemoryCell::Empty);
        assert!(!(MemoryCell::Empty < MemoryCell::Empty));

        assert!(!(MemoryCell::Empty == MemoryCell::new(100, 0)));
        assert!(MemoryCell::Empty <= MemoryCell::new(100, 0));
        assert!(MemoryCell::Empty < MemoryCell::new(100, 0));

        assert!(!(MemoryCell::new(100, 0) == MemoryCell::Empty));
        assert!(!(MemoryCell::new(100, 0) <= MemoryCell::Empty));
        assert!(!(MemoryCell::new(100, 0) < MemoryCell::Empty));

        assert!(MemoryCell::new(100, 0) == MemoryCell::new(100, 0));
        assert!(MemoryCell::new(100, 0) <= MemoryCell::new(100, 0));
        assert!(!(MemoryCell::new(100, 0) < MemoryCell::new(100, 0)));

        assert!(!(MemoryCell::new(200, 0) == MemoryCell::new(100, 0)));
        assert!(!(MemoryCell::new(200, 0) <= MemoryCell::new(100, 0)));
        assert!(!(MemoryCell::new(200, 0) < MemoryCell::new(100, 0)));

        assert!(!(MemoryCell::new(100, 0) == MemoryCell::new(200, 0)));
        assert!(MemoryCell::new(100, 0) <= MemoryCell::new(200, 0));
        assert!(MemoryCell::new(100, 0) < MemoryCell::new(200, 0));
    }
}
