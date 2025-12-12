use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OidcFlowData {
    pub csrf_token: String,
    pub nonce: String,
    pub pkce_verifier: String,
}
