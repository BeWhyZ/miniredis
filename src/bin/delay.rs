use std::pin::Pin;
use std::future::Future;
use std::task::{Context, Poll};
use std::time::{Instant, Duration};


struct Delay {
    when: Instant,
}


impl Future for Delay {
    type Output  = &'static str;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() == self.when {
            println!("hello world");
            Poll::Ready("done")
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}


#[tokio::main]
async fn main() {
    let when = Instant::now() + Duration::from_millis(10);
    let future = Delay{ when};

    let out = future.await;
    assert_eq!(out, "done");
}

