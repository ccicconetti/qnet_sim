// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MemoryCellData {
    pub created: u64,
    pub local_pair_id: u64,
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
            local_pair_id: identifier,
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
    pub fn data(&self) -> Option<MemoryCellData> {
        match self {
            MemoryCell::Empty => None,
            MemoryCell::Valid(data) | MemoryCell::Used(data) => Some(data.clone()),
        }
    }

    /// Return the local pair ID, if the cell is non-empty.
    pub fn local_pair_id(&self) -> Option<u64> {
        match self {
            MemoryCell::Empty => None,
            MemoryCell::Valid(data) | MemoryCell::Used(data) => Some(data.local_pair_id),
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
        if let MemoryCell::Valid(data) = self {
            *self = MemoryCell::Used(data.clone())
        }
    }
}

impl Default for MemoryCell {
    /// Return an empty memory cell.
    fn default() -> Self {
        MemoryCell::Empty
    }
}

impl std::fmt::Display for MemoryCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MemoryCell::Empty => "E",
                MemoryCell::Valid(_memory_cell_data) => "V",
                MemoryCell::Used(_memory_cell_data) => "U",
            }
        )
    }
}

#[allow(clippy::non_canonical_partial_ord_impl)]
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

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Role::Master => "M",
                Role::Slave => "S",
            }
        )
    }
}

/// Quantum network interface card associated with a single peer.
#[derive(Debug)]
pub struct Nic {
    /// Role of this NIC.
    role: Role,
    /// Quantum memory cells assigned to this NIC.
    memory_cells: Vec<MemoryCell>,
}

impl std::fmt::Display for Nic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {}",
            self.role,
            self.memory_cells
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join("")
        )
    }
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

        false
    }

    /// Consume an EPR pair. Return None if there is no memory cell
    /// associated with the local pair requested.
    pub fn consume(&mut self, local_pair_id: u64) -> Option<MemoryCellData> {
        for memory_cell in &mut self.memory_cells {
            if let Some(data) = memory_cell.data() {
                if data.local_pair_id == local_pair_id {
                    return memory_cell.take_data();
                }
            }
        }
        None
    }

    /// Return the occupancy of the NIC, i.e., the number of non-empty memory
    /// cells divided by the total number of cells.
    pub fn occupancy(&mut self) -> f64 {
        if self.memory_cells.is_empty() {
            0.0
        } else {
            self.memory_cells
                .iter()
                .map(|cell| !cell.is_empty() as u32)
                .sum::<u32>() as f64
                / self.memory_cells.len() as f64
        }
    }

    /// Flag a memory cell as used, identified by its local pair identifier.
    /// Return true if found and it was valid.
    pub fn used(&mut self, local_pair_id: u64) -> bool {
        for memory_cell in &mut self.memory_cells {
            if let Some(data) = memory_cell.data() {
                if data.local_pair_id == local_pair_id {
                    if memory_cell.is_valid() {
                        memory_cell.used();
                        return true;
                    } else {
                        return false;
                    }
                }
            }
        }
        false
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

    /// Return the local pair ID of the newest valid memory cell, if any.
    pub fn newest_valid(&self) -> Option<u64> {
        if let Some(memory_cell) = self
            .memory_cells
            .iter()
            .filter(|cell| cell.is_valid())
            .max()
        {
            memory_cell.local_pair_id()
        } else {
            None
        }
    }

    pub fn print_all_cells(&self) {
        for (ndx, memory_cell) in self.memory_cells.iter().enumerate() {
            println!("{}\t{} {:?}", ndx, memory_cell, memory_cell.data());
        }
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
            assert_eq!(i, nic.newest_valid().unwrap());
            assert_float_eq::assert_f64_near!(0.1 * (i + 1) as f64, nic.occupancy());
        }

        assert_float_eq::assert_f64_near!(1.0, nic.occupancy());

        for cell in &nic.memory_cells {
            assert!(!cell.is_empty());
        }

        // Consume all the EPR pairs.
        assert!(nic.consume(10).is_none());
        for i in 0..10_u64 {
            let data = nic.consume(i).unwrap();

            assert_eq!(i + 100, data.created);
            assert_eq!(i, data.local_pair_id);

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
                        (data.local_pair_id >= 1 && data.local_pair_id <= 9)
                            || data.local_pair_id == 42
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
            assert!(!nic.used(i));
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
        assert!(!nic.used(99));

        // Use all the EPR pairs.
        for i in 0..10 {
            assert!(nic.used(i));
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

        // Cannot use cells already used.
        for i in 0..10 {
            assert!(!nic.used(i));
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
            let local_pair_id = if i == 7 { 2099 } else { i };
            let exp_created = if i == 7 { 1099 } else { i + 100 };

            let data = nic.consume(local_pair_id).unwrap();
            assert_eq!(exp_created, data.created);
            assert_eq!(local_pair_id, data.local_pair_id);

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
