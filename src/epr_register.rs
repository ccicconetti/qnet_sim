// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use itertools::Itertools;

const P1: f64 = 1.0_f64;
const P2: f64 = 1.0_f64;
const ETA: f64 = 1.0_f64;

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
    /// ID of the EPR pair that this one was squashed into.
    squashed_by: Option<u64>,
    /// IDs of the two EPR pairs this EPR pair squashed.
    squashed: Option<(u64, u64)>,
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

    /// Mark this EPR pair as squashed by another one during ES.
    pub fn squash(&mut self, squashing_id: u64) {
        assert!(
            self.alice_id.is_some() || self.bob_id.is_some(),
            "ES: squashing an empty EPR"
        );
        self.alice_id = None;
        self.bob_id = None;
        self.squashed_by = Some(squashing_id);
    }

    /// Return true if both qubits have not been measured/consumed.
    pub fn is_complete(&self) -> bool {
        self.alice_id.is_some() && self.bob_id.is_some()
    }

    /// Return the ID of the other `node_id`, or None if not available.
    pub fn other_node_id(&self, node_id: u32) -> Option<u32> {
        if let Some(alice_id) = self.alice_id {
            if alice_id == node_id {
                self.bob_id
            } else {
                Some(alice_id)
            }
        } else {
            assert!(self.alice_id.is_none());
            self.bob_id.filter(|&bob_id| bob_id != node_id)
        }
    }

    // Return true if the given `node_id` is one of the end points of the pair.
    pub fn is_node(&self, node_id: u32) -> bool {
        (self.alice_id.is_some() && self.alice_id.unwrap() == node_id)
            || (self.bob_id.is_some() && self.bob_id.unwrap() == node_id)
    }
}

#[derive(Debug, Default)]
pub struct EprRegister {
    epr_pairs: std::collections::HashMap<u64, EprPair>,
    last_epr_pair_id: u64,
}

impl EprRegister {
    /// Return the number of currently active (non-measured) EPR pairs.
    pub fn len(&self) -> usize {
        self.epr_pairs.len()
    }

    /// Return true if the register is empty.
    pub fn is_empty(&self) -> bool {
        self.epr_pairs.is_empty()
    }

