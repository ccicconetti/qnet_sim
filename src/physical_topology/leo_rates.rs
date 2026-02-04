// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// Fidelities depending on the physical characteristics of the LEO link.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LeoRates {
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
}

impl Default for LeoRates {
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
        }
    }
}

impl LeoRates {
    /// Compute the transmittance of a space link with given receiver radius,
    /// in m.
    ///
    /// Parameters
    /// - `distance`: distance from transmitter to receiver, in m
    ///
    fn eta_space_radius(&self, distance: f64, radius: f64) -> f64 {
        let z_r = std::f64::consts::PI * self.w_0.powf(2.0) / self.lambda;
        let w_l = self.w_0 * (1.0 + (self.m_square * distance / z_r).powf(2.0)).sqrt();

        // println!("z_r {z_r}");
        // println!("w_l {w_l");

        1.0 - (-2.0 * radius.powf(2.0) / w_l.powf(2.0)).exp()
    }

    /// Compute the transmittance of a space link.
    ///
    /// Parameters
    /// - `distance`: distance from transmitter to receiver, in m
    ///
    fn eta_space(&self, distance: f64) -> f64 {
        self.eta_space_radius(distance, self.r_sat)
    }

    /// Compute the transmittance of a ground link.
    ///
    /// Parameters
    /// - `distance`: distance from transmitter to receiver, in m
    /// - `elevation`: angle between transmitter and receiver, in degrees
    ///
    fn eta_ground(&self, distance: f64, elevation: f64) -> f64 {
        let elevation_rad = elevation * std::f64::consts::PI / 180.0;
        let chi_est = (-self.beta / elevation_rad.cos()).exp();

        // println!("chi_est {chi_est}");

        self.eta_space_radius(distance, self.r_ogs) * chi_est
    }
}

impl super::RateComputer for LeoRates {
    fn rate(
        &self,
        topology: &super::PhysicalTopology,
        tx: u32,
        u: u32,
        v: u32,
    ) -> anyhow::Result<f64> {
        super::fidelity_computer::topology_checks(topology, tx, u, v)?;

        // eta1 and eta2 are the transmittance values on the first and
        // second link, respectively.
        let (eta1, eta2) = match super::fidelity_computer::link_type(topology, tx, u, v)? {
            super::fidelity_computer::LinkType::OneOrbitOrbit => {
                assert!(tx == u || tx == v);
                (
                    self.eta_space(
                        topology
                            .graph
                            .edge_weight(topology.graph.find_edge(u.into(), v.into()).unwrap())
                            .unwrap()
                            .distance,
                    ),
                    1.0,
                )
            }
            super::fidelity_computer::LinkType::OneOrbitGround => {
                assert!(tx == u || tx == v);
                let weight = topology
                    .graph
                    .edge_weight(topology.graph.find_edge(u.into(), v.into()).unwrap())
                    .unwrap();
                (self.eta_ground(weight.distance, weight.elevation), 1.0)
            }
            super::fidelity_computer::LinkType::TwoOrbitOrbit => (
                self.eta_space(
                    topology
                        .graph
                        .edge_weight(topology.graph.find_edge(u.into(), tx.into()).unwrap())
                        .unwrap()
                        .distance,
                ),
                self.eta_space(
                    topology
                        .graph
                        .edge_weight(topology.graph.find_edge(tx.into(), v.into()).unwrap())
                        .unwrap()
                        .distance,
                ),
            ),
            super::fidelity_computer::LinkType::TwoOrbitGround => {
                let weight_u = topology
                    .graph
                    .edge_weight(topology.graph.find_edge(u.into(), tx.into()).unwrap())
                    .unwrap();
                let weight_v = topology
                    .graph
                    .edge_weight(topology.graph.find_edge(tx.into(), v.into()).unwrap())
                    .unwrap();
                if matches!(
                    topology.graph.node_weight(u.into()).unwrap().node_type,
                    super::NodeType::OGS
                ) {
                    (
                        self.eta_ground(weight_u.distance, weight_u.elevation),
                        self.eta_space(weight_v.distance),
                    )
                } else {
                    assert!(matches!(
                        topology.graph.node_weight(v.into()).unwrap().node_type,
                        super::NodeType::OGS
                    ));
                    (
                        self.eta_space(weight_u.distance),
                        self.eta_ground(weight_v.distance, weight_v.elevation),
                    )
                }
            }
            super::fidelity_computer::LinkType::TwoGroundGround => {
                let weight_u = topology
                    .graph
                    .edge_weight(topology.graph.find_edge(u.into(), tx.into()).unwrap())
                    .unwrap();
                let weight_v = topology
                    .graph
                    .edge_weight(topology.graph.find_edge(tx.into(), v.into()).unwrap())
                    .unwrap();
                (
                    self.eta_ground(weight_u.distance, weight_u.elevation),
                    self.eta_ground(weight_v.distance, weight_v.elevation),
                )
            }
        };

        Ok(self.f * self.p_s * (eta1 * self.eta_d) * (eta2 * self.eta_d))
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
        ];

