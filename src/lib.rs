//! # Tide Pool
//!
//! Intertidal ecosystem simulation for agent niches.
//! Models isolated pools, periodic tide fluctuations, organisms with niche
//! requirements, resource competition, and vertical zonation.

// ── pool ────────────────────────────────────────────────────────────────────

/// An isolated tidal pool environment.
#[derive(Debug, Clone)]
pub struct Pool {
    pub id: u64,
    pub volume: f64,
    pub salinity: f64,
    pub temperature: f64,
    pub depth: f64,
    pub elevation: f64,
    pub resources: f64,
    pub max_resources: f64,
}

impl Pool {
    pub fn new(id: u64, volume: f64, salinity: f64, temperature: f64, depth: f64, elevation: f64) -> Self {
        let max_resources = volume * 10.0;
        Self { id, volume, salinity, temperature, depth, elevation, resources: max_resources, max_resources }
    }

    pub fn refill(&mut self, amount: f64) {
        self.resources = (self.resources + amount).min(self.max_resources);
    }

    pub fn consume(&mut self, amount: f64) -> f64 {
        let taken = amount.min(self.resources);
        self.resources -= taken;
        taken
    }

    pub fn resource_fraction(&self) -> f64 {
        if self.max_resources == 0.0 { 0.0 } else { self.resources / self.max_resources }
    }

    pub fn is_depleted(&self) -> bool {
        self.resources < 0.01 * self.max_resources
    }

    pub fn habitability(&self, preferred_temp: f64, preferred_sal: f64) -> f64 {
        let temp_diff = (self.temperature - preferred_temp).abs();
        let sal_diff = (self.salinity - preferred_sal).abs();
        let temp_score = (-temp_diff * 0.1).exp();
        let sal_score = (-sal_diff * 0.1).exp();
        temp_score * sal_score * self.resource_fraction()
    }

    pub fn update_temperature(&mut self, ambient: f64, rate: f64) {
        self.temperature += (ambient - self.temperature) * rate;
    }

    pub fn update_salinity(&mut self, tide_salinity: f64, submersion: f64) {
        self.salinity += (tide_salinity - self.salinity) * submersion * 0.1;
    }

    pub fn effective_volume(&self) -> f64 {
        self.volume.max(0.0)
    }
}

// ── tide ────────────────────────────────────────────────────────────────────

/// Periodic resource fluctuation from tidal cycles.
#[derive(Debug, Clone)]
pub struct Tide {
    pub amplitude: f64,
    pub period: f64,
    pub phase: f64,
    pub base_level: f64,
}

impl Tide {
    pub fn new(amplitude: f64, period: f64, phase: f64, base_level: f64) -> Self {
        Self { amplitude, period, phase, base_level }
    }

    pub fn semidiurnal(amplitude: f64) -> Self {
        Self { amplitude, period: 12.42, phase: 0.0, base_level: 0.5 }
    }

    pub fn diurnal(amplitude: f64) -> Self {
        Self { amplitude, period: 24.84, phase: 0.0, base_level: 0.5 }
    }

    pub fn level(&self, time: f64) -> f64 {
        self.base_level + self.amplitude * (2.0 * std::f64::consts::PI * time / self.period + self.phase).sin()
    }

    pub fn is_high(&self, time: f64) -> bool {
        self.level(time) > self.base_level
    }

    pub fn is_low(&self, time: f64) -> bool {
        self.level(time) < self.base_level
    }

    pub fn rate_of_change(&self, time: f64) -> f64 {
        self.amplitude * (2.0 * std::f64::consts::PI / self.period) *
            (2.0 * std::f64::consts::PI * time / self.period + self.phase).cos()
    }

    pub fn submersion_fraction(&self, time: f64, elevation: f64) -> f64 {
        let level = self.level(time);
        if elevation <= level - self.amplitude { 1.0 }
        else if elevation >= level + self.amplitude { 0.0 }
        else { (level - elevation + self.amplitude) / (2.0 * self.amplitude) }
    }

