use serde::{Deserialize, Serialize};

/// User model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// User ID
    pub id: u32,
    /// User's username
    pub username: String,
    /// User's email
    pub email: String,
    /// User's full name
    pub name: String,
}

/// Get dummy users for testing
pub fn get_dummy_users() -> Vec<User> {
    vec![
        User {
            id: 1,
            username: "john_doe".to_string(),
            email: "john.doe@example.com".to_string(),
            name: "John Doe".to_string(),
        },
        User {
            id: 2,
            username: "jane_smith".to_string(),
            email: "jane.smith@example.com".to_string(),
            name: "Jane Smith".to_string(),
        },
        User {
            id: 3,
            username: "bob_wilson".to_string(),
            email: "bob.wilson@example.com".to_string(),
            name: "Bob Wilson".to_string(),
        },
        User {
            id: 4,
            username: "alice_johnson".to_string(),
            email: "alice.johnson@example.com".to_string(),
            name: "Alice Johnson".to_string(),
        },
        User {
            id: 5,
            username: "charlie_brown".to_string(),
            email: "charlie.brown@example.com".to_string(),
            name: "Charlie Brown".to_string(),
        },
    ]
}
