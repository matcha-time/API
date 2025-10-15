use serde::{Deserialize, Serialize};

/// Topic model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    /// Topic ID
    pub id: u32,
    /// Topic name
    pub name: String,
    /// Topic description
    pub description: String,
    /// Topic category
    pub category: String,
}

/// Get dummy topics for testing
pub fn get_dummy_topics() -> Vec<Topic> {
    vec![
        Topic {
            id: 1,
            name: "Mathematics".to_string(),
            description: "Mathematical concepts and problem solving".to_string(),
            category: "Science".to_string(),
        },
        Topic {
            id: 2,
            name: "Programming".to_string(),
            description: "Software development and coding concepts".to_string(),
            category: "Technology".to_string(),
        },
        Topic {
            id: 3,
            name: "History".to_string(),
            description: "Historical events and figures".to_string(),
            category: "Social Studies".to_string(),
        },
        Topic {
            id: 4,
            name: "Biology".to_string(),
            description: "Life sciences and biological concepts".to_string(),
            category: "Science".to_string(),
        },
        Topic {
            id: 5,
            name: "Literature".to_string(),
            description: "Classic and contemporary literature".to_string(),
            category: "Language Arts".to_string(),
        },
    ]
}
