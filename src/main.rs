use tokio::net::tcp::{ OwnedReadHalf, OwnedWriteHalf };
use tokio::net::{ TcpListener, TcpStream };
use tokio::io::{ AsyncBufReadExt, AsyncWriteExt, BufReader };
use tokio::sync::Mutex;
use std::{ collections::HashMap, error::Error, sync::Arc, net::SocketAddr, time::SystemTime };

type ClientMap = Arc<Mutex<HashMap<SocketAddr, OwnedWriteHalf>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("10.16.4.22:56570").await?;
    println!("Sunucu: {listener:?}");

    let clients: ClientMap = Arc::new(Mutex::new(HashMap::new()));
    loop {
        match listener.accept().await {
            Ok((mut stream, addr)) => {
                println!("New connection accepted: {addr}");
                let clients_clone = Arc::clone(&clients);

                if let Err(e) = stream.write_all("Welcome to chat.\n".as_bytes()).await {
                    println!("Failed to send message to {addr}: {e}");
                }

                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, addr, clients_clone).await {
                        eprintln!("Failed to establish a connection with {addr}: {e}");
                    }
                });
            }
            Err(err) => eprintln!("Connection couldn't be accepted: {err}"),
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
        println!("{addr} added to the list.");
    }

    broadcast_message(&addr, "[SYSTEM]", &format!("{addr} joined."), &clients).await;

    println!(
        "[{}] Client {} added.",
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_millis(),
        addr
    );

    let mut line = String::new();
    loop {
        match reader.read_line(&mut line).await {
            Ok(0) => {
                println!(
                    "[{}] Connection closed by {}",
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis(),
                    addr
                );
                break;
            }
            Ok(n) => {
                let timestamp = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();
                println!("{line}");
                let message = line.trim();
                if !message.is_empty() {
                    println!("[{timestamp}] Received {n} bytes from {addr}: '{message}'");
                    broadcast_message(&addr, &addr.to_string(), message, &clients).await;
                }
                line.clear();
            }
            Err(e) => {
                println!(
                    "[{}] Read error from {}: {}",
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis(),
                    addr,
                    e
                );
                break;
            }
        }
    }

    {
        clients.lock().await.remove(&addr);
        println!("{addr} removed from the list.");
    }

    broadcast_message(&addr, "[SYSTEM]", &format!("{addr} ayrıldı."), &clients).await;

    println!(
        "[{}] Client {} removed.",
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_millis(),
        addr
    );
    Ok(())
}

async fn broadcast_message(
    sender_addr: &SocketAddr,
    sender_name: &str,
    message: &str,
    clients: &ClientMap
) {
    let mut clients_guard = clients.lock().await;
    let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();

    let formatted_message = format!("{sender_name}: {message}\n");
    let message_bytes = formatted_message.as_bytes();

    println!(
        "[{:?}] Broadcasting from {}: '{}' to {} clients",
        timestamp,
        sender_name,
        message,
        clients_guard.len() - 1
    );

    for (addr, writer) in clients_guard.iter_mut() {
        if *addr != *sender_addr {
            let target_addr = *addr;
            println!("[{timestamp:?}] Attempting to send to {target_addr}");
            match writer.write_all(message_bytes).await {
                Ok(_) => {
                    println!("[{timestamp:?}] Successfully sent to {target_addr}"); // Log 3: Başarılı gönderme
                }
                Err(e) => {
                    eprintln!("[{timestamp:?}] FAILED to send to {target_addr}: {e}"); // Log 4: Başarısız gönderme
                    // İsteğe bağlı: Burada istemciyi çıkarmayı düşünebiliriz
                }
            }
        }
    }
}