    /// Dump to stdout the content, for debugging purposes.
    pub fn dump(&self) {
        for (id, epr_pair) in self.epr_pairs.iter().sorted_by_key(|x| x.0) {
            println!("{}: {:?}", id, epr_pair);
        }
    }

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
                squashed_by: None,
                squashed: None,
            },
        );
        assert!(
            res.is_none(),
            "The EPR pair register contains already ID {epr_pair_id}"
        );

        self.last_epr_pair_id += 1;
        epr_pair_id
    }

    /// Consume an EPR pair with given ID at a node and return its data.
    ///
    /// Remove the EPR pair if both end-points consumed it.
    pub fn consume(&mut self, epr_pair_id: u64, node_id: u32) -> Option<(u64, f64)> {
        let epr_pair_id = self.find_non_squashed(epr_pair_id);

        let epr_pair = self.epr_pairs.get_mut(&epr_pair_id);
        let ret = if let Some(epr_pair) = epr_pair {
            epr_pair.consume(node_id)
        } else {
            None
        };

        if let Some((updated, fidelity, remove)) = ret {
            if remove {
                // Find all the EPR pairs squashed, including this one.
                let mut to_remove = vec![];
                self.find_all_squashed(epr_pair_id, &mut to_remove);

                // Remove all the EPR pairs.
                for id in to_remove {
                    self.epr_pairs.remove(&id);
                }
            }
            Some((updated, fidelity))
        } else {
            None
        }
    }

    /// Find the EPR pair non-squashed referenced by the given EPR pair or
    /// one of its descendants.
    fn find_non_squashed(&self, epr_pair_id: u64) -> u64 {
        if let Some(epr_pair) = self.epr_pairs.get(&epr_pair_id) {
            if let Some(squashed_by) = epr_pair.squashed_by {
                self.find_non_squashed(squashed_by)
            } else {
                epr_pair_id
            }
        } else {
            u64::MAX
        }
    }

    /// Find all the EPR pairs squashed by `epr_pair_id`, including itself.
    fn find_all_squashed(&mut self, epr_pair_id: u64, all_squashed_ids: &mut Vec<u64>) {
        if let Some(epr_pair) = self.epr_pairs.get(&epr_pair_id) {
            if let Some((squashed_alice_id, squashed_bob_id)) = epr_pair.squashed {
                self.find_all_squashed(squashed_alice_id, all_squashed_ids);
                self.find_all_squashed(squashed_bob_id, all_squashed_ids);
            }
        }
        all_squashed_ids.push(epr_pair_id);
    }

    /// Perform entanglement swapping between two EPR pairs as a result of a
    /// BSM performed by repeater node `bsm_node_id`.
    ///
    /// Parameters:
    /// - `updated`: the time the ES operation is performed.
    /// - `decay_rate`: the decay rate at the repeater.
    /// - `epr_pair_id_pred`: EPR pair ID of the node preceding the repeater.
    /// - `epr_pair_id_succ`: EPR pair ID of the node succeding the repeater.
    /// - `bsm_node_id`: ID of the repeater node.
    ///
    /// The result is that:
    /// - A new EPR pair is created with predecessor/successor end-points and
    ///   resulting fidelity, assuming that p1 = p2 = eta = 1.0.
    /// - Both the predecessor and successor EPR pairs are marked as squashed
    ///   into the new one.
    ///
    /// There are different cases if some of the EPR pairs have been
    /// (even partially) consumed:
    ///
    /// Consider:
    ///
    /// ```text
    /// A -- 1 -- B
    ///           B -- 2 -- C
    ///
    ///                result     fidelity
    /// 1: AB 2: XX    AX          0
    ///    AB    BX    AX          0
    ///    AB    CX    AC          0
    ///    AB    BC    AC          regular case, compute with formula
    /// ```
    ///
    /// Return the new EPR pair ID.
    ///
    /// Panic if there isn't at least one complete EPR pair.
    pub fn entanglement_swapping(
        &mut self,
        updated: u64,
        decay_rate: f64,
        epr_pair_id_pred: u64,
        epr_pair_id_succ: u64,
        bsm_node_id: u32,
    ) -> u64 {
        // Find the descendants of the EPR pairs given.
        let epr_pair_id_pred = self.find_non_squashed(epr_pair_id_pred);
        let epr_pair_id_succ = self.find_non_squashed(epr_pair_id_succ);

        let pred = self.epr_pairs.get(&epr_pair_id_pred);
        let succ = self.epr_pairs.get(&epr_pair_id_succ);

        assert!(
            (pred.is_some() && pred.unwrap().is_complete())
                || (succ.is_some() && succ.unwrap().is_complete())
        );

        let new_epr_pair_id = self.last_epr_pair_id;

        // Adopt this notation:
        //
        // alice ----- repeater                (pred)
        //             repeater ----- bob      (succ)
        if pred.is_some()
            && pred.unwrap().is_complete()
            && succ.is_some()
            && succ.unwrap().is_complete()
        {
            // Regular case: both EPR pairs are complete.
            #[allow(clippy::unnecessary_unwrap)]
            let pred = pred.unwrap();
            #[allow(clippy::unnecessary_unwrap)]
            let succ = succ.unwrap();

            // Check invariants.
            assert!(pred.is_node(bsm_node_id));
            assert!(succ.is_node(bsm_node_id));
            assert!(pred.updated < updated);
            assert!(succ.updated < updated);

            let alice_id = pred.other_node_id(bsm_node_id).unwrap();
            let bob_id = succ.other_node_id(bsm_node_id).unwrap();

            let f1 = crate::utils::fidelity_decay(
                pred.fidelity,
                decay_rate,
                crate::utils::to_seconds(updated - pred.updated),
            );
            let f2 = crate::utils::fidelity_decay(
                succ.fidelity,
                decay_rate,
                crate::utils::to_seconds(updated - succ.updated),
            );
            let fidelity = crate::utils::fidelity_swapping(f1, f2, P1, P2, ETA);

            self.epr_pairs.insert(
                new_epr_pair_id,
                EprPair {
                    alice_id: Some(alice_id),
                    bob_id: Some(bob_id),
                    updated,
                    fidelity,
                    squashed_by: None,
                    squashed: Some((epr_pair_id_pred, epr_pair_id_succ)),
                },
            );
        } else {
            // At least one of the EPR pairs is not present or complete.
            // Let's find which one.

            let (complete, incomplete) = if pred.is_some() && pred.unwrap().is_complete() {
                #[allow(clippy::unnecessary_unwrap)]
                (pred.unwrap(), succ)
            } else {
                assert!(succ.is_some() && succ.unwrap().is_complete());
                #[allow(clippy::unnecessary_unwrap)]
                (succ.unwrap(), pred)
            };

            let alice_id = complete.other_node_id(bsm_node_id).unwrap();
            let bob_id = if let Some(incomplete) = incomplete {
                incomplete.other_node_id(bsm_node_id)
            } else {
                None
            };

            self.epr_pairs.insert(
                new_epr_pair_id,
                EprPair {
                    alice_id: Some(alice_id),
                    bob_id,
                    updated,
                    fidelity: 0.0,
                    squashed_by: None,
                    squashed: Some((epr_pair_id_pred, epr_pair_id_succ)),
                },
            );
        }
        self.last_epr_pair_id += 1;

        // Squash the predecessor/successor EPR pairs with the new one created.
        if let Some(epr_pair) = self.epr_pairs.get_mut(&epr_pair_id_pred) {
            epr_pair.squash(new_epr_pair_id);
        }
        if let Some(epr_pair) = self.epr_pairs.get_mut(&epr_pair_id_succ) {
            epr_pair.squash(new_epr_pair_id);
        }

        new_epr_pair_id
    }
}

