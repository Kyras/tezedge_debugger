// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use bytes::Buf;
use tracing::{warn, error, field::{display, debug}};
use crypto::{
    crypto_box::{PrecomputedKey, decrypt},
    nonce::Nonce,
};
use tezos_encoding::binary_reader::BinaryReaderError;
use tezos_messages::p2p::{
    encoding::{
        metadata::MetadataMessage,
        peer::PeerMessageResponse,
    },
    binary_message::{BinaryMessage, BinaryChunk},
};
use std::convert::TryFrom;
use crate::messages::prelude::*;
use crate::storage::MessageStore;

/// Message decrypter
pub struct P2pDecrypter {
    precomputed_key: PrecomputedKey,
    nonce: Nonce,
    metadata: bool,
    ack: bool,
    inc_buf: Vec<u8>,
    dec_buf: Vec<u8>,
    input_remaining: usize,
    store: MessageStore,

}

impl P2pDecrypter {
    /// Create new decrypter from precomputed key and nonce
    pub fn new(precomputed_key: PrecomputedKey, nonce: Nonce, store: MessageStore) -> Self {
        Self {
            precomputed_key,
            nonce,
            store,
            metadata: false,
            ack: false,
            inc_buf: Default::default(),
            dec_buf: Default::default(),
            input_remaining: 0,
        }
    }

    /// Try to decrypt message
    pub fn recv_msg(&mut self, enc: &Packet, incoming: bool) -> Option<(Vec<PeerMessage>, Vec<u8>)> {
        if enc.has_payload() {
            self.inc_buf.extend_from_slice(&enc.payload());

            if self.inc_buf.len() > 2 {
                if let Some(decrypted) = self.try_decrypt(incoming) {
                    let raw = decrypted.clone();
                    self.store.stat().decipher_data(decrypted.len());
                    return self.try_deserialize(decrypted).map(|m| (m, raw));
                }
            }
        }
        None
    }

    /// Try to decrypt current buffer
    fn try_decrypt(&mut self, incoming: bool) -> Option<Vec<u8>> {
        // Read message size
        let len = (&self.inc_buf[0..2]).get_u16() as usize;

        // Decrypt only if there is enough data in buffer to decrypt
        if self.inc_buf[2..].len() >= len {
            let chunk = match BinaryChunk::try_from(self.inc_buf[0..len + 2].to_vec()) {
                Ok(chunk) => {
                    chunk
                }
                Err(e) => {
                    error!(error = display(&e), "failed to load binary chunk");
                    return None;
                }
            };

            self.inc_buf.drain(0..len + 2);
            let content = chunk.content();
            let nonce = &self.nonce_fetch();
            let pck = &self.precomputed_key;
            // Try actual decrypt
            match decrypt(content, nonce, pck) {
                Ok(msg) => {
                    // Move nonce iff the decryption succeeds
                    self.nonce_increment();
                    Some(msg)
                }
                Err(err) => {
                    tracing::info!(
                        err = debug(&err),
                        data = debug(&content),
                        nonce = debug(&nonce),
                        pck = display(&hex::encode(pck.as_ref().as_ref())),
                        incoming,
                        "failed to decrypt message",
                    );
                    None
                }
            }
        } else {
            None
        }
    }

    /// Try to deserialize decrypted message
    fn try_deserialize(&mut self, mut msg: Vec<u8>) -> Option<Vec<PeerMessage>> {
        if !self.metadata {
            self.try_deserialize_meta(&mut msg)
        } else if !self.ack {
            self.try_deserialize_ack(&mut msg)
        } else {
            self.try_deserialize_p2p(&mut msg)
        }
    }

    /// Try deserialize acknowledgment message only
    fn try_deserialize_ack(&mut self, msg: &mut Vec<u8>) -> Option<Vec<PeerMessage>> {
        use crate::messages::ack_message::AckMessage;
        self.input_remaining = self.input_remaining.saturating_sub(msg.len());
        self.dec_buf.append(msg);

        if self.input_remaining == 0 {
            loop {
                match AckMessage::from_bytes(self.dec_buf.clone()) {
                    Ok(msg) => {
                        self.dec_buf.clear();
                        self.ack = true;
                        return Some(vec![msg.into()]);
                    }
                    Err(BinaryReaderError::Underflow { bytes }) => {
                        self.input_remaining += bytes;
                        return None;
                    }
                    Err(BinaryReaderError::Overflow { bytes }) => {
                        self.dec_buf.drain(self.dec_buf.len() - bytes..);
                    }
                    Err(e) => {
                        warn!(data = debug(&self.dec_buf), error = display(&e), "failed to deserialize message");
                        return None;
                    }
                }
            }
        } else { None }
    }

    /// Try deserialize metadata message only
    fn try_deserialize_meta(&mut self, msg: &mut Vec<u8>) -> Option<Vec<PeerMessage>> {
        self.input_remaining = self.input_remaining.saturating_sub(msg.len());
        self.dec_buf.append(msg);

        if self.input_remaining == 0 {
            loop {
                match MetadataMessage::from_bytes(self.dec_buf.clone()) {
                    Ok(msg) => {
                        self.dec_buf.clear();
                        self.metadata = true;
                        return Some(vec![msg.into()]);
                    }
                    Err(BinaryReaderError::Underflow { bytes }) => {
                        self.input_remaining += bytes;
                        return None;
                    }
                    Err(BinaryReaderError::Overflow { bytes }) => {
                        self.dec_buf.drain(self.dec_buf.len() - bytes..);
                    }
                    Err(e) => {
                        warn!(data = debug(&self.dec_buf), error = display(&e), "failed to deserialize message");
                        return None;
                    }
                }
            }
        } else { None }
    }

    /// Try deserialize rest of P2P messages
    fn try_deserialize_p2p(&mut self, msg: &mut Vec<u8>) -> Option<Vec<PeerMessage>> {
        self.input_remaining = self.input_remaining.saturating_sub(msg.len());
        self.dec_buf.append(msg);

        if self.input_remaining == 0 {
            loop {
                match PeerMessageResponse::from_bytes(self.dec_buf.clone()) {
                    Ok(msg) => {
                        self.dec_buf.clear();
                        return if msg.messages().len() == 0 {
                            None
                        } else {
                            // msg.messages().iter(|x| x.into()).collect()
                            Some(msg.messages().iter().map(|x| x.clone().into()).collect())
                        };
                    }
                    Err(BinaryReaderError::Underflow { bytes }) => {
                        self.input_remaining += bytes;
                        return None;
                    }
                    Err(BinaryReaderError::Overflow { bytes }) => {
                        self.dec_buf.drain(self.dec_buf.len() - bytes..);
                    }
                    Err(e) => {
                        warn!(error = display(&e), "failed to deserialize message");
                        return None;
                    }
                }
            }
        } else { None }
    }

    #[inline]
    #[allow(dead_code)]
    /// Increment internal nonce and return previous value
    fn nonce_fetch_increment(&mut self) -> Nonce {
        let incremented = self.nonce.increment();
        std::mem::replace(&mut self.nonce, incremented)
    }

    #[inline]
    /// Return internal nonce value
    fn nonce_fetch(&self) -> Nonce {
        self.nonce.clone()
    }

    /// Increment internal nonce value
    fn nonce_increment(&mut self) {
        self.nonce = self.nonce.increment();
    }
}