use std::net::SocketAddr;

use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::errors::CodecError;
use crate::packet::{MagicRead, MagicWrite, PackId, SocketAddrRead, SocketAddrWrite};
use crate::read_buf;

/// Request sent before establishing a connection
#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Packet<B: Buf> {
    UnconnectedPing {
        send_timestamp: i64,
        magic: bool,
        client_guid: u64,
    },
    UnconnectedPong {
        send_timestamp: i64,
        server_guid: u64,
        magic: bool,
        data: B,
    },
    OpenConnectionRequest1 {
        magic: bool,
        protocol_version: u8,
        mtu: u16,
    },
    OpenConnectionReply1 {
        magic: bool,
        server_guid: u64,
        use_encryption: bool,
        mtu: u16,
    },
    OpenConnectionRequest2 {
        magic: bool,
        server_address: SocketAddr,
        mtu: u16,
        client_guid: u64,
    },
    OpenConnectionReply2 {
        magic: bool,
        server_guid: u64,
        client_address: SocketAddr,
        mtu: u16,
        encryption_enabled: bool,
    },
    IncompatibleProtocol {
        server_protocol: u8,
        magic: bool,
        server_guid: u64,
    },
    AlreadyConnected {
        magic: bool,
        server_guid: u64,
    },
}

impl<B: Buf> Packet<B> {
    pub(super) fn pack_id(&self) -> PackId {
        match self {
            Packet::UnconnectedPing { .. } => {
                // > [Wiki](https://wiki.vg/Raknet_Protocol) said:
                // > 0x02 is only replied to if there are open connections to the server.
                // It is in unconnected mod, so we just return UnconnectedPing1
                PackId::UnconnectedPing1
            }
            Packet::UnconnectedPong { .. } => PackId::UnconnectedPong,
            Packet::OpenConnectionRequest1 { .. } => PackId::OpenConnectionRequest1,
            Packet::OpenConnectionReply1 { .. } => PackId::OpenConnectionReply1,
            Packet::OpenConnectionRequest2 { .. } => PackId::OpenConnectionRequest2,
            Packet::OpenConnectionReply2 { .. } => PackId::OpenConnectionReply2,
            Packet::IncompatibleProtocol { .. } => PackId::IncompatibleProtocolVersion,
            Packet::AlreadyConnected { .. } => PackId::AlreadyConnected,
        }
    }

    pub(super) fn read_unconnected_ping(buf: &mut BytesMut) -> Self {
        Packet::UnconnectedPing {
            send_timestamp: buf.get_i64(),  // 8
            magic: buf.get_checked_magic(), // 16
            client_guid: buf.get_u64(),     // 8
        }
    }

    pub(super) fn read_open_connection_request1(buf: &mut BytesMut) -> Self {
        Packet::OpenConnectionRequest1 {
            magic: buf.get_checked_magic(), // 16
            protocol_version: buf.get_u8(), // 1
            mtu: buf.get_u16(),             // 2
        }
    }

    pub(super) fn read_open_connection_reply1(buf: &mut BytesMut) -> Self {
        Packet::OpenConnectionReply1 {
            magic: buf.get_checked_magic(),    // 16
            server_guid: buf.get_u64(),        // 8
            use_encryption: buf.get_u8() != 0, // 1
            mtu: buf.get_u16(),                // 2
        }
    }

    pub(super) fn read_open_connection_request2(buf: &mut BytesMut) -> Result<Self, CodecError> {
        Ok(Packet::OpenConnectionRequest2 {
            magic: read_buf!(buf, 16, buf.get_checked_magic()),
            server_address: buf.get_socket_addr()?,
            mtu: read_buf!(buf, 2, buf.get_u16()),
            client_guid: read_buf!(buf, 8, buf.get_u64()),
        })
    }

    pub(super) fn read_open_connection_reply2(buf: &mut BytesMut) -> Result<Self, CodecError> {
        Ok(Packet::OpenConnectionReply2 {
            magic: read_buf!(buf, 16, buf.get_checked_magic()),
            server_guid: read_buf!(buf, 8, buf.get_u64()),
            client_address: buf.get_socket_addr()?,
            mtu: read_buf!(buf, 2, buf.get_u16()),
            encryption_enabled: read_buf!(buf, 1, buf.get_u8() != 0),
        })
    }

    pub(super) fn read_incompatible_protocol(buf: &mut BytesMut) -> Self {
        Packet::IncompatibleProtocol {
            server_protocol: buf.get_u8(),  // 1
            magic: buf.get_checked_magic(), // 16
            server_guid: buf.get_u64(),     // 8
        }
    }

