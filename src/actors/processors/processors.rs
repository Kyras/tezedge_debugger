use riker::actors::*;
use std::collections::HashMap;
use crate::utility::{
    p2p_message::P2pMessage,
    http_message::HttpMessage,
};
use crate::storage::Storage;
use crate::actors::processors::p2p_archiver::P2pArchiver;

#[derive(Clone)]
pub struct Processors {
    p2p_processors: HashMap<String, ActorRef<P2pMessage>>,
    rpc_processors: HashMap<String, ActorRef<HttpMessage>>,
    storage: Storage,
}

impl Actor for Processors {
    type Msg = (); // TODO: Add control messages to spawn new processors

    fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        match ctx.actor_of_args::<P2pArchiver, _>("p2p_archiver", self.storage.clone()) {
            Ok(actor) => {
                self.p2p_processors.insert(actor.name().to_string(), actor);
            }
            Err(err) => {
                log::error!("Failed to create p2p_archiver: {}", err);
            }
        }
    }

    fn recv(&mut self, _: &Context<Self::Msg>, _: Self::Msg, _: Sender) {}
}

impl ActorFactoryArgs<Storage> for Processors {
    fn create_args(storage: Storage) -> Self {
        Self {
            p2p_processors: Default::default(),
            rpc_processors: Default::default(),
            storage,
        }
    }
}

