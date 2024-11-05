use std::sync::Arc;

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

#[derive(Debug)]
enum Operator {
    Equals,
    NotEquals,
}

#[derive(Debug)]
pub struct FieldCondition {
    field_name: String,
    operator: Operator,
    value: String,
}
#[derive(Debug)]
pub struct FieldSelector {
    conditions: Vec<FieldCondition>,
}

impl FieldSelector {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
        }
    }

    pub fn eq(&mut self, field_name: &str, value: &str) {
        self.conditions.push(FieldCondition {
            field_name: field_name.to_string(),
            operator: Operator::Equals,
            value: value.to_string(),
        });
    }

    pub fn ne(&mut self, field_name: &str, value: &str) {
        self.conditions.push(FieldCondition {
            field_name: field_name.to_string(),
            operator: Operator::NotEquals,
            value: value.to_string(),
        });
    }
    pub fn from_string(field_selector: &str) -> Self {
        let mut conditions = Self::new();
        let selectors = field_selector.split(',');
        for selector in selectors {
            let (field_name, operator, value) = if let Some(pos) = selector.find("!=") {
                (
                    selector[..pos].to_string(),
                    Operator::NotEquals,
                    selector[pos + 2..].to_string(),
                )
            } else if let Some(pos) = selector.find('=') {
                (
                    selector[..pos].to_string(),
                    Operator::Equals,
                    selector[pos + 1..].to_string(),
                )
            } else {
                continue; // Invalid selector, skip
            };

            match operator {
                Operator::NotEquals => conditions.ne(&field_name, &value),
                Operator::Equals => conditions.eq(&field_name, &value),
            };
        }
        conditions
    }
}

impl Default for FieldSelector {
    fn default() -> Self {
        Self::new()
    }
}

struct Subscriber<T> {
    field_conditions: Vec<FieldCondition>,
    sender: mpsc::UnboundedSender<Arc<Event<T>>>,
}

pub struct Watcher<T> {
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
            if Self::apply_field_conditions(&event.resource, &subscriber.field_conditions) {
                subscriber.sender.send(event.clone()).is_ok()
            } else {
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
    fn get_field_value(&self, field_name: &str) -> Option<&str>;
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Clone)]
    struct MockResource {
        foo: String,
    }

    impl Watchable for MockResource {
        fn get_field_value(&self, field_name: &str) -> Option<&str> {
            match field_name {
                "foo" => Some(&self.foo),
                _ => None,
            }
        }
    }

    #[tokio::test]
    async fn test_watcher() {
        let mut watcher = Watcher::<MockResource>::new();

        let mut field_selector = FieldSelector::new();
        field_selector.eq("foo", "bar");
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
}
