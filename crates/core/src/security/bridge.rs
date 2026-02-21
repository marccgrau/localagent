use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use localgpt_bridge::{BridgeService, BridgeError, BridgeServer};
use localgpt_bridge::peer_identity::{PeerIdentity, get_peer_identity};
use tracing::{info, error};
use tarpc::context;

/// Manages bridge processes and their credentials.
#[derive(Clone)]
pub struct BridgeManager {
    // Simulated credential store. In reality, this would be encrypted on disk.
    credentials: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl BridgeManager {
    pub fn new() -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a bridge with its encrypted credentials.
    pub async fn register_bridge(&self, bridge_id: String, secret: Vec<u8>) {
        let mut creds = self.credentials.write().await;
        creds.insert(bridge_id, secret);
    }

    /// Retrieve credentials if the identity is authorized.
    pub async fn get_credentials_for(&self, bridge_id: &str, identity: &PeerIdentity) -> Result<Vec<u8>, BridgeError> {
        // Verify identity. For now, we trust the bridge_id if the caller matches expected criteria.
        // In a real implementation, we would check if `identity.uid` owns the bridge config,
        // or if the process signature matches (on macOS).
        
        // For Linux/Unix, we might check if identity.uid == current_uid (if running as same user)
        // or a specific dedicated user.
        
        info!("Checking access for bridge: {} from {:?}", bridge_id, identity);
        
        // Simple simulation: Allow if registered.
        let creds = self.credentials.read().await;
        if let Some(secret) = creds.get(bridge_id) {
            Ok(secret.clone())
        } else {
            Err(BridgeError::NotRegistered)
        }
    }

    /// Start the bridge server listening on the given socket path.
    pub async fn serve(self, socket_path: &str) -> anyhow::Result<()> {
        let listener = BridgeServer::bind(socket_path)?;
        let manager = self.clone();
        
        info!("BridgeManager listening on {}", socket_path);

        loop {
            let conn = match listener.accept().await {
                Ok(c) => c,
                Err(e) => {
                    error!("Accept failed: {}", e);
                    continue;
                }
            };
            
            // Verify peer identity
            let identity_result = {
                #[cfg(unix)]
                { get_peer_identity(&conn) }
                #[cfg(windows)]
                { get_peer_identity(&conn) }
            };

            let identity = match identity_result {
                Ok(id) => id,
                Err(e) => {
                    error!("Peer identity verification failed: {}", e);
                    continue;
                }
            };

            info!("Accepted connection from: {:?}", identity);
            
            let handler = ConnectionHandler {
                manager: manager.clone(),
                identity,
            };

            tokio::spawn(async move {
                if let Err(e) = localgpt_bridge::handle_connection(conn, handler).await {
                    error!("Connection handling error: {:?}", e);
                }
            });
        }
    }
}

/// Per-connection handler that implements the BridgeService trait.
#[derive(Clone)]
struct ConnectionHandler {
    manager: BridgeManager,
    identity: PeerIdentity,
}

impl BridgeService for ConnectionHandler {
    async fn get_credentials(self, _: context::Context, bridge_id: String) -> Result<Vec<u8>, BridgeError> {
        self.manager.get_credentials_for(&bridge_id, &self.identity).await
    }
}
