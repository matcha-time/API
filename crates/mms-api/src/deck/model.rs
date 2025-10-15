use serde::{Deserialize, Serialize};

/// Deck model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    /// Deck ID
    pub id: u32,
    /// Deck name
    pub name: String,
    /// Deck description
    pub description: String,
    /// Topic ID this deck belongs to
    pub topic_id: u32,
    /// Number of cards in the deck
    pub card_count: u32,
    /// Difficulty level (1-5)
    pub difficulty: u8,
}

/// Get dummy decks for testing
pub fn get_dummy_decks() -> Vec<Deck> {
    vec![
        Deck {
            id: 1,
            name: "Basic Algebra".to_string(),
            description: "Fundamental algebraic concepts and equations".to_string(),
            topic_id: 1, // Mathematics
            card_count: 25,
            difficulty: 2,
        },
        Deck {
            id: 2,
            name: "Calculus Basics".to_string(),
            description: "Introduction to calculus concepts".to_string(),
            topic_id: 1, // Mathematics
            card_count: 30,
            difficulty: 4,
        },
        Deck {
            id: 3,
            name: "Rust Programming".to_string(),
            description: "Rust language fundamentals and syntax".to_string(),
            topic_id: 2, // Programming
            card_count: 40,
            difficulty: 3,
        },
        Deck {
            id: 4,
            name: "JavaScript ES6+".to_string(),
            description: "Modern JavaScript features and syntax".to_string(),
            topic_id: 2, // Programming
            card_count: 35,
            difficulty: 2,
        },
        Deck {
            id: 5,
            name: "World War II".to_string(),
            description: "Key events and figures from World War II".to_string(),
            topic_id: 3, // History
            card_count: 50,
            difficulty: 3,
        },
        Deck {
            id: 6,
            name: "Cell Biology".to_string(),
            description: "Cellular structures and processes".to_string(),
            topic_id: 4, // Biology
            card_count: 45,
            difficulty: 3,
        },
        Deck {
            id: 7,
            name: "Shakespeare's Works".to_string(),
            description: "Famous plays and sonnets by William Shakespeare".to_string(),
            topic_id: 5, // Literature
            card_count: 20,
            difficulty: 4,
        },
    ]
}