        for (var, name) in expected_positive_values {
            if *var <= 0.0 {
                errors.push(format!("{} ({}) <= 0", var, name))
            }
        }

        if !errors.is_empty() {
            anyhow::bail!("invalid leo rates: {}", errors.join(","))
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_topology::{fidelity_computer::LinkType, RateComputer};
    use std::io::Write;

    #[test]
    fn test_leo_rates() {
        let rates = LeoRates::default();
        let topo = crate::physical_topology::tests::test_topo();

        // 1-O-O
        let rate_1oo = rates.rate(&topo, 0, 0, 3).unwrap();
        assert_float_eq::assert_f64_near!(
            rates.rate(&topo, 0, 0, 3).unwrap(),
            rates.rate(&topo, 0, 3, 0).unwrap()
        );

        // 1-O-G
        let rate_1og = rates.rate(&topo, 0, 0, 1).unwrap();
        assert_float_eq::assert_f64_near!(
            rates.rate(&topo, 0, 0, 1).unwrap(),
            rates.rate(&topo, 0, 1, 0).unwrap()
        );

        // 2-O-O
        let rate_2oo = rates.rate(&topo, 0, 3, 4).unwrap();
        assert_float_eq::assert_f64_near!(
            rates.rate(&topo, 0, 3, 4).unwrap(),
            rates.rate(&topo, 0, 4, 3).unwrap()
        );

        // 2-O-G
        let rate_2og = rates.rate(&topo, 0, 1, 3).unwrap();
        assert_float_eq::assert_f64_near!(
            rates.rate(&topo, 0, 1, 3).unwrap(),
            rates.rate(&topo, 0, 3, 1).unwrap()
        );

        // 2-G-G
        let rate_2gg = rates.rate(&topo, 0, 1, 2).unwrap();
        assert_float_eq::assert_f64_near!(
            rates.rate(&topo, 0, 1, 2).unwrap(),
            rates.rate(&topo, 0, 2, 1).unwrap()
        );

        // Reality checks.
        assert!(rate_1oo < rates.f * rates.p_s);
        assert!(rate_1oo > rate_2oo);
        assert!(rate_1og > rate_2og);
        assert!(rate_2og > rate_2gg);
        assert!(rate_1oo > rate_1og);
    }

    #[ignore]
    #[test]
    fn print_leo_rates() {
        let rates = LeoRates::default();

        let mut outfile = std::fs::OpenOptions::new()
            .write(true)
            .append(false)
            .create(true)
            .truncate(true)
            .open("leo_rates.csv")
            .unwrap();

        let elevation = 42.0; // degrees, only for orbit-ground links
        let link_types = [
            LinkType::OneOrbitOrbit,
            LinkType::OneOrbitGround,
            LinkType::TwoOrbitOrbit,
            LinkType::TwoOrbitGround,
            LinkType::TwoGroundGround,
        ];

        let _ = writeln!(outfile, "link_type,distance_km,rate");
        for link_type in link_types {
            for distance in 1..60 {
                let distance = distance * 100;
                let distance_m = distance as f64 * 1000.0;

                let (eta1, eta2) = match link_type {
                    LinkType::OneOrbitOrbit => (rates.eta_space(distance_m), 1.0),
                    LinkType::OneOrbitGround => (rates.eta_ground(distance_m, elevation), 1.0),
                    LinkType::TwoOrbitOrbit => {
                        (rates.eta_space(distance_m), rates.eta_space(distance_m))
                    }
                    LinkType::TwoOrbitGround => (
                        rates.eta_space(distance_m),
                        rates.eta_ground(distance_m, elevation),
                    ),
                    LinkType::TwoGroundGround => (
                        rates.eta_ground(distance_m, elevation),
                        rates.eta_ground(distance_m, elevation),
                    ),
                };

                let rate = rates.f * rates.p_s * eta1 * eta2 * rates.eta_d.powf(2.0);
                let _ = writeln!(outfile, "{:?},{},{}", link_type, distance, rate);
            }
        }
    }
}
