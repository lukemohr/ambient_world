//! Core logic for the world state.

use rand::{Rng, seq::IndexedRandom};

const DECAY_FACTOR: f64 = 1.;

/// Defines the current world state.
///
/// The world state is used to affect audio and visuals.
pub struct WorldState {
    density: f64,
    rhythm: f64,
    tension: f64,
    energy: f64,
    warmth: f64,
}

/// World state to share outwardly at a point in time.
pub struct WorldSnapshot {
    density: f64,
    rhythm: f64,
    tension: f64,
    energy: f64,
    warmth: f64,
}

impl Default for WorldState {
    fn default() -> Self {
        Self {
            density: 0.5,
            rhythm: 0.5,
            tension: 0.5,
            energy: 0.5,
            warmth: 0.5,
        }
    }
}

impl WorldState {
    /// Initializes the world state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Introduces a random drift to the world state parameters.
    pub fn drift(&self, df: f64, rng: &mut impl Rng) -> Self {
        let drift_dir = [-1., 1.];
        let mut compute_drift = |current: f64| {
            let dir = drift_dir.choose(rng).copied().unwrap_or(0.);
            (current + df * dir).clamp(0., 1.)
        };
        let compute_decay = |current: f64| {
            let decay: f64 = DECAY_FACTOR * df * (current - 0.5) / 0.5;
            (current - decay).clamp(0., 1.)
        };
        let mut apply_transform = |value: f64| compute_decay(compute_drift(value));

        Self {
            density: apply_transform(self.density),
            rhythm: apply_transform(self.rhythm),
            tension: apply_transform(self.tension),
            energy: apply_transform(self.energy),
            warmth: apply_transform(self.warmth),
        }
    }
}

impl WorldSnapshot {
    /// Creates a snapshot of the current world state.
    pub fn from_world_state(world_state: &WorldState) -> Self {
        Self {
            density: world_state.density,
            rhythm: world_state.rhythm,
            tension: world_state.tension,
            energy: world_state.energy,
            warmth: world_state.warmth,
        }
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
            state = state.drift(0.05, &mut rng);
        }
        assert!((0.0..=1.0).contains(&state.density));
        assert!((0.0..=1.0).contains(&state.rhythm));
        assert!((0.0..=1.0).contains(&state.tension));
        assert!((0.0..=1.0).contains(&state.energy));
        assert!((0.0..=1.0).contains(&state.warmth));
    }
}
