use tokio::net::tcp::{ OwnedReadHalf, OwnedWriteHalf };
use tokio::net::{ TcpListener, TcpStream };
use tokio::io::{ AsyncBufReadExt, AsyncWriteExt, BufReader };
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::net::SocketAddr;

type ClientMap = Arc<Mutex<HashMap<SocketAddr, OwnedWriteHalf>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Sunucu: {:?}", listener);

    let clients: ClientMap = Arc::new(Mutex::new(HashMap::new()));
    loop {
        match listener.accept().await {
            Ok((mut stream, addr)) => {
                println!("Yeni bağlantı kabul edildi: {}", addr);
                let clients_clone = Arc::clone(&clients);

                if let Err(e) = stream.write_all("Sohbete hoş gelidiniz.".as_bytes()).await {
                    println!("{} adresine mesaj gönderilemedi: {}", addr, e);
                }

                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, addr, clients_clone).await {
                        eprintln!("{} ile bağlantı kurulamadı: {}", addr, e);
                    }
                });
            }
            Err(err) => eprintln!("Bağlantı kabul edilemedi: {}", err),
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    clients: ClientMap
) -> Result<(), Box<dyn Error>> {
    let (reader, writer) = stream.into_split();
    let mut reader: BufReader<OwnedReadHalf> = BufReader::new(reader);

    {
        clients.lock().await.insert(addr, writer);
        println!("{} listeye eklendi.", addr);
    }

    broadcast_message(&addr, "[SYSTEM]", &format!("{addr} katıldı."), &clients).await;

    let mut line = String::new();
    loop {
        match reader.read_line(&mut line).await {
            Ok(0) => {
                println!("Bağlantı kapandı: {}", addr);
                break;
            }
            Ok(_) => {
                let message = line.trim();
                if !message.is_empty() {
                    println!("{} : {}", addr, message);
                    broadcast_message(&addr, &addr.to_string(), message, &clients).await;
                }
                line.clear();
            }
            Err(e) => {
                eprintln!("{} adresinden mesaj okunamadı: {}", addr, e);
                break;
            }
        }
    }

    {
        clients.lock().await.remove(&addr);
        println!("{} listeden çıkarıldı.", addr);
    }

    broadcast_message(&addr, "[SYSTEM]", &format!("{} ayrıldı.", addr), &clients).await;
    Ok(())
}

async fn broadcast_message(
    sender_addr: &SocketAddr,
    sender_name: &str,
    message: &str,
    clients: &ClientMap
) {
    let formatted_message = format!("{}: {}", sender_name, message);

    for (addr, writer) in clients.lock().await.iter_mut() {
        if *addr != *sender_addr {
            if let Err(e) = writer.write_all(formatted_message.as_bytes()).await {
                eprintln!("{} adresine mesaj gönderilemedi: {}", addr, e);
            }
        }
    }
}
