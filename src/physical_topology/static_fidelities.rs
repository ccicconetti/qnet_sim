// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// Static fidelities, which only depend on the link type.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StaticFidelities {
    /// One hop, orbit-to-orbit.
    pub f_o: f64,
    /// One hop, orbit-to-ground.
    pub f_g: f64,
    /// Two hops, orbit-to-orbit.
    pub f_oo: f64,
    /// Two hops, orbit-to-ground.
    pub f_og: f64,
    /// Two hops, ground-to-ground.
    pub f_gg: f64,
}

impl Default for StaticFidelities {
    fn default() -> Self {
        Self {
            f_o: 1.0,
            f_g: 1.0,
            f_oo: 1.0,
            f_og: 1.0,
            f_gg: 1.0,
        }
    }
}

impl super::FidelityComputer for StaticFidelities {
    fn fidelity(
        &self,
        topology: &super::PhysicalTopology,
        tx: u32,
        u: u32,
        v: u32,
    ) -> anyhow::Result<f64> {
        super::fidelity_computer::topology_checks(topology, tx, u, v)?;

        match super::fidelity_computer::link_type(topology, tx, u, v)? {
            super::fidelity_computer::LinkType::OneOrbitOrbit => Ok(self.f_o),
            super::fidelity_computer::LinkType::OneOrbitGround => Ok(self.f_g),
            super::fidelity_computer::LinkType::TwoOrbitOrbit => Ok(self.f_oo),
            super::fidelity_computer::LinkType::TwoOrbitGround => Ok(self.f_og),
            super::fidelity_computer::LinkType::TwoGroundGround => Ok(self.f_gg),
        }
    }

    fn valid(&self) -> anyhow::Result<()> {
        let fidelities = vec![
            (self.f_o, "one-hop, orbit-to-orbit"),
            (self.f_g, "one-hop, orbit-to-ground"),
            (self.f_oo, "two-hops, orbit-to-orbit"),
            (self.f_og, "two-hops, orbit-to-ground"),
            (self.f_gg, "two-hops, ground-to-ground"),
        ];
        let mut errors = vec![];
        for (fidelity, name) in fidelities {
            if fidelity < 0.0 {
                errors.push(format!("{fidelity} fidelity ({name}) is < 0"));
            } else if fidelity > 1.0 {
                errors.push(format!("{fidelity} fidelity ({name}) is > 1"));
            }
        }
        if !errors.is_empty() {
            anyhow::bail!("invalid static fidelities: {}", errors.join(","))
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_topology::FidelityComputer;
    use crate::physical_topology::{NodeType, PhysicalTopology};

    #[test]
    fn test_static_fidelities() {
        let fidelities = StaticFidelities {
            f_o: 0.6,
            f_g: 0.7,
            f_oo: 0.8,
            f_og: 0.9,
            f_gg: 1.0,
        };

        let mut topo = PhysicalTopology::from_distances(vec![
            (0, 1, 1.0),
            (0, 2, 1.0),
            (0, 3, 1.0),
            (0, 4, 1.0),
            (4, 5, 1.0),
        ]);

        topo.graph.node_weight_mut(0.into()).unwrap().node_type = NodeType::SAT;
        topo.graph.node_weight_mut(1.into()).unwrap().node_type = NodeType::OGS;
        topo.graph.node_weight_mut(2.into()).unwrap().node_type = NodeType::OGS;
        topo.graph.node_weight_mut(3.into()).unwrap().node_type = NodeType::SAT;
        topo.graph.node_weight_mut(4.into()).unwrap().node_type = NodeType::SAT;
        topo.graph.node_weight_mut(5.into()).unwrap().node_type = NodeType::SAT;

        assert_eq!(fidelities.f_o, fidelities.fidelity(&topo, 0, 0, 3).unwrap());
        assert_eq!(fidelities.f_o, fidelities.fidelity(&topo, 0, 3, 0).unwrap());
        assert_eq!(fidelities.f_g, fidelities.fidelity(&topo, 0, 0, 1).unwrap());
        assert_eq!(fidelities.f_g, fidelities.fidelity(&topo, 0, 1, 0).unwrap());
        assert_eq!(
            fidelities.f_oo,
            fidelities.fidelity(&topo, 0, 3, 4).unwrap()
        );
        assert_eq!(
            fidelities.f_og,
            fidelities.fidelity(&topo, 0, 1, 3).unwrap()
        );
        assert_eq!(
            fidelities.f_gg,
            fidelities.fidelity(&topo, 0, 1, 2).unwrap()
        );
    }
}