#[cfg(test)]
mod tests {
    use petgraph::matrix_graph::Zero;

    use super::{EprPair, EprRegister};

    #[test]
    fn test_epr_pair_consume() {
        let mut epr_pair = EprPair {
            alice_id: Some(1),
            bob_id: Some(2),
            updated: 999,
            fidelity: 0.5,
            squashed_by: None,
            squashed: None,
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

    #[test]
    fn test_epr_pair_register_entanglement_swapping() {
        let mut register = EprRegister::default();
        assert!(register.epr_pairs.is_empty());

        // 1 -- 2
        //      2 -- 3
        //           3 -- 4
        let epr1 = register.new_epr_pair(1, 2, 999, 0.9);
        let epr2 = register.new_epr_pair(2, 3, 999, 0.9);
        let epr3 = register.new_epr_pair(3, 4, 999, 0.9);
        assert_eq!(3, register.epr_pairs.len());

        let new_epr1 = register.entanglement_swapping(1999, 0.001, epr1, epr2, 2);
        let new_epr2 = register.entanglement_swapping(2999, 0.001, new_epr1, epr3, 3);
        assert_eq!(5, register.epr_pairs.len());

        let (updated1, fidelity1) = register.consume(new_epr2, 1).unwrap();
        assert_eq!(5, register.epr_pairs.len());

        let (updated2, fidelity2) = register.consume(new_epr2, 4).unwrap();
        assert!(register.epr_pairs.is_empty());

        assert_eq!(2999, updated1);
        assert_eq!(2999, updated2);

        assert_float_eq::assert_f64_near!(0.7382222197811112, fidelity1);
        assert_float_eq::assert_f64_near!(0.7382222197811112, fidelity2);
    }

    #[test]
    fn test_epr_pair_register_entanglement_swapping_alt() {
        let mut register = EprRegister::default();
        assert!(register.epr_pairs.is_empty());

        // 1 -- 2
        //      2 -- 3
        //           3 -- 4
        let epr1 = register.new_epr_pair(1, 2, 999, 0.9);
        let epr2 = register.new_epr_pair(2, 3, 999, 0.9);
        let epr3 = register.new_epr_pair(3, 4, 999, 0.9);
        assert_eq!(3, register.epr_pairs.len());

        // Use old EPR ID.
        register.entanglement_swapping(1999, 0.001, epr1, epr2, 2);
        register.entanglement_swapping(2999, 0.001, epr2, epr3, 3);
        assert_eq!(5, register.epr_pairs.len());

        let (updated1, fidelity1) = register.consume(epr1, 1).unwrap();
        assert_eq!(5, register.epr_pairs.len());

        let (updated2, fidelity2) = register.consume(epr3, 4).unwrap();
        assert!(register.epr_pairs.is_empty());

        assert_eq!(2999, updated1);
        assert_eq!(2999, updated2);

        assert_float_eq::assert_f64_near!(0.7382222197811112, fidelity1);
        assert_float_eq::assert_f64_near!(0.7382222197811112, fidelity2);
    }

    #[test]
    fn test_epr_pair_register_es_consumed_pairs() {
        let mut register = EprRegister::default();
        assert!(register.epr_pairs.is_empty());

        // 1 -- 2
        //      2 -- 3

        // Consume epr1 on node 1
        let epr1 = register.new_epr_pair(1, 2, 999, 0.9);
        let epr2 = register.new_epr_pair(2, 3, 999, 0.9);
        register.consume(epr1, 1);
        let epr_new = register.entanglement_swapping(1999, 0.001, epr1, epr2, 2);

        let (updated, fidelity) = register.consume(epr_new, 3).unwrap();
        assert_eq!(1999, updated);
        assert!(fidelity.is_zero());
        assert!(register.epr_pairs.is_empty());

        // Consume epr1 on node 2
        let epr1 = register.new_epr_pair(1, 2, 999, 0.9);
        let epr2 = register.new_epr_pair(2, 3, 999, 0.9);
        register.consume(epr1, 2);
        let epr_new = register.entanglement_swapping(1999, 0.001, epr1, epr2, 2);

        let (updated, fidelity) = register.consume(epr_new, 1).unwrap();
        assert_eq!(1999, updated);
        assert!(fidelity.is_zero());
        let (updated, fidelity) = register.consume(epr_new, 3).unwrap();
        assert_eq!(1999, updated);
        assert!(fidelity.is_zero());
        assert!(register.epr_pairs.is_empty());

        // Consume epr2 on node 2
        let epr1 = register.new_epr_pair(1, 2, 999, 0.9);
        let epr2 = register.new_epr_pair(2, 3, 999, 0.9);
        register.consume(epr2, 2);
        let epr_new = register.entanglement_swapping(1999, 0.001, epr1, epr2, 2);

        let (updated, fidelity) = register.consume(epr_new, 1).unwrap();
        assert_eq!(1999, updated);
        assert!(fidelity.is_zero());
        let (updated, fidelity) = register.consume(epr_new, 3).unwrap();
        assert_eq!(1999, updated);
        assert!(fidelity.is_zero());
        assert!(register.epr_pairs.is_empty());

        // Consume epr2 on node 3
        let epr1 = register.new_epr_pair(1, 2, 999, 0.9);
        let epr2 = register.new_epr_pair(2, 3, 999, 0.9);
        register.consume(epr2, 3);
        let epr_new = register.entanglement_swapping(1999, 0.001, epr1, epr2, 2);

        let (updated, fidelity) = register.consume(epr_new, 1).unwrap();
        assert_eq!(1999, updated);
        assert!(fidelity.is_zero());
        assert!(register.epr_pairs.is_empty());

        // Consume completely epr1
        let epr1 = register.new_epr_pair(1, 2, 999, 0.9);
        let epr2 = register.new_epr_pair(2, 3, 999, 0.9);
        register.consume(epr1, 1);
        register.consume(epr1, 2);
        let epr_new = register.entanglement_swapping(1999, 0.001, epr1, epr2, 2);

        let (updated, fidelity) = register.consume(epr_new, 3).unwrap();
        assert_eq!(1999, updated);
        assert!(fidelity.is_zero());
        assert!(register.epr_pairs.is_empty());

        // Consume completely epr2
        let epr1 = register.new_epr_pair(1, 2, 999, 0.9);
        let epr2 = register.new_epr_pair(2, 3, 999, 0.9);
        register.consume(epr2, 2);
        register.consume(epr2, 3);
        let epr_new = register.entanglement_swapping(1999, 0.001, epr1, epr2, 2);

        let (updated, fidelity) = register.consume(epr_new, 1).unwrap();
        assert_eq!(1999, updated);
        assert!(fidelity.is_zero());
        assert!(register.epr_pairs.is_empty());
    }
}
