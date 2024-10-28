use std::hash::DefaultHasher;
use std::pin::Pin;
use std::future::Future;
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::{Instant, Duration};
use std::collections::VecDeque;
use crossbeam::channel;
use std::sync::{Arc, Mutex};
use futures::task::{self, ArcWake};


struct Delay {
    when: Instant,
    waker: Option<Arc<Mutex<Waker>>>,
}


impl Future for Delay {
    type Output  = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // 若这是 Future 第一次被调用，那么需要先生成一个计时器线程。
        // 若不是第一次调用(该线程已在运行)，那要确保已存储的 `Waker` 跟当前任务的 `waker` 匹配
        if let Some(waker) = &self.waker {
            let mut waker = waker.lock().unwrap();

            // 检查之前存储的 `waker` 是否跟当前任务的 `waker` 相匹配.
            // 这是必要的，原因是 `Delay Future` 的实例可能会在两次 `poll` 之间被转移到另一个任务中，然后
            // 存储的 waker 被该任务进行了更新。
            // 这种情况一旦发生，`Context` 包含的 `waker` 将不同于存储的 `waker`。
            // 因此我们必须对存储的 `waker` 进行更新
            if !waker.will_wake(cx.waker()) {
                *waker = cx.waker().clone();
            }
        } else {
            let when = self.when;
            let waker = Arc::new(Mutex::new(cx.waker().clone()));
            self.waker = Some(waker.clone());
            thread::spawn(move ||{
                let now  = Instant::now();
                if when > now {
                    thread::sleep(when-now);
                }
                // 计时结束，通过调用 `waker` 来通知执行器
                let waker = waker.lock().unwrap();
                waker.wake_by_ref();
            });
        }

        // 一旦 waker 被存储且计时器线程已经开始，我们就需要检查 `delay` 是否已经完成
        // 若计时已完成，则当前 Future 就可以完成并返回 `Poll::Ready`
        if Instant::now() >= self.when {
            Poll::Ready(())
        } else {
            // 计时尚未结束，Future 还未完成，因此返回 `Poll::Pending`.
            //
            // `Future` 特征要求当 `Pending` 被返回时，那我们要确保当资源准备好时，必须调用 `waker` 以通
            // 知执行器。 在我们的例子中，会通过生成的计时线程来保证
            //
            // 如果忘记调用 waker， 那等待我们的将是深渊：该任务将被永远的挂起，无法再执行
            Poll::Pending
        }
    }
}


fn main() {
    let mut mini_tokio = MiniTokio::new();

    mini_tokio.spawn(async move {
        let when = Instant::now() + Duration::from_secs(3);
        let future = Delay {when};

        let out = future.await;
        assert_eq!(out, "done");

    });

    mini_tokio.run();
}


struct MiniTokio {
    scheduled: channel::Receiver<Arc<Task>>,
    sender: channel::Sender<Arc<Task>>,
}

struct Task {
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
    executor: channel::Sender<Arc<Task>>,
}

impl Task {
    fn schedule(self: &Arc<Self>) {
        self.executor.send(self.clone());
    }

    fn poll(self: Arc<Self>) {
        let waker = task::waker(self.clone());
        let mut cx = Context::from_waker(&waker);

        let mut future = self.future.try_lock().unwrap();
        let _ = future.as_mut().poll(&mut cx);
    }

    fn spawn<F>(future: F, sender: &channel::Sender<Arc<Task>>)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task = Arc::new(Self{
            future: Mutex::new(Box::pin(future)),
            executor: sender.clone(),
        });

        let _ = sender.send(task);
        
    }
}

// using vTable
impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.schedule();
    }
}


impl MiniTokio {
    fn new() -> Self {
        let (sender, scheduled) = channel::unbounded();

        MiniTokio {
            scheduled,
            sender,
        }
    }

    // spawn, generating a future, and put it to deque
    fn spawn<F>(&mut self, future: F)
    where
        F: Future<Output = ()> + Send + 'static
    {
        Task::spawn(future, &self.sender)
    }

    // run
    fn run(&self) {
        while let Ok(task) = self.scheduled.recv() {
            task.poll()
        }
    }
}