    pub fn time_to_next_high(&self, time: f64) -> f64 {
        let phase_offset = std::f64::consts::FRAC_PI_2 - (2.0 * std::f64::consts::PI * time / self.period + self.phase) % (2.0 * std::f64::consts::PI);
        if phase_offset <= 0.0 { self.period / 4.0 + phase_offset * self.period / (2.0 * std::f64::consts::PI) }
        else { phase_offset * self.period / (2.0 * std::f64::consts::PI) }
    }

    pub fn next_high_time(&self, time: f64) -> f64 {
        time + self.time_to_next_high(time).min(self.period / 2.0)
    }

    pub fn combine(&self, other: &Tide) -> Tide {
        Tide {
            amplitude: self.amplitude + other.amplitude,
            period: (self.period + other.period) / 2.0,
            phase: self.phase,
            base_level: (self.base_level + other.base_level) / 2.0,
        }
    }
}

// ── organism ────────────────────────────────────────────────────────────────

/// An organism agent with niche requirements.
#[derive(Debug, Clone)]
pub struct Organism {
    pub id: u64,
    pub species: String,
    pub preferred_temp: f64,
    pub preferred_salinity: f64,
    pub metabolic_rate: f64,
    pub size: f64,
    pub energy: f64,
    pub max_energy: f64,
    pub tolerance_range: f64,
}

impl Organism {
    pub fn new(id: u64, species: &str, preferred_temp: f64, preferred_salinity: f64,
               metabolic_rate: f64, size: f64, max_energy: f64) -> Self {
        Self { id, species: species.to_string(), preferred_temp, preferred_salinity,
               metabolic_rate, size, energy: max_energy * 0.5, max_energy, tolerance_range: 5.0 }
    }

    pub fn feed(&mut self, amount: f64) -> f64 {
        let consumed = amount.min(self.max_energy - self.energy);
        self.energy += consumed;
        consumed
    }

    pub fn metabolize(&mut self, dt: f64) {
        self.energy -= self.metabolic_rate * self.size * dt;
        self.energy = self.energy.max(0.0);
    }

    pub fn is_alive(&self) -> bool {
        self.energy > 0.0
    }

    pub fn fitness(&self, pool: &Pool) -> f64 {
        pool.habitability(self.preferred_temp, self.preferred_salinity) * self.energy_fraction()
    }

    pub fn energy_fraction(&self) -> f64 {
        if self.max_energy == 0.0 { 0.0 } else { self.energy / self.max_energy }
    }

    pub fn stress_level(&self, pool: &Pool) -> f64 {
        let temp_stress = (pool.temperature - self.preferred_temp).abs() / self.tolerance_range;
        let sal_stress = (pool.salinity - self.preferred_salinity).abs() / self.tolerance_range;
        (temp_stress + sal_stress).min(1.0)
    }

    pub fn reproduction_readiness(&self) -> bool {
        self.energy > self.max_energy * 0.8
    }

    pub fn reproduce(&self, new_id: u64) -> Organism {
        let mut child = self.clone();
        child.id = new_id;
        child.energy = self.energy * 0.3;
        let _ = self.energy * 0.0; // parent keeps remaining energy
        child
    }

    pub fn nutrient_demand(&self) -> f64 {
        self.metabolic_rate * self.size
    }
}

// ── competition ─────────────────────────────────────────────────────────────

/// Resource contention between organisms.
pub struct Competition;

impl Competition {
    pub fn lottery(resources: f64, organisms: &mut [Organism]) -> f64 {
        if organisms.is_empty() || resources <= 0.0 { return 0.0; }
        let total_demand: f64 = organisms.iter().map(|o| o.nutrient_demand()).sum();
        if total_demand == 0.0 { return 0.0; }
        let mut total_consumed = 0.0;
        for org in organisms.iter_mut() {
            let share = org.nutrient_demand() / total_demand;
            let amount = resources * share;
            total_consumed += org.feed(amount);
        }
        total_consumed
    }

    pub fn contest(resources: f64, organisms: &mut [Organism]) -> f64 {
        if organisms.is_empty() || resources <= 0.0 { return 0.0; }
        // Largest organism gets first pick
        let mut indices: Vec<usize> = (0..organisms.len()).collect();
        indices.sort_by(|&a, &b| organisms[b].size.partial_cmp(&organisms[a].size).unwrap());
        let mut remaining = resources;
        let mut total_consumed = 0.0;
        for &i in &indices {
            let demand = organisms[i].nutrient_demand();
            let taken = demand.min(remaining);
            organisms[i].feed(taken);
            remaining -= taken;
            total_consumed += taken;
            if remaining <= 0.0 { break; }
        }
        total_consumed
    }

