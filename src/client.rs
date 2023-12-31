use std::sync::Arc;

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{tcp::OwnedWriteHalf, TcpStream},
    sync::{mpsc, Mutex},
};

enum TaskConfig {
    Send(String),
}

pub struct Task {
    config: TaskConfig,
}

impl Task {
    pub fn send(msg: &str) -> Task {
        Task {
            config: TaskConfig::Send(msg.to_string()),
        }
    }
}

pub struct TaskSpawner {
    send: mpsc::Sender<Task>,
}

async fn handle_task(socket: Arc<Mutex<OwnedWriteHalf>>, task: Task, tx: mpsc::Sender<String>) {
    let mut socket = socket.lock().await;
    match task.config {
        TaskConfig::Send(mut msg) => {
            if !msg.ends_with("\n") {
                msg.push('\n');
            }

            socket.write_all(msg.as_bytes()).await.unwrap();
            socket.flush().await.unwrap();
        }
    }
}

async fn poll_messages(socket: Arc<Mutex<TcpStream>>, tx: mpsc::Sender<String>) {
    let mut socket = socket.lock().await;
    let (reader, _) = socket.split();
    let mut reader = BufReader::new(reader);

    let mut res = String::new();
    match reader.read_line(&mut res).await {
        Ok(0) => return,
        Ok(_) => {
            tx.send(res).await.unwrap();
        }
        Err(_) => todo!(),
    }
}

impl TaskSpawner {
    pub fn new() -> (TaskSpawner, mpsc::Receiver<String>) {
        let (send, mut recv) = mpsc::channel::<Task>(100);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let (tx, rx) = mpsc::channel(100);

        let socket = rt.block_on(TcpStream::connect("127.0.0.1:8080")).unwrap();
        let (reader, writer) = socket.into_split();
        let mut reader = BufReader::new(reader);
        let writer = Arc::new(Mutex::new(writer));

        std::thread::spawn(move || {
            rt.block_on(async move {
                loop {
                    let mut buf = String::new();
                    tokio::select! {
                        _ = reader.read_line(&mut buf) => {
                            let tx = tx.clone();
                            tx.send(buf).await.unwrap();
                        }
                        task = recv.recv() => {
                            if let Some(task) = task {
                                tokio::spawn(handle_task(writer.clone(), task, tx.clone()));
                            }
                        }
                    }
                }
            });
        });

        (TaskSpawner { send }, rx)
    }

    pub fn spawn_task(&self, task: Task) {
        match self.send.blocking_send(task) {
            Ok(_) => {}
            Err(_) => panic!("The shared runtime has shut down."),
        }
    }
}
