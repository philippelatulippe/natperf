use std::env;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::io;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::task::JoinSet;

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("server") => listen(args.next().unwrap_or("5555".to_string()).parse().unwrap_or(5555)).await,
        Some("client") => fanout(
            args.next().expect("provide a hostname"),
            args.next().unwrap_or("5555".to_string()).parse().unwrap_or(5555),
            args.next().unwrap_or("100".to_string()).parse().unwrap_or(100), //TODO: this sucks lol
        ).await,
        Some(_) => usage(),
        None => usage()
    }
}

fn usage() -> io::Result<()> {
    let program_name = env::current_exe()?
        .file_name().ok_or(ErrorKind::NotFound)?
        .to_str().ok_or(ErrorKind::NotFound)?
        .to_owned();
    eprintln!("Usage: {} server [port]", program_name);
    eprintln!("       {} client HOSTNAME [port] [max clients]", program_name);
    Ok(())
}

async fn fanout(hostname: String, port: u16, max_clients: u32) -> io::Result<()> {
    let mut join_set = JoinSet::<io::Result<String>>::new();

    let mut interval = tokio::time::interval(Duration::from_millis(200));
    
    for num in 0..max_clients {
        let host = hostname.clone(); // TODO: I know this string will outlive the closure
        join_set.spawn(async move {
            let mut stream = TcpStream::connect((host, port)).await?;

            let (reader, mut _writer) = stream.split();
            let mut line_reader = BufReader::new(reader);
            let mut buf = String::new();

            tokio::time::sleep(Duration::from_secs(10)).await;
            
            match line_reader.read_line(&mut buf).await {
                Ok(0) => Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
                Ok(_) => Ok(extract_port(buf)?),
                Err(err) => Err(err),
            }
        });

        if num % 200 == 0 {
            interval.tick().await;
        }
    }

    let results = join_set.join_all().await;
    eprintln!("All tasks finished");

    let ports: Vec<String> = results.iter().map(|result| match result {
        Ok(port) => port.clone(),
        Err(err) => err.to_string().clone(),
    }).collect();

    println!("{}", ports.join(",").as_str());
    
    Ok(())
}

fn extract_port(buf: String) -> io::Result<String> {
    match buf.trim_end().split(" ").skip(2).next().ok_or(io::ErrorKind::InvalidData) {
        Ok(port) => Ok(port.to_string()),
        Err(err) => Err(err.into()),
    } // TODO: redundant
}

async fn listen(port: u16) -> io::Result<()> {
    let listener = TcpListener::bind(("::", port)).await?;

    eprintln!("listening on {port}");
    
    loop {
        match listener.accept().await {
            Ok((socket, address)) => _ = tokio::spawn(async move {
                respond(socket, address).await;
            }),
            Err(error) => eprintln!("couldn't accept: {:?}", error),
        }
    }
}

async fn respond(mut socket: TcpStream, address: SocketAddr) {
    let ip = address.ip().to_string();
    let port = address.port();
    _ = socket.write_all(format!("Hello {ip} {port}\n").as_bytes()).await;

    let (reader, mut writer) = socket.split();
    let mut line_reader = BufReader::new(reader);
    let mut buf = String::new();
    
    while let Ok(bytes_read) = line_reader.read_line(&mut buf).await {
        if bytes_read <= 0 {
            break;
        }
        match writer.write_all(buf.as_bytes()).await {
            Ok(_) => { buf.clear() }
            Err(_) => break
        }
    }
}
