use std::{
    io::{Read, Write},
    net::TcpStream,
};

use tokio::sync::{mpsc, oneshot};

struct Actor {
    receiver: mpsc::Receiver<ActorMessage>,
    socket: TcpStream,
}

pub enum ActorMessage {
    GetMessage { respond_to: oneshot::Sender<String> },
    SendMessage { msg: String },
}

impl Actor {
    fn new(receiver: mpsc::Receiver<ActorMessage>) -> Actor {
        let socket = TcpStream::connect("127.0.0.1:8080").unwrap();
        Actor { receiver, socket }
    }
    fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::GetMessage { respond_to } => {
                let mut buf = String::new();

                if let Ok(_) = self.socket.read_to_string(&mut buf) {
                    let _ = respond_to.send(buf);
                }
            }
            ActorMessage::SendMessage { mut msg } => {
                if !msg.ends_with("\n") {
                    msg.push('\n');
                }
                let _ = self.socket.write_all(msg.as_bytes());
            }
        }
    }
}

async fn run_my_actor(mut actor: Actor) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg);
    }
}

#[derive(Clone)]
pub struct ActorHandle {
    sender: mpsc::Sender<ActorMessage>,
}

impl ActorHandle {
    pub fn new() -> ActorHandle {
        let (sender, receiver) = mpsc::channel(8);
        let actor = Actor::new(receiver);
        tokio::spawn(run_my_actor(actor));

        ActorHandle { sender }
    }

    pub async fn get_message(&self) -> String {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::GetMessage { respond_to: send };

        let _ = self.sender.send(msg);
        recv.await.expect("Actor has been killed")
    }

    pub async fn send_message(&self, msg: &str) {
        let msg = ActorMessage::SendMessage {
            msg: msg.to_string(),
        };
        let _ = self.sender.send(msg);
    }
}
