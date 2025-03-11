// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

// EPR pair.
#[derive(Debug)]
pub struct EprPair {
    /// Identifier of one of the nodes holding the EPR pair.
    alice_id: u32,
    /// Identifier of the other node holding the EPR pair.
    bob_id: u32,
    /// Time when the fidelity was last updated.
    updated: u64,
    /// Fidelity the EPR pair had at time `updated`.
    fidelity: f64,
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
                alice_id,
                bob_id,
                updated,
                fidelity,
            },
        );
        assert!(
            res.is_none(),
            "The EPR pair register contains already ID {}",
            epr_pair_id
        );

        self.last_epr_pair_id += 1;
        epr_pair_id
    }

    /// Retrieve an EPR pair with given ID.
    pub fn pop(&mut self, epr_pair_id: u64) -> Option<EprPair> {
        self.epr_pairs.remove(&epr_pair_id)
    }
}

#[cfg(test)]
mod tests {
    use super::EprRegister;

    #[test]
    fn test_epr_pair_register() {
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

        assert!(register.pop(999).is_none());
        for i in 0..100_u64 {
            let epr_pair = register.pop(i);
            assert!(epr_pair.is_some());
            assert_eq!(i, epr_pair.unwrap().updated);
        }

        assert!(register.pop(0).is_none());
        assert!(register.pop(99).is_none());
    }
}
