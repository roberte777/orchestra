pub mod field_selector;
pub use field_selector::{FieldCondition, FieldSelector, FieldSelectorBuilder, Operator};
use std::{
    fmt::{Display, Formatter},
    sync::Arc,
};

use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub enum EventType {
    Added,
    Modified,
    Deleted,
}

#[derive(Clone, Debug)]
pub struct Event<T> {
    pub event_type: EventType,
    pub resource: T,
}

impl Display for EventType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let event_type_str = match self {
            EventType::Added => "added",
            EventType::Modified => "modified",
            EventType::Deleted => "deleted",
        };
        write!(f, "{}", event_type_str)
    }
}

struct Subscriber<T> {
    field_conditions: Vec<FieldCondition>,
    sender: mpsc::UnboundedSender<Arc<Event<T>>>,
}

pub struct Watcher<T>
where
    T: Watchable,
{
    subscribers: Vec<Subscriber<T>>,
}

impl<T> Watcher<T>
where
    T: Watchable,
{
    pub fn new() -> Self {
        Self {
            subscribers: Vec::new(),
        }
    }

    pub fn subscribe(
        &mut self,
        field_selector: FieldSelector,
    ) -> mpsc::UnboundedReceiver<Arc<Event<T>>> {
        let field_conditions = field_selector.conditions;
        let (sender, receiver) = mpsc::unbounded_channel();
        self.subscribers.push(Subscriber {
            field_conditions,
            sender,
        });
        receiver
    }

    pub fn notify(&mut self, event: Event<T>) {
        let event = Arc::new(event);
        self.subscribers.retain(|subscriber| {
            if subscriber.sender.is_closed() {
                // Remove the subscriber if the sender is closed
                false
            } else if Self::apply_field_conditions(&event.resource, &subscriber.field_conditions) {
                // Try to send the event; remove the subscriber if send fails
                subscriber.sender.send(event.clone()).is_ok()
            } else {
                // Field conditions do not match; keep the subscriber
                true
            }
        });
    }
    fn apply_field_conditions(resource: &T, conditions: &[FieldCondition]) -> bool
    where
        T: Watchable,
    {
        for condition in conditions {
            let field_value = resource.get_field_value(&condition.field_name);
            match (&condition.operator, field_value) {
                (Operator::Equals, Some(value)) => {
                    if value != condition.value {
                        return false;
                    }
                }
                (Operator::NotEquals, Some(value)) => {
                    if value == condition.value {
                        return false;
                    }
                }
                _ => {
                    // Field not found or other issue
                    return false;
                }
            }
        }
        true
    }
}

impl<T> Default for Watcher<T>
where
    T: Watchable + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

pub trait Watchable {
    fn get_field_value(&self, field_name: &str) -> Option<String>;
}

#[cfg(test)]
mod test {
    use mpsc::error::TryRecvError;

    use super::*;

    #[derive(Clone)]
    struct MockResource {
        foo: String,
    }

    impl Watchable for MockResource {
        fn get_field_value(&self, field_name: &str) -> Option<String> {
            match field_name {
                "foo" => Some(self.foo.clone()),
                _ => None,
            }
        }
    }

    #[tokio::test]
    async fn test_watcher() {
        let mut watcher = Watcher::<MockResource>::new();

        let field_selector = FieldSelectorBuilder::new().eq("foo", "bar").build();
        let mut receiver = watcher.subscribe(field_selector);

        let resource1 = MockResource {
            foo: "bar".to_string(),
        };

        watcher.notify(Event {
            event_type: EventType::Added,
            resource: resource1,
        });

        let event = receiver.try_recv().expect("Should be able to receive");
        assert_eq!(event.resource.foo, "bar");
    }

    struct NestedResource {
        mock: MockResource,
    }

    impl Watchable for NestedResource {
        fn get_field_value(&self, field_name: &str) -> Option<String> {
            match field_name {
                "mock.foo" => Some(self.mock.foo.clone()),
                _ => None,
            }
        }
    }
    #[tokio::test]
    async fn test_nested_watcher() {
        let mut watcher = Watcher::<NestedResource>::new();

        let field_selector = FieldSelectorBuilder::new().eq("mock.foo", "bar").build();
        let mut receiver = watcher.subscribe(field_selector);

        let resource1 = NestedResource {
            mock: MockResource {
                foo: "bar".to_string(),
            },
        };

        watcher.notify(Event {
            event_type: EventType::Added,
            resource: resource1,
        });

        let event = receiver.try_recv().expect("Should be able to receive");
        assert_eq!(event.resource.mock.foo, "bar");
    }

    #[tokio::test]
    async fn test_negative_nested_watcher() {
        let mut watcher = Watcher::<NestedResource>::new();

        let field_selector = FieldSelectorBuilder::new().eq("foo", "non match").build();
        let mut receiver = watcher.subscribe(field_selector);

        let resource1 = NestedResource {
            mock: MockResource {
                foo: "bar".to_string(),
            },
        };

        watcher.notify(Event {
            event_type: EventType::Added,
            resource: resource1,
        });

        let event = receiver.try_recv();
        assert!(matches!(event, Err(TryRecvError::Empty)));
    }

    #[tokio::test]
    async fn test_no_selector() {
        let mut watcher = Watcher::<NestedResource>::new();

        let field_selector = FieldSelector::default();
        let mut receiver = watcher.subscribe(field_selector);

        let resource1 = NestedResource {
            mock: MockResource {
                foo: "bar".to_string(),
            },
        };

        watcher.notify(Event {
            event_type: EventType::Added,
            resource: resource1,
        });

        let event = receiver.try_recv().expect("Should be able to receive");
        assert_eq!(event.resource.mock.foo, "bar");
    }
}
