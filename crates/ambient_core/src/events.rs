//! Defines the events that can occur in the world.

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Event {
    Tick { dt: f64 },
    Trigger { kind: TriggerKind, intensity: f64 },
    Perform(PerformAction),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TriggerKind {
    Pulse,
    Stir,
    Calm,
    Heat,
    Tense,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PerformAction {
    Pulse { intensity: f64 },
    Stir { intensity: f64 },
    Calm { intensity: f64 },
    Heat { intensity: f64 },
    Tense { intensity: f64 },
    Scene { name: String },
    Freeze { seconds: f64 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_event_tick_serialization() {
        let event = Event::Tick { dt: 1.5 };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_event_trigger_serialization() {
        let event = Event::Trigger {
            kind: TriggerKind::Pulse,
            intensity: 0.8,
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_event_perform_serialization() {
        let event = Event::Perform(PerformAction::Pulse { intensity: 0.7 });
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);

        let scene_event = Event::Perform(PerformAction::Scene {
            name: "sunrise".to_string(),
        });
        let json = serde_json::to_string(&scene_event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(scene_event, deserialized);
    }

    #[test]
    fn test_trigger_kind_serialization() {
        let kinds = vec![
            TriggerKind::Pulse,
            TriggerKind::Stir,
            TriggerKind::Calm,
            TriggerKind::Heat,
            TriggerKind::Tense,
        ];
        for kind in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            let deserialized: TriggerKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, deserialized);
        }
    }
}
