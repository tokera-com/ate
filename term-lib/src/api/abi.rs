use async_trait::async_trait;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use tokio::sync::mpsc;
use wasm_bus::abi::SerializationFormat;

use super::*;

#[derive(Debug, Clone)]
pub struct ConsoleRect {
    pub cols: u32,
    pub rows: u32,
}

pub struct ReqwestResponse {
    pub pos: usize,
    pub data: Option<Vec<u8>>,
    pub ok: bool,
    pub redirected: bool,
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
}

// This ABI implements a set of emulated operating system
// functions that are specific to a console session
#[async_trait]
pub trait ConsoleAbi
where
    Self: Send + Sync,
{
    /// Writes output to the console
    async fn stdout(&self, data: Vec<u8>);

    /// Writes output to the console
    async fn stderr(&self, data: Vec<u8>);

    /// Flushes the output to the console
    async fn flush(&self);

    /// Writes output to the log
    async fn log(&self, text: String);

    /// Gets the number of columns and rows in the terminal
    async fn console_rect(&self) -> ConsoleRect;

    /// Clears the terminal
    async fn cls(&self);

    /// Tell the process to exit (if it can)
    async fn exit(&self);
}

// This ABI implements a number of low level operating system
// functions that this terminal depends upon
#[async_trait]
pub trait SystemAbi
where
    Self: Send + Sync,
{
    /// Starts an asynchronous task that will run on a shared worker pool
    /// This task must not block the execution or it could cause a deadlock
    fn task_shared(
        &self,
        task: Box<
            dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + 'static,
        >,
    );

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool that has a stateful thread local variable
    /// It is ok for this task to block execution and any async futures within its scope
    fn task_stateful(
        &self,
        task: Box<
            dyn FnOnce(Rc<RefCell<ThreadLocal>>) -> Pin<Box<dyn Future<Output = ()> + 'static>>
                + Send
                + 'static,
        >,
    );

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    fn task_dedicated(
        &self,
        task: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static>,
    );

    /// Starts an asynchronous task on the current thread. This is useful for
    /// launching background work with variables that are not Send.
    fn task_local(&self, task: Pin<Box<dyn Future<Output = ()> + 'static>>);

    /// Puts the current thread to sleep for a fixed number of milliseconds
    fn sleep(&self, ms: u128) -> AsyncResult<()>;

    /// Fetches a data file from the local context of the process
    fn fetch_file(&self, path: &str) -> AsyncResult<Result<Vec<u8>, u32>>;

    /// Performs a HTTP or HTTPS request to a destination URL
    fn reqwest(
        &self,
        url: &str,
        method: &str,
        headers: Vec<(String, String)>,
        data: Option<Vec<u8>>,
    ) -> AsyncResult<Result<ReqwestResponse, u32>>;

    /// Make a web socket connection to a particular URL
    async fn web_socket(&self, url: &str) -> Result<Box<dyn WebSocketAbi>, String>;
}

// System call extensions that provide generics
#[async_trait]
pub trait SystemAbiExt {
    /// Starts an asynchronous task that will run on a shared worker pool
    /// This task must not block the execution or it could cause a deadlock
    /// The return value of the spawned thread can be read either synchronously
    /// or asynchronously
    fn spawn_shared<F, Fut>(&self, task: F) -> AsyncResult<Fut::Output>
    where
        F: FnOnce() -> Fut,
        F: Send + 'static,
        Fut: Future + Send + 'static,
        Fut::Output: Send;

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool that has a stateful thread local variable
    /// It is ok for this task to block execution and any async futures within its scope
    /// The return value of the spawned thread can be read either synchronously
    /// or asynchronously
    fn spawn_stateful<F, Fut>(&self, task: F) -> AsyncResult<Fut::Output>
    where
        F: FnOnce(Rc<RefCell<ThreadLocal>>) -> Fut,
        F: Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send;

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    /// The return value of the spawned thread can be read either synchronously
    /// or asynchronously
    fn spawn_dedicated<F, Fut>(&self, task: F) -> AsyncResult<Fut::Output>
    where
        F: FnOnce() -> Fut,
        F: Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send;

    /// Starts an asynchronous task that will run on a shared worker pool
    /// This task must not block the execution or it could cause a deadlock
    /// This is the fire-and-forget variet of spawning background work
    fn fork_shared<F, Fut>(&self, task: F)
    where
        F: FnOnce() -> Fut,
        F: Send + 'static,
        Fut: Future + Send + 'static,
        Fut::Output: Send;

    /// Attempts to send the message instantly however if that does not
    /// work it spawns a background thread and sends it there instead
    fn fork_send<T: Send + 'static>(&self, sender: &mpsc::Sender<T>, msg: T);

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool that has a stateful thread local variable
    /// It is ok for this task to block execution and any async futures within its scope
    /// This is the fire-and-forget variet of spawning background work
    fn fork_stateful<F, Fut>(&self, task: F)
    where
        F: FnOnce(Rc<RefCell<ThreadLocal>>) -> Fut,
        F: Send + 'static,
        Fut: Future + 'static;

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    /// This is the fire-and-forget variet of spawning background work
    fn fork_dedicated<F, Fut>(&self, task: F)
    where
        F: FnOnce() -> Fut,
        F: Send + 'static,
        Fut: Future + 'static;

    /// Starts an asynchronous task on the current thread. This is useful for
    /// launching background work with variables that are not Send.
    /// This is the fire-and-forget variet of spawning background work
    fn fork_local<F>(&self, task: F)
    where
        F: Future + 'static;
}

#[async_trait]
impl SystemAbiExt for dyn SystemAbi {
    fn spawn_shared<F, Fut>(&self, task: F) -> AsyncResult<Fut::Output>
    where
        F: FnOnce() -> Fut,
        F: Send + 'static,
        Fut: Future + Send + 'static,
        Fut::Output: Send,
    {
        let (tx_result, rx_result) = mpsc::channel(1);
        self.task_shared(Box::new(move || {
            let task = task();
            Box::pin(async move {
                let ret = task.await;
                let _ = tx_result.send(ret).await;
            })
        }));
        AsyncResult::new(SerializationFormat::Bincode, rx_result)
    }

    fn spawn_stateful<F, Fut>(&self, task: F) -> AsyncResult<Fut::Output>
    where
        F: FnOnce(Rc<RefCell<ThreadLocal>>) -> Fut,
        F: Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send,
    {
        let (tx_result, rx_result) = mpsc::channel(1);
        self.task_stateful(Box::new(move |thread_local| {
            let task = task(thread_local);
            Box::pin(async move {
                let ret = task.await;
                let _ = tx_result.send(ret).await;
            })
        }));
        AsyncResult::new(SerializationFormat::Bincode, rx_result)
    }

    fn spawn_dedicated<F, Fut>(&self, task: F) -> AsyncResult<Fut::Output>
    where
        F: FnOnce() -> Fut,
        F: Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send,
    {
        let (tx_result, rx_result) = mpsc::channel(1);
        self.task_dedicated(Box::new(move || {
            let task = task();
            Box::pin(async move {
                let ret = task.await;
                let _ = tx_result.send(ret).await;
            })
        }));
        AsyncResult::new(SerializationFormat::Bincode, rx_result)
    }

    fn fork_shared<F, Fut>(&self, task: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        F: Send + 'static,
        Fut: Future + Send + 'static,
        Fut::Output: Send,
    {
        self.task_shared(Box::new(move || {
            let task = task();
            Box::pin(async move {
                let _ = task.await;
            })
        }));
    }

    fn fork_send<T: Send + 'static>(&self, sender: &mpsc::Sender<T>, msg: T) {
        if let Err(mpsc::error::TrySendError::Full(msg)) = sender.try_send(msg) {
            let sender = sender.clone();
            self.task_shared(Box::new(move || {
                Box::pin(async move {
                    let _ = sender.send(msg).await;
                })
            }));
        }
    }

    fn fork_stateful<F, Fut>(&self, task: F)
    where
        F: FnOnce(Rc<RefCell<ThreadLocal>>) -> Fut,
        F: Send + 'static,
        Fut: Future + 'static,
    {
        self.task_stateful(Box::new(move |thread_local| {
            let task = task(thread_local);
            Box::pin(async move {
                let _ = task.await;
            })
        }));
    }

    fn fork_dedicated<F, Fut>(&self, task: F)
    where
        F: FnOnce() -> Fut,
        F: Send + 'static,
        Fut: Future + 'static,
    {
        self.task_dedicated(Box::new(move || {
            let task = task();
            Box::pin(async move {
                let _ = task.await;
            })
        }));
    }

    fn fork_local<F>(&self, task: F)
    where
        F: Future + 'static,
    {
        self.task_local(Box::pin(async move {
            let _ = task.await;
        }))
    }
}
