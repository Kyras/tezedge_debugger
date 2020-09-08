// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use tokio::sync::mpsc::{UnboundedSender, unbounded_channel, UnboundedReceiver};
use tracing::{trace, info, error, field::{display, debug}};
use failure::Error;
use crypto::{
    crypto_box::precompute,
    nonce::{NoncePair, generate_nonces},
};
use std::{
    convert::TryFrom,
    net::{SocketAddr, IpAddr},
};
use tezos_messages::p2p::{
    binary_message::{BinaryChunk},
    encoding::prelude::*,
};
use crate::{
    utility::prelude::*,
    messages::prelude::*,
    system::{SystemSettings, orchestrator::CONNECTIONS},
    storage::MessageStore,
};
use tezos_messages::p2p::binary_message::BinaryMessage;
use crate::system::orchestrator::ConnectionState;

/// P2P Message parser. Aggregates data, deciphers and deserializes.
/// Deserialized data are send to the primary data processor.
struct Parser {
    pub initializer: SocketAddr,
    receiver: UnboundedReceiver<Packet>,
    processor_sender: UnboundedSender<P2pMessage>,
    encryption: ParserEncryption,
    state: ParserState,
}

impl Parser {
    /// Create new parser for specific remote address
    fn new(initializer: SocketAddr, receiver: UnboundedReceiver<Packet>, processor_sender: UnboundedSender<P2pMessage>, settings: SystemSettings) -> Self {
        if let Ok(mut lock) = CONNECTIONS.write() {
            lock.insert(initializer, None);
        }

        Self {
            initializer,
            receiver,
            processor_sender,
            encryption: ParserEncryption::new(initializer, settings.local_address, settings.identity, settings.storage),
            state: ParserState::Unencrypted,
        }
    }

    /// Wait until new message is received, and parse it.
    async fn parse_next(&mut self) -> bool {
        match self.receiver.recv().await {
            Some(packet) => {
                trace!(process_length = packet.ip_buffer().len(), "processing packet");
                self.parse(packet).await
            }
            None => {
                error!("p2p parser channel closed abruptly");
                false
            }
        }
    }

    /// Parse TCP packet. Returns a flag, describing, if parser
    /// are done and should be closed.
    async fn parse(&mut self, packet: Packet) -> bool {
        // If packet is closing, process last buffers and close parser
        let finish = !packet.is_closing();
        if packet.has_payload() {
            // Decide, how to handle message, determined by the inner state
            let p2p_msg = match self.state {
                ParserState::Unencrypted => self.parse_unencrypted(packet).await,
                ParserState::Encrypted => self.parse_encrypted(packet).await,
                _ => { return true; }
            };

            // If internal parsers were able to deserialize message. Send it to the processor
            if let Some(p2p_msg) = p2p_msg {
                if let Err(err) = self.processor_sender.send(p2p_msg) {
                    error!(error = display(&err), "processor channel closed abruptly");
                }
            }
        }
        finish
    }

    /// Parse unencrypted - ConnectionMessage
    async fn parse_unencrypted(&mut self, packet: Packet) -> Option<P2pMessage> {
        match self.encryption.process_unencrypted(packet) {
            Ok(result) => {
                if self.encryption.is_initialized() {
                    self.state = ParserState::Encrypted;
                }
                result
            }
            Err(err) => {
                trace!(addr = display(self.initializer), error = display(&err), "is not valid tezos p2p connection");
                self.state = ParserState::Irrelevant;
                None
            }
        }
    }

    /// Parse encrypted message
    async fn parse_encrypted(&mut self, packet: Packet) -> Option<P2pMessage> {
        if !self.encryption.is_initialized() {
            self.parse_unencrypted(packet).await
        } else {
            match self.encryption.process_encrypted(packet) {
                Ok(result) => result,
                Err(err) => {
                    info!(addr = display(self.initializer), error = display(&err), "received invalid message");
                    self.state = ParserState::Irrelevant;
                    None
                }
            }
        }
    }
}

impl Drop for Parser {
    fn drop(&mut self) {
        if let Ok(mut lock) = CONNECTIONS.write() {
            if let None = lock.remove(&self.initializer) {
                error!("connection state was not inserted");
            }
        }
    }
}

/// Spawn new p2p parser, returning channel to send packets for processing
pub fn spawn_p2p_parser(initializer: SocketAddr, processor_sender: UnboundedSender<P2pMessage>, settings: SystemSettings) -> UnboundedSender<Packet> {
    let (sender, receiver) = unbounded_channel::<Packet>();
    tokio::spawn(async move {
        let mut parser = Parser::new(initializer, receiver, processor_sender, settings.clone());
        while parser.parse_next().await {
            trace!(addr = display(initializer), "parsed new message");
        }
    });
    sender
}

/// States, in which parser operates
enum ParserState {
    // Nodes did not exchanged Connection messages yet
    Unencrypted,
    // Nodes did exchanged connection messages
    Encrypted,
    // Is not connection containing tezos p2p communication, ignore it
    Irrelevant,
}

