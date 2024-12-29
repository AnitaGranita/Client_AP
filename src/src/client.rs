use crossbeam::channel::{unbounded, Sender, Receiver};
//let (sender, receiver): (Sender<Packet>, Receiver<Packet>) = unbounded();

type NodeId = u8;

#[derive(Debug, Clone)]
enum NodeType {
    Client,
    Drone,
    Server,
}

#[derive(Debug)]
struct SourceRoutingHeader {
    hop_index: usize,
    hops: Vec<NodeId>,
}

#[derive(Debug)]
enum PacketType {
    MsgFragment(Fragment),
    Ack(Ack),
    Nack(Nack),
    FloodRequest(FloodRequest),
    FloodResponse(FloodResponse),
}

#[derive(Debug)]
struct Packet {
    pack_type: PacketType,
    routing_header: SourceRoutingHeader,
    session_id: u64,
}

#[derive(Debug)]
struct Fragment {
    fragment_index: u64,
    total_n_fragments: u64,
    length: u8,
    data: [u8; 128],
}

#[derive(Debug)]
struct Ack {
    fragment_index: u64,
}

#[derive(Debug)]
struct Nack {
    fragment_index: u64,
    nack_type: NackType,
}

#[derive(Debug)]
enum NackType {
    ErrorInRouting(NodeId),
    DestinationIsDrone,
    Dropped,
    UnexpectedRecipient(NodeId),
}

#[derive(Debug)]
struct FloodRequest {
    flood_id: u64,
    initiator_id: NodeId,
    path_trace: Vec<(NodeId, NodeType)>,
}

#[derive(Debug)]
struct FloodResponse {
    flood_id: u64,
    path_trace: Vec<(NodeId, NodeType)>,
}

struct Client {
    id: NodeId,
    connected_drones: Vec<NodeId>,
    network_topology: Vec<(NodeId, NodeType)>,
}

impl Client {
    fn new(id: NodeId, connected_drones: Vec<NodeId>) -> Self {
        Self {
            id,
            connected_drones,
            network_topology: Vec::new(),
        }
    }

    fn handle_packet(&mut self, packet: Packet) {
        match packet.pack_type {
            PacketType::Ack(ack) => {
                println!("Received Ack for fragment {}", ack.fragment_index);
            }
            PacketType::Nack(nack) => {
                println!("Received Nack: {:?}", nack.nack_type);
            }
            _ => println!("Packet type not handled"),
        }
    }

    // Converts flood_request to a method on Client
    fn flood_request(&self, flood_id: u64, sender: &Sender<Packet>) {
        let request = Packet {
            pack_type: PacketType::FloodRequest(FloodRequest {
                flood_id,
                initiator_id: self.id,
                path_trace: vec![(self.id, NodeType::Client)],
            }),
            routing_header: SourceRoutingHeader {
                hop_index: 1,
                hops: self.connected_drones.clone(),
            },
            session_id: 0,
        };

        /// !!devo capire come risolvere!!
        for (i, drone) in self.connected_drones.iter().enumerate() {
            let request = if i == 0 {
                request
            } else {
                Packet {
                    pack_type: request.pack_type,
                    routing_header: SourceRoutingHeader {
                        hop_index: request.routing_header.hop_index,
                        hops: request.routing_header.hops.clone(),
                    },
                    session_id: request.session_id,
                }
            };

            sender.send(request).expect("Failed to send flood request");
        }
    }

    fn handle_flood_response(&mut self, response: FloodResponse) {
        for path in response.path_trace {
            if !self.network_topology.contains(&path) {
                self.network_topology.push(path);
            }
        }
    }

    fn send_packet(&self, destination: NodeId, data: Vec<u8>, sender: &Sender<Packet>) {
        let route = self.compute_route(destination);
        let fragments = self.fragment_message(data);

        for (index, fragment) in fragments.into_iter().enumerate() {
            let packet = Packet {
                pack_type: PacketType::MsgFragment(fragment),
                routing_header: SourceRoutingHeader {
                    hop_index: 1,
                    hops: route.clone(),
                },
                session_id: self.generate_session_id(),
            };

            sender.send(packet).expect("Failed to send packet");
        }
    }

    fn compute_route(&self, destination: NodeId) -> Vec<NodeId> {
        vec![self.id, destination]
    }

    fn fragment_message(&self, data: Vec<u8>) -> Vec<Fragment> {
        let mut fragments = Vec::new();
        let total_n_fragments = (data.len() as f64 / 128.0).ceil() as u64;

        for i in 0..total_n_fragments {
            let start = (i * 128) as usize;
            let end = usize::min(start + 128, data.len());
            let mut fragment_data: [u8; 128] = [0; 128];
            fragment_data[..end - start].copy_from_slice(&data[start..end]);

            fragments.push(Fragment {
                fragment_index: i,
                total_n_fragments,
                length: (end - start) as u8,
                data: fragment_data,
            });
        }
        fragments
    }
}
