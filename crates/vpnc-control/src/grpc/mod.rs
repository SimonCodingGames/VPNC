//! gRPC services for nodes and clients.

pub mod node;

pub use node::{NodeGrpcService, NodeRuntimeState};

/// Generated protobuf types.
pub mod pb {
    pub mod node {
        tonic::include_proto!("vpnc.node.v1");
    }
    pub mod client {
        tonic::include_proto!("vpnc.client.v1");
    }
}
