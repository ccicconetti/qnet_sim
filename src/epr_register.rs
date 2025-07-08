// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

// EPR pair.
#[derive(Debug)]
pub struct EprPair {
    /// Identifier of one of the nodes holding the EPR pair or None if consumed.
    alice_id: Option<u32>,
    /// Identifier of the other node holding the EPR pair or None if consumed.
    bob_id: Option<u32>,
    /// Time when the fidelity was last updated.
    updated: u64,
    /// Fidelity the EPR pair at `updated` time.
    fidelity: f64,
}

impl EprPair {
    /// Consume one end of the EPR pair.
    /// Return None if `node_id` does not match any of the nodes' indices,
    /// otherwise return true if the EPR pair is fully consumed.
    pub fn consume(&mut self, node_id: u32) -> Option<(u64, f64, bool)> {
        let alice_id = self.alice_id.unwrap_or(u32::MAX);
        let bob_id = self.bob_id.unwrap_or(u32::MAX);

        if node_id != alice_id && node_id != bob_id {
            return None;
        }

        if node_id == alice_id {
            self.alice_id = None;
        } else if node_id == bob_id {
            self.bob_id = None;
        }

        Some((
            self.updated,
            self.fidelity,
            self.alice_id.is_none() && self.bob_id.is_none(),
        ))
    }
}

#[derive(Debug, Default)]
pub struct EprRegister {
    epr_pairs: std::collections::HashMap<u64, EprPair>,
    last_epr_pair_id: u64,
}

impl EprRegister {
    /// Create a new EPR pair with given characteristics. Return its identifier.
    pub fn new_epr_pair(&mut self, alice_id: u32, bob_id: u32, updated: u64, fidelity: f64) -> u64 {
        let epr_pair_id = self.last_epr_pair_id;

        let res = self.epr_pairs.insert(
            epr_pair_id,
            EprPair {
                alice_id: Some(alice_id),
                bob_id: Some(bob_id),
                updated,
                fidelity,
            },
        );
        assert!(
            res.is_none(),
            "The EPR pair register contains already ID {epr_pair_id}"
        );

        self.last_epr_pair_id += 1;
        epr_pair_id
    }

    /// Consume an EPR pair with given ID at a node.
    /// Remove the EPR pair if both end-points consumed it.
    pub fn consume(&mut self, epr_pair_id: u64, node_id: u32) -> Option<(u64, f64)> {
        let epr_pair = self.epr_pairs.get_mut(&epr_pair_id);
        let ret = if let Some(epr_pair) = epr_pair {
            epr_pair.consume(node_id)
        } else {
            None
        };

        if let Some((updated, fidelity, remove)) = ret {
            if remove {
                self.epr_pairs.remove(&epr_pair_id);
            }
            Some((updated, fidelity))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EprPair, EprRegister};

    #[test]
    fn test_epr_pair_consume() {
        let mut epr_pair = EprPair {
            alice_id: Some(1),
            bob_id: Some(2),
            updated: 999,
            fidelity: 0.5,
        };

        assert!(epr_pair.consume(42).is_none());

        let (updated, fidelity, remove) = epr_pair.consume(1).unwrap();
        assert_eq!(999, updated);
        assert_float_eq::assert_f64_near!(0.5, fidelity);
        assert!(!remove);

        assert!(epr_pair.consume(1).is_none());

        let (updated, fidelity, remove) = epr_pair.consume(2).unwrap();
        assert_eq!(999, updated);
        assert_float_eq::assert_f64_near!(0.5, fidelity);
        assert!(remove);

        assert!(epr_pair.consume(1).is_none());
        assert!(epr_pair.consume(2).is_none());
    }

    #[test]
    fn test_epr_pair_register_single() {
        let mut register = EprRegister::default();
        assert_eq!(0, register.new_epr_pair(1, 2, 990, 0.42));

        assert!(register.consume(0, 99).is_none());
        assert!(register.consume(1, 1).is_none());
        assert!(register.consume(1, 2).is_none());

        let (updated, fidelity) = register.consume(0, 1).unwrap();
        assert_eq!(990, updated);
        assert_float_eq::assert_f64_near!(0.42, fidelity);

        let (updated, fidelity) = register.consume(0, 2).unwrap();
        assert_eq!(990, updated);
        assert_float_eq::assert_f64_near!(0.42, fidelity);

        assert!(register.consume(0, 1).is_none());
        assert!(register.consume(0, 2).is_none());
    }

    #[test]
    fn test_epr_pair_register_many() {
        let mut register = EprRegister::default();
        for i in 0..100_u64 {
            let alice_id = 1;
            let bob_id = 2;
            let updated = i;
            let fidelity = 0.42;
            assert_eq!(
                i,
                register.new_epr_pair(alice_id, bob_id, updated, fidelity)
            );
        }
        assert_eq!(100, register.epr_pairs.len());

        assert!(register.consume(999, 1).is_none());
        for i in 0..100_u64 {
            let epr_pair = register.consume(i, 1);
            assert!(epr_pair.is_some());
            let (updated, fidelity) = epr_pair.unwrap();
            assert_eq!(i, updated);
            assert_float_eq::assert_f64_near!(0.42, fidelity);
        }

        assert!(register.consume(0, 1).is_none());
        assert!(register.consume(99, 1).is_none());
    }
}
