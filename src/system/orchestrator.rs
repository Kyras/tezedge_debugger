use tokio::sync::mpsc::{
    UnboundedSender, unbounded_channel,
};
use tracing::{trace, info, error};
use std::{
    collections::{HashMap, hash_map::Entry},
};
use crate::{
    system::prelude::*,
    messages::tcp_packet::Packet,
};
use crate::system::processor::spawn_processor;

pub fn spawn_packet_orchestrator(settings: SystemSettings) -> UnboundedSender<Packet> {
    let (sender, mut receiver) = unbounded_channel::<Packet>();

    tokio::spawn(async move {
        let mut packet_processors = HashMap::new();
        let message_processor = spawn_processor(settings.clone());
        let settings = settings;
        loop {
            let message_processor = message_processor.clone();
            if let Some(packet) = receiver.recv().await {
                let packet: Packet = packet;
                let remote_addr = if packet.source_addr().ip() == settings.local_address {
                    packet.destination_address()
                } else {
                    packet.source_addr()
                };
                let entry = packet_processors.entry(remote_addr);
                let mut occupied_entry;
                let processor;

                // Packet is closing connection
                if packet.is_closing() {
                    if let Entry::Occupied(entry) = entry {
                        // There is still running processor, this packet will notify it to shut down
                        occupied_entry = entry.remove();
                        processor = &mut occupied_entry;
                    } else {
                        // Processor is already shut down, ignore the packet
                        continue;
                    }
                } else {
                    // Is packet for processing
                    let addr = packet.source_addr();
                    let settings = settings.clone();
                    processor = entry.or_insert_with(move || {
                        // If processor does not exists, create new one
                        info!(addr = display(addr), "spawning p2p parser");
                        spawn_p2p_parser(remote_addr, message_processor.clone(), settings)
                    });
                };

                match processor.send(packet) {
                    Ok(()) => {
                        trace!("sent packet to p2p");
                    }
                    Err(_) => {
                        error!("p2p parser channel closed abruptly");
                    }
                }
            } else {
                error!("packet consuming channel closed unexpectedly");
                break;
            }
        }
    });

    return sender;
}