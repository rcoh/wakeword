use std::backtrace::Backtrace;
use std::future::Future;
use std::sync::Arc;
use std::task::Context;

use futures_util::task::{ArcWake, AtomicWaker};
use tokio::runtime::Handle;
use tokio::runtime::Id;

struct WakeWarner {
    runtime_id: Id,
    waker: AtomicWaker,
}

impl WakeWarner {
    fn new(runtime_id: Id) -> Self {
        Self {
            runtime_id,
            waker: AtomicWaker::new(),
        }
    }
}

impl ArcWake for WakeWarner {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        eprintln!("waking! thread id: {:?}", std::thread::current().name());
        if let Ok(handle) = Handle::try_current() {
            println!("rt id: {} (us: {})", handle.id(), arc_self.runtime_id);
            if handle.id() != arc_self.runtime_id {
                eprintln!("cross thread wake! {}", Backtrace::force_capture());
            }
        }
        arc_self.waker.wake();
    }
}

pin_project_lite::pin_project! {
struct WakeInstrumented<F> {
    #[pin]
    f: F,
    waker: Arc<WakeWarner>,
}
}

pub fn instrument<F>(handle: &Handle, f: F) -> WakeInstrumented<F> {
    WakeInstrumented {
        f,
        waker: Arc::new(WakeWarner::new(handle.id())),
    }
}

impl<F: Future> Future for WakeInstrumented<F> {
    type Output = F::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        // Register the waker
        let this = self.project();
        let waker = this.waker;
        waker.waker.register(cx.waker());
        // Get the instrumented waker
        let waker_ref = futures_util::task::waker_ref(waker);
        let mut cx = Context::from_waker(&waker_ref);
        this.f.poll(&mut cx)
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::{instrument, WakeWarner};

    #[tokio::test]
    async fn test_cross_runtime_wakes() {
        let rt_1 = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name_fn(|| "rt-1".to_string())
            .build()
            .unwrap();
        let rt_2 = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name_fn(|| "rt-2".to_string())
            .build()
            .unwrap();

        let rt2_handle = rt_2.handle().clone();

        let result: Result<(), tokio::task::JoinError> = rt_1
            .spawn(instrument(rt_1.handle(), async move {
                let (tx, rx) = tokio::sync::oneshot::channel();

                rt2_handle.spawn(instrument(&rt2_handle, async move {
                    eprintln!("thread id: {:?}", std::thread::current().name());
                    tx.send(5).unwrap();
                }));
                eprintln!(
                    "waiting on rx thread id: {:?}",
                    std::thread::current().name()
                );
                rx.await.unwrap();
            }))
            .await;
        rt_1.shutdown_background();
        rt_2.shutdown_background();
    }
}