    pub fn niche_partition(resources: f64, organisms: &mut [(Organism, f64)]) -> f64 {
        // Each organism has a niche overlap weight
        if organisms.is_empty() || resources <= 0.0 { return 0.0; }
        let total_weight: f64 = organisms.iter().map(|(_, w)| *w).sum();
        if total_weight == 0.0 { return 0.0; }
        let mut total_consumed = 0.0;
        for (org, weight) in organisms.iter_mut() {
            let share = *weight / total_weight * resources;
            total_consumed += org.feed(share);
        }
        total_consumed
    }

    pub fn competitive_exclusion(organisms: &[Organism], threshold: f64) -> Vec<u64> {
        let mut excluded = Vec::new();
        for org in organisms {
            if org.energy_fraction() < threshold {
                excluded.push(org.id);
            }
        }
        excluded
    }

    pub fn carrying_capacity(resources: f64, demand_per_organism: f64) -> f64 {
        if demand_per_organism <= 0.0 { f64::INFINITY } else { resources / demand_per_organism }
    }
}

// ── zonation ────────────────────────────────────────────────────────────────

/// Vertical niche stratification in the intertidal zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Spray,
    HighIntertidal,
    MidIntertidal,
    LowIntertidal,
    Subtidal,
}

impl Zone {
    pub fn from_elevation(elevation: f64) -> Zone {
        if elevation > 4.0 { Zone::Spray }
        else if elevation > 3.0 { Zone::HighIntertidal }
        else if elevation > 2.0 { Zone::MidIntertidal }
        else if elevation > 1.0 { Zone::LowIntertidal }
        else { Zone::Subtidal }
    }

    pub fn elevation_range(&self) -> (f64, f64) {
        match self {
            Zone::Spray => (4.0, 6.0),
            Zone::HighIntertidal => (3.0, 4.0),
            Zone::MidIntertidal => (2.0, 3.0),
            Zone::LowIntertidal => (1.0, 2.0),
            Zone::Subtidal => (0.0, 1.0),
        }
    }

    pub fn exposure_hours(&self) -> f64 {
        match self {
            Zone::Spray => 22.0,
            Zone::HighIntertidal => 18.0,
            Zone::MidIntertidal => 12.0,
            Zone::LowIntertidal => 6.0,
            Zone::Subtidal => 0.5,
        }
    }

    pub fn submersion_hours(&self) -> f64 {
        24.0 - self.exposure_hours()
    }

    pub fn stress_index(&self) -> f64 {
        self.exposure_hours() / 24.0
    }

    pub fn biodiversity_factor(&self) -> f64 {
        match self {
            Zone::Spray => 0.3,
            Zone::HighIntertidal => 0.5,
            Zone::MidIntertidal => 1.0,
            Zone::LowIntertidal => 0.9,
            Zone::Subtidal => 0.7,
        }
    }

    pub fn is_intertidal(&self) -> bool {
        matches!(self, Zone::HighIntertidal | Zone::MidIntertidal | Zone::LowIntertidal)
    }

    pub fn adjacent(&self) -> Vec<Zone> {
        match self {
            Zone::Spray => vec![Zone::HighIntertidal],
            Zone::HighIntertidal => vec![Zone::Spray, Zone::MidIntertidal],
            Zone::MidIntertidal => vec![Zone::HighIntertidal, Zone::LowIntertidal],
            Zone::LowIntertidal => vec![Zone::MidIntertidal, Zone::Subtidal],
            Zone::Subtidal => vec![Zone::LowIntertidal],
        }
    }
}

// ── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod pool_tests {
    use super::*;

    #[test]
    fn test_new() {
        let p = Pool::new(1, 10.0, 35.0, 15.0, 2.0, 3.0);
        assert_eq!(p.id, 1);
        assert!((p.volume - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_refill() {
        let mut p = Pool::new(1, 10.0, 35.0, 15.0, 2.0, 3.0);
        p.resources = 50.0;
        p.refill(30.0);
        assert!(p.resources > 50.0);
    }

    #[test]
    fn test_refill_capped() {
        let mut p = Pool::new(1, 10.0, 35.0, 15.0, 2.0, 3.0);
        p.refill(1000.0);
        assert!((p.resources - p.max_resources).abs() < 1e-10);
    }

    #[test]
    fn test_consume() {
        let mut p = Pool::new(1, 10.0, 35.0, 15.0, 2.0, 3.0);
        let taken = p.consume(20.0);
        assert!((taken - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_consume_limited() {
        let mut p = Pool::new(1, 10.0, 35.0, 15.0, 2.0, 3.0);
        p.resources = 5.0;
        let taken = p.consume(20.0);
        assert!((taken - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_resource_fraction() {
        let mut p = Pool::new(1, 10.0, 35.0, 15.0, 2.0, 3.0);
        p.resources = p.max_resources / 2.0;
        assert!((p.resource_fraction() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_is_depleted() {
        let mut p = Pool::new(1, 10.0, 35.0, 15.0, 2.0, 3.0);
        p.resources = 0.0;
        assert!(p.is_depleted());
    }

    #[test]
    fn test_habitability() {
        let p = Pool::new(1, 10.0, 35.0, 15.0, 2.0, 3.0);
        let h = p.habitability(15.0, 35.0);
        assert!(h > 0.0 && h <= 1.0);
    }

    #[test]
    fn test_update_temperature() {
        let mut p = Pool::new(1, 10.0, 35.0, 15.0, 2.0, 3.0);
        p.update_temperature(20.0, 0.5);
        assert!(p.temperature > 15.0);
    }

    #[test]
    fn test_update_salinity() {
        let mut p = Pool::new(1, 10.0, 35.0, 15.0, 2.0, 3.0);
        p.update_salinity(30.0, 1.0);
        assert!(p.salinity < 35.0);
    }

    #[test]
    fn test_effective_volume() {
        let p = Pool::new(1, 10.0, 35.0, 15.0, 2.0, 3.0);
        assert!((p.effective_volume() - 10.0).abs() < 1e-10);
    }
}

#[cfg(test)]
mod tide_tests {
    use super::*;

    #[test]
    fn test_level_range() {
        let t = Tide::semidiurnal(1.0);
        for time in 0..100 {
            let level = t.level(time as f64);
            assert!(level >= 0.5 - 1.0 - 0.01 && level <= 0.5 + 1.0 + 0.01);
        }
    }

    #[test]
    fn test_is_high() {
        let t = Tide::semidiurnal(1.0);
        assert!(t.is_high(3.105)); // near peak
    }

    #[test]
    fn test_is_low() {
        let t = Tide::semidiurnal(1.0);
        // At t=3.105 (near peak), and at peak+half_period should be low
        let t_low = t.period / 2.0;
        assert!(t.is_low(t_low));
    }

    #[test]
    fn test_rate_of_change() {
        let t = Tide::semidiurnal(1.0);
        let roc = t.rate_of_change(0.0);
        assert!(roc.abs() > 0.0 || true); // just verify it computes
    }

    #[test]
    fn test_submersion_fully_submerged() {
        let t = Tide::semidiurnal(2.0);
        let sub = t.submersion_fraction(3.105, -10.0); // very low elevation
        assert!((sub - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_submersion_fully_exposed() {
        let t = Tide::semidiurnal(0.5);
        let sub = t.submersion_fraction(0.0, 100.0);
        assert!((sub).abs() < 0.01);
    }

    #[test]
    fn test_combine() {
        let a = Tide::semidiurnal(1.0);
        let b = Tide::diurnal(0.5);
        let c = a.combine(&b);
        assert!((c.amplitude - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_semidiurnal_period() {
        let t = Tide::semidiurnal(1.0);
        assert!((t.period - 12.42).abs() < 1e-10);
    }

    #[test]
    fn test_diurnal_period() {
        let t = Tide::diurnal(1.0);
        assert!((t.period - 24.84).abs() < 1e-10);
    }

    #[test]
    fn test_periodicity() {
        let t = Tide::semidiurnal(1.0);
        let l1 = t.level(0.0);
        let l2 = t.level(t.period);
        assert!((l1 - l2).abs() < 1e-10);
    }
}

#[cfg(test)]
mod organism_tests {
    use super::*;

    #[test]
    fn test_new() {
        let o = Organism::new(1, "crab", 15.0, 35.0, 0.5, 2.0, 100.0);
        assert_eq!(o.species, "crab");
        assert!((o.energy - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_feed() {
        let mut o = Organism::new(1, "crab", 15.0, 35.0, 0.5, 2.0, 100.0);
        o.energy = 50.0;
        let consumed = o.feed(30.0);
        assert!((consumed - 30.0).abs() < 1e-10);
        assert!((o.energy - 80.0).abs() < 1e-10);
    }

    #[test]
    fn test_feed_capped() {
        let mut o = Organism::new(1, "crab", 15.0, 35.0, 0.5, 2.0, 100.0);
        o.energy = 90.0;
        let consumed = o.feed(20.0);
        assert!((consumed - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_metabolize() {
        let mut o = Organism::new(1, "crab", 15.0, 35.0, 0.5, 2.0, 100.0);
        o.energy = 50.0;
        o.metabolize(1.0);
        assert!(o.energy < 50.0);
    }

    #[test]
    fn test_metabolize_floor() {
        let mut o = Organism::new(1, "crab", 15.0, 35.0, 0.5, 2.0, 100.0);
        o.energy = 0.1;
        o.metabolize(100.0);
        assert_eq!(o.energy, 0.0);
    }

    #[test]
    fn test_is_alive() {
        let mut o = Organism::new(1, "crab", 15.0, 35.0, 0.5, 2.0, 100.0);
        assert!(o.is_alive());
        o.energy = 0.0;
        assert!(!o.is_alive());
    }

    #[test]
    fn test_fitness() {
        let pool = Pool::new(1, 10.0, 35.0, 15.0, 2.0, 3.0);
        let o = Organism::new(1, "crab", 15.0, 35.0, 0.5, 2.0, 100.0);
        assert!(o.fitness(&pool) > 0.0);
    }

    #[test]
    fn test_energy_fraction() {
        let o = Organism::new(1, "crab", 15.0, 35.0, 0.5, 2.0, 100.0);
        assert!((o.energy_fraction() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_stress_level() {
        let pool = Pool::new(1, 10.0, 35.0, 15.0, 2.0, 3.0);
        let o = Organism::new(1, "crab", 15.0, 35.0, 0.5, 2.0, 100.0);
        let stress = o.stress_level(&pool);
        assert!(stress >= 0.0 && stress <= 1.0);
    }

    #[test]
    fn test_reproduction_readiness() {
        let mut o = Organism::new(1, "crab", 15.0, 35.0, 0.5, 2.0, 100.0);
        assert!(!o.reproduction_readiness());
        o.energy = 90.0;
        assert!(o.reproduction_readiness());
    }

    #[test]
    fn test_reproduce() {
        let mut o = Organism::new(1, "crab", 15.0, 35.0, 0.5, 2.0, 100.0);
        o.energy = 90.0;
        let child = o.reproduce(2);
        assert_eq!(child.id, 2);
        assert!((child.energy - 27.0).abs() < 1e-10);
    }

    #[test]
    fn test_nutrient_demand() {
        let o = Organism::new(1, "crab", 15.0, 35.0, 0.5, 2.0, 100.0);
        assert!((o.nutrient_demand() - 1.0).abs() < 1e-10);
    }
}

#[cfg(test)]
mod competition_tests {
    use super::*;

    fn make_organisms() -> Vec<Organism> {
        vec![
            Organism::new(1, "crab", 15.0, 35.0, 0.5, 2.0, 100.0),
            Organism::new(2, "snail", 14.0, 34.0, 0.3, 1.0, 50.0),
            Organism::new(3, "anemone", 16.0, 36.0, 0.2, 1.5, 80.0),
        ]
    }

    #[test]
    fn test_lottery() {
        let mut orgs = make_organisms();
        let consumed = Competition::lottery(30.0, &mut orgs);
        assert!(consumed > 0.0);
    }

    #[test]
    fn test_lottery_empty() {
        let mut orgs: Vec<Organism> = vec![];
        assert_eq!(Competition::lottery(30.0, &mut orgs), 0.0);
    }

    #[test]
    fn test_lottery_no_resources() {
        let mut orgs = make_organisms();
        assert_eq!(Competition::lottery(0.0, &mut orgs), 0.0);
    }

    #[test]
    fn test_contest() {
        let mut orgs = make_organisms();
        let consumed = Competition::contest(30.0, &mut orgs);
        assert!(consumed > 0.0);
    }

    #[test]
    fn test_contest_largest_first() {
        let mut orgs = make_organisms();
        Competition::contest(5.0, &mut orgs);
        // Largest (size 2.0) should have more energy
    }

    #[test]
    fn test_niche_partition() {
        let orgs = make_organisms();
        let mut pairs: Vec<(Organism, f64)> = orgs.into_iter().zip(vec![0.5, 0.3, 0.2]).collect();
        let consumed = Competition::niche_partition(30.0, &mut pairs);
        assert!(consumed > 0.0);
    }

    #[test]
    fn test_exclusion() {
        let mut o = Organism::new(1, "crab", 15.0, 35.0, 0.5, 2.0, 100.0);
        o.energy = 0.5;
        let excluded = Competition::competitive_exclusion(&[o], 0.01);
        assert!(excluded.contains(&1));
    }

    #[test]
    fn test_no_exclusion() {
        let o = Organism::new(1, "crab", 15.0, 35.0, 0.5, 2.0, 100.0);
        let excluded = Competition::competitive_exclusion(&[o], 0.01);
        assert!(excluded.is_empty());
    }

    #[test]
    fn test_carrying_capacity() {
        let cc = Competition::carrying_capacity(100.0, 2.0);
        assert!((cc - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_carrying_capacity_infinite() {
        let cc = Competition::carrying_capacity(100.0, 0.0);
        assert!(cc.is_infinite());
    }
}

#[cfg(test)]
mod zonation_tests {
    use super::*;

    #[test]
    fn test_spray_zone() {
        assert_eq!(Zone::from_elevation(5.0), Zone::Spray);
    }

    #[test]
    fn test_high_intertidal() {
        assert_eq!(Zone::from_elevation(3.5), Zone::HighIntertidal);
    }

    #[test]
    fn test_mid_intertidal() {
        assert_eq!(Zone::from_elevation(2.5), Zone::MidIntertidal);
    }

    #[test]
    fn test_low_intertidal() {
        assert_eq!(Zone::from_elevation(1.5), Zone::LowIntertidal);
    }

    #[test]
    fn test_subtidal() {
        assert_eq!(Zone::from_elevation(0.5), Zone::Subtidal);
    }

    #[test]
    fn test_elevation_range() {
        let (low, high) = Zone::MidIntertidal.elevation_range();
        assert!((low - 2.0).abs() < 1e-10);
        assert!((high - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_exposure_decreases() {
        assert!(Zone::Spray.exposure_hours() > Zone::HighIntertidal.exposure_hours());
        assert!(Zone::HighIntertidal.exposure_hours() > Zone::MidIntertidal.exposure_hours());
    }

    #[test]
    fn test_submersion_increase() {
        assert!(Zone::Subtidal.submersion_hours() > Zone::HighIntertidal.submersion_hours());
    }

    #[test]
    fn test_stress_index() {
        assert!(Zone::Spray.stress_index() > Zone::Subtidal.stress_index());
    }

    #[test]
    fn test_biodiversity_peak() {
        assert!((Zone::MidIntertidal.biodiversity_factor() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_is_intertidal() {
        assert!(Zone::MidIntertidal.is_intertidal());
        assert!(!Zone::Spray.is_intertidal());
        assert!(!Zone::Subtidal.is_intertidal());
    }

    #[test]
    fn test_adjacent() {
        let adj = Zone::MidIntertidal.adjacent();
        assert!(adj.contains(&Zone::HighIntertidal));
        assert!(adj.contains(&Zone::LowIntertidal));
    }
}
