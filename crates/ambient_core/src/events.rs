//! Defines the events that can occur in the world.

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Event {
    Tick { dt: f64 },
    Trigger { kind: TriggerKind, intensity: f64 },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TriggerKind {
    Pulse,
    Stir,
    Calm,
    Heat,
    Tense,
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
