use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use bytes::{BufMut, Bytes, BytesMut};
use mini_redis::{Frame, Result};
use mini_redis::frame::Error::Incomplete;
use std::io::Cursor;


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



struct Connection {
    stream: BufWriter<TcpStream>,
    // buffer: BytesMut,
    buffer: Vec<u8>,
    cursor: usize,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        // Connection {
        //     stream,
        //     buffer: BytesMut::with_capacity(4096),
        // }

        Connection {
            stream: BufWriter::new(stream),
            buffer: vec![0; 4096],
            cursor: 0,
        }
    }

    pub fn parse_frame(&mut self) -> Result<Option<Frame>> {
        // 创建 `T: Buf` 类型
        let mut buf = Cursor::new(&self.buffer[..]);
        // 检查是否读取了足够解析出一个帧的数据
        match Frame::check(&mut buf) {
            Ok(_) => {
                // 获取组成该帧的字节数
                let len = buf.position() as usize;

                // 在解析开始之前，重置内部的游标位置
                buf.set_position(0);

                // parse frame
                let frame = Frame::parse(&mut buf)?;

                // 解析完成，将缓冲区该帧的数据移除
                unsafe {
                    self.buffer.advance_mut(len);
                }

                // 返回解析出的帧
                Ok(Some(frame))

            }
            Err(Incomplete) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        loop {
            // 尝试从缓冲区的数据中解析出一个数据帧，
            // 只有当数据足够被解析时，才返回对应的帧
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame))
            }

            // // 如果缓冲区中的数据还不足以被解析为一个数据帧，
            // // 那么我们需要从 socket 中读取更多的数据
            // //
            // // 读取成功时，会返回读取到的字节数，0 代表着读到了数据流的末尾
            // if 0 == self.stream.read_buf(&mut self.buffer).await? {
            //     // 代码能执行到这里，说明了对端关闭了连接，
            //     // 需要看看缓冲区是否还有数据，若没有数据，说明所有数据成功被处理，
            //     // 若还有数据，说明对端在发送帧的过程中断开了连接，导致只发送了部分数据
            //     if self.buffer.is_empty() {
            //         return Ok(None);
            //     } else {
            //         return Err("connection reset by peer".into());
            //     }
            // }


            // using read and vec to implement

            // 确保缓冲区长度足够
            if self.buffer.len() == self.cursor {
                self.buffer.resize(self.cursor * 2, 0);
            }
            // 从游标位置开始将数据读入缓冲区
            let n = self.stream.read(&mut self.buffer[self.cursor..]).await?;

            if 0 == n {
                if self.cursor == 0 {
                    return Ok(None);
                } else {
                    return Err("connection reset by peer".into());
                }
            } else {
                // 更新游标位置
                self.cursor += n;
            }

        }
    }

    pub async fn write_frame(&mut self, frame: &Frame) -> Result<()> {

        match frame {
            Frame::Simple(val) => {
                self.stream.write_u8(b'+').await?;
                self.stream.write_all(val.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Error(val) => {
                self.stream.write_u8(b'-').await?;
                self.stream.write_all(val.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Integer(val) => {
                self.stream.write_u8(b':').await?;
                self.write_decimal(*val).await?;
            }
            Frame::Null => self.stream.write_all(b"$-1\r\n").await?,
            Frame::Bulk(val) => {
                let len = val.len();

                self.stream.write_u8(b'$').await?;
                self.write_decimal(len as u64).await?;
                self.stream.write_all(val).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Array(_val) => unimplemented!(),
        }

        self.stream.flush().await;
    
        Ok(())
    }

}
