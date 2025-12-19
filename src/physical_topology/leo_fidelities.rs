// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// Fidelities depending on the physical characteristics of the LEO link.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LeoFidelities {
    /// Detector efficiency, $\eta_d$.
    eta_d: f64,
    /// SPDC pumping frequency, in Hz.
    f: f64,
    /// Photon entanglement generation probability, $p_s$.
    p_s: f64,
    /// Beam width at the transmitter, in m, $W_0$.
    w_0: f64,
    /// Radius of the satellite receiver telescope, in m.
    r_sat: f64,
    /// Radius of the on-ground station receiver telescope, in m.
    r_ogs: f64,
    /// Wavelength, in m, $\lambda$.
    lambda: f64,
    /// Quality factor of the Gaussian beam, $M^2$.
    m_square: f64,
    /// Atmospheric extinction parameter at 580 nm, $\beta$.
    beta: f64,
    /// Initial pair fidelity, $F_0$.
    f_0: f64,
    /// Total brightness of the sky background, $H_b$,
    /// in $W m^{-2} sr^{-1} nm^{-1}$.
    h_b: f64,
    /// Field of view of the receiver, $\Omega_{fov}$, in $sr$.
    omega_fov: f64,
    /// Spectral filter bandwidth, $B_f$, in m.
    b_f: f64,
    /// Time filter bandwidth, $\Delta t = 1/f$.
    delta_t: f64,
}

impl Default for LeoFidelities {
    fn default() -> Self {
        Self {
            eta_d: 0.9,
            f: 20.0e6,
            p_s: 1e-3,
            w_0: 0.25,
            r_sat: 1.0,
            r_ogs: 1.0,
            lambda: 580.0e-9,
            m_square: 3.0,
            beta: 1.1,
            f_0: 0.98,
            h_b: 1.5e-6,
            omega_fov: 20e-6_f64.powf(2.0),
            b_f: 0.5e-9,
            delta_t: 1.0 / 20.0e6,
        }
    }
}

impl super::FidelityComputer for LeoFidelities {
    fn fidelity(
        &self,
        topology: &super::PhysicalTopology,
        tx: u32,
        u: u32,
        v: u32,
    ) -> anyhow::Result<f64> {
        super::fidelity_computer::topology_checks(topology, tx, u, v)?;

        match super::fidelity_computer::link_type(topology, tx, u, v)? {
            super::fidelity_computer::LinkType::OneOrbitOrbit => Ok(self.f_0),
            super::fidelity_computer::LinkType::OneOrbitGround => todo!(),
            super::fidelity_computer::LinkType::TwoOrbitOrbit => todo!(),
            super::fidelity_computer::LinkType::TwoOrbitGround => todo!(),
            super::fidelity_computer::LinkType::TwoGroundGround => todo!(),
        }
    }

    fn valid(&self) -> anyhow::Result<()> {
        let mut errors = vec![];

        let expected_positive_values = vec![
            (&self.eta_d, "eta_d"),
            (&self.f, "f"),
            (&self.p_s, "p_s"),
            (&self.w_0, "w_0"),
            (&self.r_sat, "r_sat"),
            (&self.r_ogs, "r_ogs"),
            (&self.lambda, "lambda"),
            (&self.m_square, "m_square"),
            (&self.beta, "beta"),
            (&self.f_0, "f_0"),
            (&self.h_b, "h_b"),
            (&self.omega_fov, "omega_fov"),
            (&self.b_f, "b_f"),
            (&self.delta_t, "delta_t"),
        ];

        for (var, name) in expected_positive_values {
            if *var <= 0.0 {
                errors.push(format!("{} ({}) <= 0", var, name))
            }
        }

        if self.f_0 > 1.0 {
            errors.push(format!("f_0 ({}) > 1", self.f_0))
        }

        if !errors.is_empty() {
            anyhow::bail!("invalid leo fidelities: {}", errors.join(","))
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
    fn test_leo_fidelities() {
        let fidelities = LeoFidelities::default();

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

        assert_eq!(fidelities.f_0, fidelities.fidelity(&topo, 0, 0, 3).unwrap());
        // assert_eq!(fidelities.f_g, fidelities.fidelity(&topo, 0, 0, 1).unwrap());
        // assert_eq!(
        //     fidelities.f_oo,
        //     fidelities.fidelity(&topo, 0, 3, 4).unwrap()
        // );
        // assert_eq!(
        //     fidelities.f_og,
        //     fidelities.fidelity(&topo, 0, 1, 3).unwrap()
        // );
        // assert_eq!(
        //     fidelities.f_gg,
        //     fidelities.fidelity(&topo, 0, 1, 2).unwrap()
        // );
    }
}