/// Structure driving P2P Encryption and decrypts messages
pub struct ParserEncryption {
    incoming: bool,
    initializer: SocketAddr,
    local_address: IpAddr,
    identity: Identity,
    store: MessageStore,
    first_connection_message: Option<(SocketAddr, ConnectionMessage)>,
    second_connection_message: Option<(SocketAddr, ConnectionMessage)>,
    incoming_decrypter: Option<P2pDecrypter>,
    outgoing_decrypter: Option<P2pDecrypter>,
}

impl ParserEncryption {
    /// Create new not-yet initialized decrypter
    pub fn new(initializer: SocketAddr, local_address: IpAddr, identity: Identity, store: MessageStore) -> Self {
        Self {
            initializer,
            local_address,
            identity,
            store,
            incoming: false,
            first_connection_message: None,
            second_connection_message: None,
            incoming_decrypter: None,
            outgoing_decrypter: None,
        }
    }

    /// Check if decrypter was already "upgraded" to encrypted state
    pub fn is_initialized(&self) -> bool {
        self.incoming_decrypter.is_some() && self.outgoing_decrypter.is_some()
    }

    /// Extract remote address from given packet
    pub fn extract_remote(&self, packet: &Packet) -> (SocketAddr, bool) {
        let incoming = self.local_address == packet.destination_address().ip();
        (if incoming { packet.source_address() } else { packet.destination_address() }, incoming)
    }

    /// Process unencrypted message. If both connection messages has been exchanged. Decrypter
    /// will automatically upgrade to encrypted state
    pub fn process_unencrypted(&mut self, packet: Packet) -> Result<Option<P2pMessage>, Error> {
        if self.is_initialized() {
            self.process_encrypted(packet)
        } else {
            let chunk = BinaryChunk::try_from(packet.payload().to_vec())?;
            let raw_bytes = chunk.content().to_vec();
            let conn_msg = ConnectionMessage::try_from(chunk)?;
            let mut upgrade = false;
            let (remote, incoming) = self.extract_remote(&packet);

            let place = if let Some(_) = self.first_connection_message {
                if packet.source_address() == self.initializer {
                    info!(
                        initializer = display(self.initializer.clone()),
                        src = display(packet.source_address()),
                        dst = display(packet.destination_address()),
                        "received duplicate connection message"
                    );
                    return Ok(None);
                } else {
                    upgrade = true;
                    &mut self.second_connection_message
                }
            } else {
                &mut self.first_connection_message
            };
            *place = Some((packet.source_address(), conn_msg.clone()));

            if upgrade {
                self.upgrade()?;
            }

            Ok(Some(P2pMessage::new(remote, incoming, vec![conn_msg], raw_bytes)))
        }
    }

    /// Process encrypted message
    pub fn process_encrypted(&mut self, packet: Packet) -> Result<Option<P2pMessage>, Error> {
        let (remote, incoming) = self.extract_remote(&packet);

        let decrypter = if !incoming {
            &mut self.incoming_decrypter
        } else {
            &mut self.outgoing_decrypter
        };

        Ok(decrypter.as_mut()
            .map(|decrypter| {
                tracing::trace!(incoming, "trying to decrypt message");
                decrypter.recv_msg(&packet, incoming)
            }).flatten()
            .map(|(msgs, raw_bytes)| {
                P2pMessage::new(remote, incoming, msgs, raw_bytes)
            }))
    }

    /// Upgrade this decrypter to work on encrypted communication
    pub fn upgrade(&mut self) -> Result<(), Error> {
        if let (Some((first_source, first)), Some((_, second))) = (&self.first_connection_message, &self.second_connection_message) {
            let incoming = first_source.ip() != self.local_address;
            self.incoming = incoming;
            let (sent, received) = if incoming {
                (second, first)
            } else {
                (first, second)
            };

            let sent_data = BinaryChunk::from_content(&sent.as_bytes()?)?;
            let recv_data = BinaryChunk::from_content(&received.as_bytes()?)?;

            let NoncePair { remote, local } = generate_nonces(
                &sent_data.raw(),
                &recv_data.raw(),
                incoming,
            );

            let precomputed_key = precompute(
                &hex::encode(&received.public_key),
                &self.identity.secret_key,
            )?;

            info!(
                sent=debug(sent_data.raw()),
                recv=debug(recv_data.raw()),
                local=debug(&local),
                remote=debug(&remote),
                pk=display(hex::encode(precomputed_key.as_ref().as_ref())),
                "upgrade",
            );

            self.incoming_decrypter = Some(P2pDecrypter::new(precomputed_key.clone(), local, self.store.clone()));
            self.outgoing_decrypter = Some(P2pDecrypter::new(precomputed_key.clone(), remote, self.store.clone()));

            if let Ok(mut lock) = CONNECTIONS.write() {
                lock.insert(self.initializer, Some(ConnectionState {
                    incoming,
                    peer_id: hex::encode(&received.public_key),
                }));
            }

            info!(
                initializer = display(self.initializer),
                "connection upgraded to encrypted"
            );
            Ok(())
        } else {
            unreachable!()
        }
    }
}