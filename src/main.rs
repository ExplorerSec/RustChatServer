use std::net::SocketAddr;
use std::result::Result::Ok;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use tokio::select;
use tokio::signal::ctrl_c;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = "127.0.0.1:15963";
    let (tx, _) = broadcast::channel::<(String,SocketAddr)>(20);
    let tcp_listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Server Running on {}",addr);
    let cancel_token = CancellationToken::new();
    let token = cancel_token.clone();
    tokio::spawn(async move {
        match ctrl_c().await {
            Ok(_) => {
                println!("Shutdown Tasks!");
                token.cancel();
                //return ;
            }
            Err(err) => {
                println!("Err: {:?}", err);
            }
        };
    });
    loop {
        select! {
            _ = cancel_token.cancelled() =>{
                tokio::time::sleep(Duration::from_millis(500)).await;
                break;
            }
            Ok((mut tcp_stream, addr)) = tcp_listener.accept() =>{
                let tx = tx.clone();
                let mut rx = tx.subscribe();
                let token = cancel_token.clone();
                tokio::spawn(async move {
                    let (r, mut w) = tcp_stream.split();
                    let mut reader = BufReader::new(r);
                    let mut msg = String::new();
                    loop {
                        select! {
                            Ok(n) = reader.read_line(&mut msg) =>{
                                if n!=0{
                                   tx.send((msg.clone(),addr)).expect("Tx Send ERR");
                                   msg.clear();
                                }
                            }
                            Ok((recv_msg,broadcast_addr)) = rx.recv() => {
                                if broadcast_addr!=addr{
                                    w.write_all(recv_msg.as_bytes()).await.expect("Socket WriteBack ERR");
                                }
                               
                            }
                            _ = token.cancelled() =>{
                                tx.send(("Server Shutdown....".into(),addr)).expect("token.cancelled ERR");
                                break ;
                            }
                        }
                    }
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use std::time::Duration;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_broadcast() {
        let (tx, mut rx) = broadcast::channel::<String>(20);
        tokio::spawn(async move {
            loop {
                if let Ok(msg) = rx.recv().await {
                    println!("Recv: {}", msg);
                }
            }
        });
        let mut handles = Vec::new();
        for i in 0..8 {
            let tx = tx.clone();
            handles.push(tokio::spawn(async move {
                for j in 0..10 {
                    tx.send(format!("From:{}, num:{}", i, j)).expect("Send Err");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }));
        }
        for handle in handles {
            let _ = handle.await;
        }
    }
}
