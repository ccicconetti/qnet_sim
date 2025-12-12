// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

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
        topology.node_valid(tx)?;
        topology.node_valid(u)?;
        topology.node_valid(v)?;
        let tx = petgraph::graph::NodeIndex::from(tx);
        let u = petgraph::graph::NodeIndex::from(u);
        let v = petgraph::graph::NodeIndex::from(v);
        anyhow::ensure!(
            topology.graph.node_weight(tx).unwrap().transmitters > 0,
            "there are no transmitters on board of {}",
            tx.index()
        );
        anyhow::ensure!(
            u != v,
            "rx nodes are the same: {} = {}",
            u.index(),
            v.index()
        );
        anyhow::ensure!(
            matches!(
                topology.graph.node_weight(tx).unwrap().node_type,
                super::NodeType::SAT
            ),
            "node is an OGS and cannot be a transmitter: {}",
            tx.index()
        );

        if tx == u {
            anyhow::ensure!(
                topology.graph.find_edge(tx, v).is_some(),
                "there is no edge between nodes {} and {}",
                tx.index(),
                v.index()
            );
            match topology.graph.node_weight(v).unwrap().node_type {
                super::NodeType::SAT => Ok(self.f_o),
                super::NodeType::OGS => Ok(self.f_g),
            }
        } else if tx == v {
            anyhow::ensure!(
                topology.graph.find_edge(tx, u).is_some(),
                "there is no edge between nodes {} and {}",
                tx.index(),
                u.index()
            );
            match topology.graph.node_weight(u).unwrap().node_type {
                super::NodeType::SAT => Ok(self.f_o),
                super::NodeType::OGS => Ok(self.f_g),
            }
        } else {
            anyhow::ensure!(
                topology.graph.find_edge(tx, u).is_some(),
                "there is no edge between nodes {} and {}",
                tx.index(),
                u.index()
            );
            anyhow::ensure!(
                topology.graph.find_edge(tx, v).is_some(),
                "there is no edge between nodes {} and {}",
                tx.index(),
                v.index()
            );
            match topology.graph.node_weight(u).unwrap().node_type {
                super::NodeType::SAT => match topology.graph.node_weight(v).unwrap().node_type {
                    super::NodeType::SAT => Ok(self.f_oo),
                    super::NodeType::OGS => Ok(self.f_og),
                },
                super::NodeType::OGS => match topology.graph.node_weight(v).unwrap().node_type {
                    super::NodeType::SAT => Ok(self.f_og),
                    super::NodeType::OGS => Ok(self.f_gg),
                },
            }
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
