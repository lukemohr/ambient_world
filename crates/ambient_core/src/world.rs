//! Core logic for the world state.

use rand::{Rng, seq::IndexedRandom};

const DRIFT_FACTOR: f64 = 0.2;
const DECAY_FACTOR: f64 = 0.1;

/// Defines the current world state.
///
/// The world state is used to affect audio and visuals.
pub struct WorldState {
    density: f64,
    rhythm: f64,
    tension: f64,
    energy: f64,
    warmth: f64,
    sparkle_impulse: f64,
}

/// World state to share outwardly at a point in time.
#[derive(Clone, serde::Serialize)]
pub struct WorldSnapshot {
    density: f64,
    rhythm: f64,
    tension: f64,
    energy: f64,
    warmth: f64,
    sparkle_impulse: f64,
}

impl Default for WorldState {
    fn default() -> Self {
        Self {
            density: 0.5,
            rhythm: 0.5,
            tension: 0.5,
            energy: 0.5,
            warmth: 0.5,
            sparkle_impulse: 0.0,
        }
    }
}

impl WorldState {
    /// Initializes the world state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Introduces a random drift to the world state parameters.
    /// TODO: This already takes RNG as parameter - good for deterministic mode.
    /// TODO: Future: Add WorldState::new_deterministic(seed) for testing.
    pub fn drift(&mut self, df: f64, rng: &mut impl Rng) {
        let drift_dir = [-1., 1.];
        let mut compute_drift = |current: f64| {
            let dir = drift_dir.choose(rng).copied().unwrap_or(0.);
            (current + DRIFT_FACTOR * df * dir).clamp(0., 1.)
        };
        let compute_decay = |current: f64| {
            let decay: f64 = DECAY_FACTOR * df * (current - 0.5) / 0.5;
            (current - decay).clamp(0., 1.)
        };
        let mut apply_transform = |value: f64| compute_decay(compute_drift(value));

        self.set_density(apply_transform(self.density()));
        self.set_rhythm(apply_transform(self.rhythm()));
        self.set_tension(apply_transform(self.tension()));
        self.set_energy(apply_transform(self.energy()));
        self.set_warmth(apply_transform(self.warmth()));

        // Decay sparkle impulse over time
        let current_impulse = self.sparkle_impulse();
        self.set_sparkle_impulse((current_impulse - df * 2.0).max(0.0));
    }

    // Getters
    pub fn density(&self) -> f64 {
        self.density
    }

    pub fn rhythm(&self) -> f64 {
        self.rhythm
    }

    pub fn tension(&self) -> f64 {
        self.tension
    }

    pub fn energy(&self) -> f64 {
        self.energy
    }

    pub fn warmth(&self) -> f64 {
        self.warmth
    }

    pub fn sparkle_impulse(&self) -> f64 {
        self.sparkle_impulse
    }

    // Setters
    pub fn set_density(&mut self, value: f64) {
        self.density = value.clamp(0., 1.);
    }

    pub fn set_rhythm(&mut self, value: f64) {
        self.rhythm = value.clamp(0., 1.);
    }

    pub fn set_tension(&mut self, value: f64) {
        self.tension = value.clamp(0., 1.);
    }

    pub fn set_energy(&mut self, value: f64) {
        self.energy = value.clamp(0., 1.);
    }

    pub fn set_warmth(&mut self, value: f64) {
        self.warmth = value.clamp(0., 1.);
    }

    pub fn set_sparkle_impulse(&mut self, value: f64) {
        self.sparkle_impulse = value.max(0.); // Allow values > 1.0 for impulses
    }
}

impl WorldSnapshot {
    /// Creates a snapshot of the current world state.
    pub fn from_world_state(world_state: &WorldState) -> Self {
        Self {
            density: world_state.density(),
            rhythm: world_state.rhythm(),
            tension: world_state.tension(),
            energy: world_state.energy(),
            warmth: world_state.warmth(),
            sparkle_impulse: world_state.sparkle_impulse(),
        }
    }

    // Getters
    pub fn density(&self) -> f64 {
        self.density
    }

    pub fn rhythm(&self) -> f64 {
        self.rhythm
    }

    pub fn tension(&self) -> f64 {
        self.tension
    }

    pub fn energy(&self) -> f64 {
        self.energy
    }

    pub fn warmth(&self) -> f64 {
        self.warmth
    }

    pub fn sparkle_impulse(&self) -> f64 {
        self.sparkle_impulse
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn test_drift_bounds() {
        let mut rng = StdRng::from_seed([0; 32]);
        let mut state = WorldState::new();
        for _ in 0..10000 {
            state.drift(0.05, &mut rng);
        }
        assert!((0.0..=1.0).contains(&state.density()));
        assert!((0.0..=1.0).contains(&state.rhythm()));
        assert!((0.0..=1.0).contains(&state.tension()));
        assert!((0.0..=1.0).contains(&state.energy()));
        assert!((0.0..=1.0).contains(&state.warmth()));
        assert!(state.sparkle_impulse() >= 0.0);
    }
}
