// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use rand_distr::num_traits::Inv;

/// Fidelities depending on the physical characteristics of the LEO link.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LeoFidelities {
    /// Detector efficiency, $\eta_d$.
    pub eta_d: f64,
    /// SPDC pumping frequency, in Hz.
    pub f: f64,
    /// Photon entanglement generation probability, $p_s$.
    pub p_s: f64,
    /// Beam width at the transmitter, in m, $W_0$.
    pub w_0: f64,
    /// Radius of the satellite receiver telescope, in m.
    pub r_sat: f64,
    /// Radius of the on-ground station receiver telescope, in m.
    pub r_ogs: f64,
    /// Wavelength, in m, $\lambda$.
    pub lambda: f64,
    /// Quality factor of the Gaussian beam, $M^2$.
    pub m_square: f64,
    /// Atmospheric extinction parameter at 580 nm, $\beta$.
    pub beta: f64,
    /// Initial pair fidelity, $F_0$.
    pub f_0: f64,
    /// Total brightness of the sky background, $H_b$,
    /// in $W m^{-2} sr^{-1} m^{-1}$.
    pub h_b: f64,
    /// Field of view of the receiver, $\Omega_{fov}$, in $sr$.
    pub omega_fov: f64,
    /// Spectral filter bandwidth, $B_f$, in m.
    pub b_f: f64,
    /// Time filter bandwidth, $\Delta t = 1/f$.
    pub delta_t: f64,
    /// Frequency of the background photons.
    pub nu: f64,
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
            h_b: 1e3, // clear night brightness
            omega_fov: 20e-6_f64.powf(2.0),
            b_f: 0.5e-9,
            delta_t: 20.0e6.inv(),
            nu: physical_constants::SPEED_OF_LIGHT_IN_VACUUM / 580e-9,
        }
    }
}

impl LeoFidelities {
    /// Compute the probability that the detection was due to a signal photon.
    ///
    /// Parameters
    /// - `distance`: distance from transmitter to receiver, in m
    /// - `elevation`: angle between transmitter and receiver, in degrees
    ///
    fn mu(&self, distance: f64, elevation: f64) -> f64 {
        let z_r = std::f64::consts::PI * self.w_0.powf(2.0) / self.lambda;
        let w =
            self.w_0 * ((1.0 + ((self.m_square * distance / z_r) as f64).powf(2.0)) as f64).sqrt();
        let chi_est = (-self.beta / (elevation * std::f64::consts::PI / 180.0).cos()).exp();
        let eta_t = 1.0 - ((-2.0 * self.r_ogs.powf(2.0) / w.powf(2.0)) as f64).exp();
        let eta_g = eta_t * chi_est;

        // Number of signal photons per time window that we expect to observe
        // (proportional to the transmittance of the channel).
        let ns = self.p_s * eta_g * self.eta_d;

        // Expected number of environmental photons in the same time window.
        let nn = (self.h_b / (physical_constants::PLANCK_CONSTANT * self.nu))
            * self.omega_fov
            * std::f64::consts::PI
            * self.r_ogs.powf(2.0)
            * self.b_f
            * self.delta_t;

        // println!("chi_est {chi_est}");
        // println!("eta_t {eta_t}");
        // println!("z_r {z_r}");
        // println!("w {w}");
        // println!("eta_g {eta_g}");
        // println!("ns {ns}");
        // println!("nn {nn}");

        ns / (ns + nn)
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

        // mu1 and mu2 are the probabilities that the detection was due to a
        // signal photon on the first and second link, respectively.
        let (mu1, mu2) = match super::fidelity_computer::link_type(topology, tx, u, v)? {
            super::fidelity_computer::LinkType::OneOrbitOrbit => (1.0, 1.0),
            super::fidelity_computer::LinkType::OneOrbitGround => {
                assert!(tx == u || tx == v);
                let edge = topology
                    .graph
                    .edge_weight(topology.graph.find_edge(u.into(), v.into()).unwrap())
                    .unwrap();

                (self.mu(edge.distance, edge.elevation), 1.0)
            }
            super::fidelity_computer::LinkType::TwoOrbitOrbit => (1.0, 1.0),
            super::fidelity_computer::LinkType::TwoOrbitGround => {
                let ogs_node = if matches!(
                    topology.graph.node_weight(u.into()).unwrap().node_type,
                    super::NodeType::OGS
                ) {
                    u
                } else {
                    assert!(matches!(
                        topology.graph.node_weight(v.into()).unwrap().node_type,
                        super::NodeType::OGS
                    ));
                    v
                };
                let edge = topology
                    .graph
                    .edge_weight(
                        topology
                            .graph
                            .find_edge(ogs_node.into(), tx.into())
                            .unwrap(),
                    )
                    .unwrap();

                (self.mu(edge.distance, edge.elevation), 1.0)
            }
            super::fidelity_computer::LinkType::TwoGroundGround => {
                let edge1 = topology
                    .graph
                    .edge_weight(topology.graph.find_edge(u.into(), tx.into()).unwrap())
                    .unwrap();
                let edge2 = topology
                    .graph
                    .edge_weight(topology.graph.find_edge(v.into(), tx.into()).unwrap())
                    .unwrap();

                (
                    self.mu(edge1.distance, edge1.elevation),
                    self.mu(edge2.distance, edge2.elevation),
                )
            }
        };

        Ok(self.f_0 * mu1 * mu2 + (1.0 - mu1 * mu2) * 0.25)
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
            (&self.nu, "nu"),
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
    use std::io::Write;

    #[test]
    fn test_leo_fidelities() {
        let fidelities = LeoFidelities::default();
        let topo = crate::physical_topology::tests::test_topo();

        assert_eq!(fidelities.f_0, fidelities.fidelity(&topo, 0, 0, 3).unwrap());

        // Note: the values in the assertions have _not_ been derived from
        // external knowledge: they only verify non-regression, not correctness.
        assert_float_eq::assert_f64_near!(
            0.7541389793653572,
            fidelities.fidelity(&topo, 0, 0, 1).unwrap()
        );
        assert_eq!(fidelities.f_0, fidelities.fidelity(&topo, 0, 3, 4).unwrap());
        assert_float_eq::assert_f64_near!(
            0.7541389793653572,
            fidelities.fidelity(&topo, 0, 1, 3).unwrap()
        );
        assert_float_eq::assert_f64_near!(
            0.5981590555007453,
            fidelities.fidelity(&topo, 0, 1, 2).unwrap()
        );
    }

    #[ignore]
    #[test]
    fn print_leo_fidelities() {
        let fidelities = LeoFidelities::default();

        let mut outfile = std::fs::OpenOptions::new()
            .write(true)
            .append(false)
            .create(true)
            .truncate(true)
            .open("leo_fidelities.csv")
            .unwrap();

        let h_b_values = [10.0, 1e2, 1e3, 1e4, 1e5, 1e6];
        let _ = writeln!(outfile, "elevation_degrees,h_b,distance_km,fidelity");
        for elevation in 1..6 {
            let elevation = elevation * 10;
            for distance in 1..60 {
                let distance = distance * 100;
                for h_b_value in &h_b_values {
                    let mut fidelities = fidelities.clone();
                    fidelities.h_b = *h_b_value;
                    let mu = fidelities.mu(distance as f64 * 1000.0, elevation as f64);
                    let fidelity = mu * fidelities.f_0 + (1.0 - mu) * 0.25;

                    let _ = writeln!(
                        outfile,
                        "{},{},{},{}",
                        elevation, h_b_value, distance, fidelity
                    );
                }
            }
        }
    }
}
