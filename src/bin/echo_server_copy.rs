use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;


// #[tokio::main]
// async fn main() -> io::Result<()>{
//     let listener = TcpListener::bind("127.0.0.1::6142").await?;

//     loop {
//         let (mut socket , _) = listener.accept().await?;

//         // let (mut rd, mut wr) = io::split(socket);
//         tokio::spawn(async move {
//             let (mut rd, mut wr) = socket.split();

//             wr.write_all(b"hello\r\n").await?;
//             wr.write_all(b"world\r\n").await?;

//             if io::copy(&mut rd, &mut wr).await.is_err() {
//                 eprintln!("failed to copy")
//             }
//         });
//     }
// }

#[tokio::main]
async fn main() -> io::Result<()>{
    let listener = TcpListener::bind("127.0.0.1::6142").await?;

    loop {
        let (mut socket , _) = listener.accept().await?;

        // let (mut rd, mut wr) = io::split(socket);
        tokio::spawn(async move {
            let mut buf = vec![0; 1024];

            loop {
                match socket.try_read(&mut buf) {
                    // reader is done
                    Ok(0) => return,
                    Ok(n) => {
                        // copy the data back to socket
                        if socket.write_all(&buf[..n]).await.is_err() {
                            // 
                            return;
                        }
                    },
                    Err(_) => {
                        return;
                    },

                }
            }
        });
    }
}
