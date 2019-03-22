use tentacle::service::{ProtocolHandle, ProtocolMeta};
use tentacle::{builder::MetaBuilder, ProtocolId};
use tentacle_identify::IdentifyProtocol as InnerIdentifyProtocol;

pub use tentacle_identify::AddrManager as PeerManager;
pub use tentacle_identify::{MisbehaveResult, Misbehavior};

/// Protocol name (handshake)
pub const PROTOCOL_NAME: &str = "identify";

/// Protocol support versions
pub const SUPPORT_VERSIONS: [&str; 1] = ["0.1"];

/// Identify protocol
pub struct IdentifyProtocol {}

impl IdentifyProtocol {
    /// Build an `IdentifyProtocol` instance
    pub fn build<TPeerManager>(id: ProtocolId, peer_mgr: TPeerManager) -> ProtocolMeta
    where
        TPeerManager: PeerManager + 'static,
    {
        let boxed_ident = Box::new(InnerIdentifyProtocol::new(id, peer_mgr));

        MetaBuilder::default()
            .id(id)
            .name(name!(PROTOCOL_NAME))
            .support_versions(support_versions!(SUPPORT_VERSIONS))
            .service_handle(|| ProtocolHandle::Callback(boxed_ident))
            .build()
    }
}
