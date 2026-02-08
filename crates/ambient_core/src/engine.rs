use crate::events::{Event, TriggerKind};
use crate::world::{WorldSnapshot, WorldState};

/// The engine that updates the world state over time.
/// TODO: Consider adding drift parameter here
pub struct WorldEngine {
    state: WorldState,
}

impl Default for WorldEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldEngine {
    /// Initializes the world engine with a default state.
    pub fn new() -> Self {
        Self {
            state: WorldState::new(),
        }
    }

    /// Apply event.
    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Tick { dt } => {
                self.state.drift(dt, &mut rand::rng());
            }
            Event::Trigger { kind, intensity } => match kind {
                TriggerKind::Pulse => {
                    self.state.set_energy(self.state.energy() + intensity);
                    // TODO: Unhardcode these values
                    self.state
                        .set_tension(self.state.tension() + 0.1 * intensity);
                }
                TriggerKind::Stir => {
                    self.state.set_density(self.state.density() + intensity);
                    self.state
                        .set_tension(self.state.tension() + 0.1 * intensity);
                }
                TriggerKind::Calm => {
                    self.state.set_tension(self.state.tension() - intensity);
                    self.state
                        .set_density(self.state.density() - 0.1 * intensity);
                }
                TriggerKind::Heat => {
                    self.state.set_warmth(self.state.warmth() + intensity);
                    self.state.set_energy(self.state.energy() + 0.1 * intensity);
                }
                TriggerKind::Tense => {
                    self.state.set_tension(self.state.tension() + intensity);
                }
            },
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
}
