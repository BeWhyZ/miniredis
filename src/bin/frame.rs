use bytes::Bytes;
use tokio::net::TcpStream;



enum Frame {
    Simple(String),
    Error(String),
    Integer(u64),
    Bulk(Bytes),
    Null,
    Array(Vec<Frame>),
}


