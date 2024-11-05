#[derive(Debug)]
pub enum Operator {
    Equals,
    NotEquals,
}

pub struct FieldSelectorBuilder {
    conditions: Vec<FieldCondition>,
}

impl FieldSelectorBuilder {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
        }
    }

    pub fn eq(mut self, field_name: &str, value: &str) -> Self {
        self.conditions.push(FieldCondition {
            field_name: field_name.to_string(),
            operator: Operator::Equals,
            value: value.to_string(),
        });
        self
    }

    pub fn ne(mut self, field_name: &str, value: &str) -> Self {
        self.conditions.push(FieldCondition {
            field_name: field_name.to_string(),
            operator: Operator::NotEquals,
            value: value.to_string(),
        });
        self
    }

    pub fn build(self) -> FieldSelector {
        FieldSelector {
            conditions: self.conditions,
        }
    }
}

impl Default for FieldSelectorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct FieldCondition {
    pub field_name: String,
    pub operator: Operator,
    pub value: String,
}
#[derive(Debug)]
pub struct FieldSelector {
    pub conditions: Vec<FieldCondition>,
}

impl FieldSelector {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
        }
    }

    fn add_eq(&mut self, field_name: &str, value: &str) {
        self.conditions.push(FieldCondition {
            field_name: field_name.to_string(),
            operator: Operator::Equals,
            value: value.to_string(),
        });
    }

    fn add_ne(&mut self, field_name: &str, value: &str) {
        self.conditions.push(FieldCondition {
            field_name: field_name.to_string(),
            operator: Operator::NotEquals,
            value: value.to_string(),
        });
    }

    pub fn builder() -> FieldSelectorBuilder {
        FieldSelectorBuilder::new()
    }

    pub fn from_query(field_selector: &str) -> Self {
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
                Operator::NotEquals => conditions.add_ne(&field_name, &value),
                Operator::Equals => conditions.add_eq(&field_name, &value),
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