    pub(super) fn read_already_connected(buf: &mut BytesMut) -> Self {
        Packet::AlreadyConnected {
            magic: buf.get_checked_magic(), // 16
            server_guid: buf.get_u64(),     // 8
        }
    }

    pub(super) fn write(self, buf: &mut BytesMut) {
        match self {
            Packet::UnconnectedPing {
                send_timestamp,
                magic: _magic,
                client_guid,
            } => {
                buf.put_i64(send_timestamp);
                buf.put_magic();
                buf.put_u64(client_guid);
            }
            Packet::UnconnectedPong {
                send_timestamp,
                server_guid,
                magic: _magic,
                data,
            } => {
                buf.put_i64(send_timestamp);
                buf.put_u64(server_guid);
                buf.put_magic();
                buf.put(data);
            }
            Packet::OpenConnectionRequest1 {
                magic: _magic,
                protocol_version,
                mtu,
            } => {
                buf.put_magic();
                buf.put_u8(protocol_version);
                buf.put_u16(mtu);
            }
            Packet::OpenConnectionReply1 {
                magic: _magic,
                server_guid,
                use_encryption: _use_encryption,
                mtu,
            } => {
                buf.put_magic();
                buf.put_u64(server_guid);
                buf.put_u8(0);
                buf.put_u16(mtu);
            }
            Packet::OpenConnectionRequest2 {
                magic: _magic,
                server_address,
                mtu,
                client_guid,
            } => {
                buf.put_magic();
                buf.put_socket_addr(server_address);
                buf.put_u16(mtu);
                buf.put_u64(client_guid);
            }
            Packet::OpenConnectionReply2 {
                magic: _magic,
                server_guid,
                client_address,
                mtu,
                encryption_enabled: _encryption_enabled,
            } => {
                buf.put_magic();
                buf.put_u64(server_guid);
                buf.put_socket_addr(client_address);
                buf.put_u16(mtu);
                buf.put_u8(0);
            }
            Packet::IncompatibleProtocol {
                server_protocol,
                magic: _magic,
                server_guid,
            } => {
                buf.put_u8(server_protocol);
                buf.put_magic();
                buf.put_u64(server_guid);
            }
            Packet::AlreadyConnected {
                magic: _magic,
                server_guid,
            } => {
                buf.put_magic();
                buf.put_u64(server_guid);
            }
        }
    }
}

impl Packet<BytesMut> {
    pub(super) fn read_unconnected_pong(buf: &mut BytesMut) -> Self {
        Packet::UnconnectedPong {
            send_timestamp: buf.get_i64(),  // 8
            server_guid: buf.get_u64(),     // 8
            magic: buf.get_checked_magic(), // 16
            data: buf.split(),              // ?
        }
    }

    pub(crate) fn freeze(self) -> Packet<Bytes> {
        match self {
            Packet::UnconnectedPing {
                send_timestamp,
                magic,
                client_guid,
            } => Packet::UnconnectedPing {
                send_timestamp,
                magic,
                client_guid,
            },
            Packet::UnconnectedPong {
                send_timestamp,
                server_guid,
                magic,
                data,
            } => Packet::UnconnectedPong {
                send_timestamp,
                server_guid,
                magic,
                data: data.freeze(),
            },
            Packet::OpenConnectionRequest1 {
                magic,
                protocol_version,
                mtu,
            } => Packet::OpenConnectionRequest1 {
                magic,
                protocol_version,
                mtu,
            },
            Packet::OpenConnectionReply1 {
                magic,
                server_guid,
                use_encryption,
                mtu,
            } => Packet::OpenConnectionReply1 {
                magic,
                server_guid,
                use_encryption,
                mtu,
            },
            Packet::OpenConnectionRequest2 {
                magic,
                server_address,
                mtu,
                client_guid,
            } => Packet::OpenConnectionRequest2 {
                magic,
                server_address,
                mtu,
                client_guid,
            },
            Packet::OpenConnectionReply2 {
                magic,
                server_guid,
                client_address,
                mtu,
                encryption_enabled,
            } => Packet::OpenConnectionReply2 {
                magic,
                server_guid,
                client_address,
                mtu,
                encryption_enabled,
            },
            Packet::IncompatibleProtocol {
                server_protocol,
                magic,
                server_guid,
            } => Packet::IncompatibleProtocol {
                server_protocol,
                magic,
                server_guid,
            },
            Packet::AlreadyConnected { magic, server_guid } => {
                Packet::AlreadyConnected { magic, server_guid }
            }
        }
    }
}
