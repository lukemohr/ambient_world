use crate::events::{Event, PerformAction, TriggerKind};
use crate::world::{WorldSnapshot, WorldState};

/// The engine that updates the world state over time.
/// TODO: Consider adding drift parameter here
/// TODO: For deterministic mode/testing: inject RNG instead of using rand::rng()
/// TODO: Add WorldEngine::new_with_rng(rng) and WorldEngine::new_deterministic(seed) constructors
pub struct WorldEngine {
    state: WorldState,
    sparkle_phase: f64,
}

impl Default for WorldEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldEngine {
    /// Initializes the world engine with a default state.
    /// TODO: Add new_with_rng(rng) and new_deterministic(seed) for testing/replay
    pub fn new() -> Self {
        Self {
            state: WorldState::new(),
            sparkle_phase: 0.0,
        }
    }

    /// Apply event.
    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Tick { dt } => {
                // TODO: For deterministic mode: use injected RNG instead of rand::rng()
                self.state.drift(dt, &mut rand::rng());
                self.update_sparkles(dt);
            }
            Event::Trigger { kind, intensity } => match kind {
                TriggerKind::Pulse => self.apply_pulse(intensity),
                TriggerKind::Stir => self.apply_stir(intensity),
                TriggerKind::Calm => self.apply_calm(intensity),
                TriggerKind::Heat => self.apply_heat(intensity),
                TriggerKind::Tense => self.apply_tense(intensity),
            },
            Event::Perform(action) => match action {
                PerformAction::Pulse { intensity } => self.apply_pulse(intensity),
                PerformAction::Stir { intensity } => self.apply_stir(intensity),
                PerformAction::Calm { intensity } => self.apply_calm(intensity),
                PerformAction::Heat { intensity } => self.apply_heat(intensity),
                PerformAction::Tense { intensity } => self.apply_tense(intensity),
                PerformAction::Scene { name } => self.apply_scene(name),
                PerformAction::Freeze { seconds } => self.apply_freeze(seconds),
            },
        }
    }

    /// Apply pulse action: increases energy and slightly increases tension
    fn apply_pulse(&mut self, intensity: f64) {
        self.state.set_energy(self.state.energy() + intensity);
        self.state
            .set_tension(self.state.tension() + 0.1 * intensity);
    }

    /// Apply stir action: increases density and slightly increases tension
    fn apply_stir(&mut self, intensity: f64) {
        self.state.set_density(self.state.density() + intensity);
        self.state
            .set_tension(self.state.tension() + 0.1 * intensity);
    }

    /// Apply calm action: decreases tension and slightly decreases density
    fn apply_calm(&mut self, intensity: f64) {
        self.state.set_tension(self.state.tension() - intensity);
        self.state
            .set_density(self.state.density() - 0.1 * intensity);
    }

    /// Apply heat action: increases warmth and slightly increases energy
    fn apply_heat(&mut self, intensity: f64) {
        self.state.set_warmth(self.state.warmth() + intensity);
        self.state.set_energy(self.state.energy() + 0.1 * intensity);
    }

    /// Apply tense action: directly increases tension
    fn apply_tense(&mut self, intensity: f64) {
        self.state.set_tension(self.state.tension() + intensity);
    }

    /// Apply scene change (placeholder for future implementation)
    fn apply_scene(&mut self, name: String) {
        // For now, just log the scene change
        // TODO: Implement scene transitions
        tracing::info!("Scene change requested: {}", name);
    }

    /// Apply freeze action (placeholder for future implementation)
    fn apply_freeze(&mut self, seconds: f64) {
        // For now, just log the freeze request
        // TODO: Implement freeze functionality
        tracing::info!("Freeze requested for {} seconds", seconds);
    }

    /// Update sparkle generation based on rhythm and density
    fn update_sparkles(&mut self, dt: f64) {
        // Advance sparkle phase based on rhythm (higher rhythm = faster sparkle rate)
        let rhythm_factor = self.state.rhythm() * 2.0 + 0.5; // 0.5 to 2.5
        self.sparkle_phase += dt * rhythm_factor;

        // Check if we should generate a sparkle
        // Base probability modulated by density (higher density = more sparkles)
        let base_probability = 0.3; // Base sparkle rate per second
        let density_factor = self.state.density() * 2.0 + 0.5; // 0.5 to 2.5
        let sparkle_probability = base_probability * density_factor * dt;

        if rand::random::<f64>() < sparkle_probability {
            // TODO: For deterministic mode: use injected RNG instead of rand::random()
            // Generate a sparkle impulse
            // Strength based on current energy level
            let strength = 0.5 + self.state.energy() * 0.5; // 0.5 to 1.0
            self.state.set_sparkle_impulse(strength);
            tracing::debug!("Sparkle generated with strength {:.3}", strength);
        }
    }

    /// Retrieves the current world state snapshot.
    pub fn get_snapshot(&self) -> WorldSnapshot {
        WorldSnapshot::from_world_state(&self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn test_tick_event_bounds() {
        let mut engine = WorldEngine::new();
        let _rng = StdRng::from_seed([0; 32]);
        for _ in 0..100 {
            engine.apply(Event::Tick { dt: 0.05 });
        }
        let snapshot = engine.get_snapshot();
        assert!((0.0..=1.0).contains(&snapshot.density()));
        assert!((0.0..=1.0).contains(&snapshot.rhythm()));
        assert!((0.0..=1.0).contains(&snapshot.tension()));
        assert!((0.0..=1.0).contains(&snapshot.energy()));
        assert!((0.0..=1.0).contains(&snapshot.warmth()));
    }

    #[test]
    fn test_trigger_pulse() {
        let mut engine = WorldEngine::new();
        let intensity = 0.3;
        engine.apply(Event::Trigger {
            kind: TriggerKind::Pulse,
            intensity,
        });
        let snapshot = engine.get_snapshot();
        assert_eq!(snapshot.energy(), 0.5 + intensity); // 0.8
        assert_eq!(snapshot.tension(), 0.5 + 0.1 * intensity); // 0.53
        // Others unchanged
        assert_eq!(snapshot.density(), 0.5);
        assert_eq!(snapshot.rhythm(), 0.5);
        assert_eq!(snapshot.warmth(), 0.5);
    }

    #[test]
    fn test_trigger_stir() {
        let mut engine = WorldEngine::new();
        let intensity = 0.2;
        engine.apply(Event::Trigger {
            kind: TriggerKind::Stir,
            intensity,
        });
        let snapshot = engine.get_snapshot();
        assert_eq!(snapshot.density(), 0.5 + intensity); // 0.7
        assert_eq!(snapshot.tension(), 0.5 + 0.1 * intensity); // 0.52
        // Others unchanged
        assert_eq!(snapshot.energy(), 0.5);
        assert_eq!(snapshot.rhythm(), 0.5);
        assert_eq!(snapshot.warmth(), 0.5);
    }

    #[test]
    fn test_trigger_calm() {
        let mut engine = WorldEngine::new();
        let intensity = 0.4;
        engine.apply(Event::Trigger {
            kind: TriggerKind::Calm,
            intensity,
        });
        let snapshot = engine.get_snapshot();
        assert_eq!(snapshot.tension(), 0.5 - intensity); // 0.1
        assert_eq!(snapshot.density(), 0.5 - 0.1 * intensity); // 0.46
        // Others unchanged
        assert_eq!(snapshot.energy(), 0.5);
        assert_eq!(snapshot.rhythm(), 0.5);
        assert_eq!(snapshot.warmth(), 0.5);
    }

    #[test]
    fn test_trigger_heat() {
        let mut engine = WorldEngine::new();
        let intensity = 0.25;
        engine.apply(Event::Trigger {
            kind: TriggerKind::Heat,
            intensity,
        });
        let snapshot = engine.get_snapshot();
        assert_eq!(snapshot.warmth(), 0.5 + intensity); // 0.75
        assert_eq!(snapshot.energy(), 0.5 + 0.1 * intensity); // 0.525
        // Others unchanged
        assert_eq!(snapshot.density(), 0.5);
        assert_eq!(snapshot.rhythm(), 0.5);
        assert_eq!(snapshot.tension(), 0.5);
    }

    #[test]
    fn test_trigger_tense() {
        let mut engine = WorldEngine::new();
        let intensity = 0.6;
        engine.apply(Event::Trigger {
            kind: TriggerKind::Tense,
            intensity,
        });
        let snapshot = engine.get_snapshot();
        assert_eq!(snapshot.tension(), (0.5 + intensity).min(1.0)); // 1.1 clamped to 1.0
        // Others unchanged
        assert_eq!(snapshot.density(), 0.5);
        assert_eq!(snapshot.rhythm(), 0.5);
        assert_eq!(snapshot.energy(), 0.5);
        assert_eq!(snapshot.warmth(), 0.5);
    }

    #[test]
    fn test_trigger_bounds_clamping() {
        let mut engine = WorldEngine::new();
        // Apply high intensity to test clamping
        engine.apply(Event::Trigger {
            kind: TriggerKind::Pulse,
            intensity: 2.0, // Should clamp energy to 1.0
        });
        let snapshot = engine.get_snapshot();
        assert_eq!(snapshot.energy(), 1.0);
        assert_eq!(snapshot.tension(), 0.5 + 0.1 * 2.0); // 0.7
    }

    #[test]
    fn test_perform_pulse() {
        let mut engine = WorldEngine::new();
        let intensity = 0.3;
        engine.apply(Event::Perform(PerformAction::Pulse { intensity }));
        let snapshot = engine.get_snapshot();
        assert_eq!(snapshot.energy(), 0.5 + intensity); // 0.8
        assert_eq!(snapshot.tension(), 0.5 + 0.1 * intensity); // 0.53
        // Others unchanged
        assert_eq!(snapshot.density(), 0.5);
        assert_eq!(snapshot.rhythm(), 0.5);
        assert_eq!(snapshot.warmth(), 0.5);
    }

    #[test]
    fn test_perform_stir() {
        let mut engine = WorldEngine::new();
        let intensity = 0.2;
        engine.apply(Event::Perform(PerformAction::Stir { intensity }));
        let snapshot = engine.get_snapshot();
        assert_eq!(snapshot.density(), 0.5 + intensity); // 0.7
        assert_eq!(snapshot.tension(), 0.5 + 0.1 * intensity); // 0.52
        // Others unchanged
        assert_eq!(snapshot.energy(), 0.5);
        assert_eq!(snapshot.rhythm(), 0.5);
        assert_eq!(snapshot.warmth(), 0.5);
    }

    #[test]
    fn test_perform_calm() {
        let mut engine = WorldEngine::new();
        let intensity = 0.4;
        engine.apply(Event::Perform(PerformAction::Calm { intensity }));
        let snapshot = engine.get_snapshot();
        assert_eq!(snapshot.tension(), 0.5 - intensity); // 0.1
        assert_eq!(snapshot.density(), 0.5 - 0.1 * intensity); // 0.46
        // Others unchanged
        assert_eq!(snapshot.energy(), 0.5);
        assert_eq!(snapshot.rhythm(), 0.5);
        assert_eq!(snapshot.warmth(), 0.5);
    }

    #[test]
    fn test_perform_scene() {
        let mut engine = WorldEngine::new();
        engine.apply(Event::Perform(PerformAction::Scene {
            name: "sunrise".to_string(),
        }));
        // Scene changes are logged but don't affect state yet
        let snapshot = engine.get_snapshot();
        assert_eq!(snapshot.density(), 0.5);
        assert_eq!(snapshot.rhythm(), 0.5);
        assert_eq!(snapshot.tension(), 0.5);
        assert_eq!(snapshot.energy(), 0.5);
        assert_eq!(snapshot.warmth(), 0.5);
    }
}
