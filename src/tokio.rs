use std::{future::Future, thread};

use ::tokio::{
    runtime::{Builder, Handle, Runtime},
    task,
};
use futures::future::pending;
use once_cell::sync::OnceCell;
use pyo3::prelude::*;

use crate::generic;

/// <span class="module-item stab portability" style="display: inline; border-radius: 3px; padding: 2px; font-size: 80%; line-height: 1.2;"><code>attributes</code></span>
/// re-exports for macros
#[cfg(feature = "attributes")]
pub mod re_exports {
    /// re-export pending to be used in tokio macros without additional dependency
    pub use futures::future::pending;
    /// re-export tokio::runtime to build runtimes in tokio macros without additional dependency
    pub use tokio::runtime;
}

/// <span class="module-item stab portability" style="display: inline; border-radius: 3px; padding: 2px; font-size: 80%; line-height: 1.2;"><code>attributes</code></span>
#[cfg(feature = "attributes")]
pub use pyo3_asyncio_macros::tokio_main as main;

/// <span class="module-item stab portability" style="display: inline; border-radius: 3px; padding: 2px; font-size: 80%; line-height: 1.2;"><code>attributes</code></span>
/// <span class="module-item stab portability" style="display: inline; border-radius: 3px; padding: 2px; font-size: 80%; line-height: 1.2;"><code>testing</code></span>
/// Registers a `tokio` test with the `pyo3-asyncio` test harness
#[cfg(all(feature = "attributes", feature = "testing"))]
pub use pyo3_asyncio_macros::tokio_test as test;

static TOKIO_RUNTIME_HANDLE: OnceCell<Handle> = OnceCell::new();

const EXPECT_TOKIO_INIT: &str = "Tokio runtime must be initialized";

impl generic::JoinError for task::JoinError {
    fn is_panic(&self) -> bool {
        task::JoinError::is_panic(self)
    }
}

struct TokioRuntime;

impl generic::Runtime for TokioRuntime {
    type JoinError = task::JoinError;
    type JoinHandle = task::JoinHandle<()>;

    fn spawn<F>(fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        get_handle().spawn(async move {
            fut.await;
        })
    }
}

/// Initialize the Tokio Runtime with a custom build
pub fn init(runtime: Handle) {
    TOKIO_RUNTIME_HANDLE
        .set(runtime)
        .expect("Tokio Runtime has already been initialized");
}

fn current_thread() -> Runtime {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Couldn't build the current-thread Tokio runtime")
}

fn start_current_thread() {
    thread::spawn(move || {
        TOKIO_RUNTIME_HANDLE
            .get()
            .unwrap()
            .block_on(pending::<()>());
    });
}

/// Initialize the Tokio Runtime with current-thread scheduler
///
/// # Panics
/// This function will panic if called a second time. See [`init_current_thread_once`] if you want
/// to avoid this panic.
pub fn init_current_thread() {
    init(current_thread().handle().clone());
    start_current_thread();
}

/// Get a reference to the current tokio runtime
pub fn get_handle<'a>() -> &'a Handle {
    TOKIO_RUNTIME_HANDLE.get().expect(EXPECT_TOKIO_INIT)
}

fn multi_thread() -> Runtime {
    Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Couldn't build the multi-thread Tokio runtime")
}

/// Initialize the Tokio Runtime with the multi-thread scheduler
///
/// # Panics
/// This function will panic if called a second time. See [`init_multi_thread_once`] if you want to
/// avoid this panic.
pub fn init_multi_thread() {
    init(multi_thread().handle().clone());
}

/// Ensure that the Tokio Runtime is initialized
///
/// If the runtime has not been initialized already, the multi-thread scheduler
/// is used. Calling this function a second time is a no-op.
pub fn init_multi_thread_once() {
    TOKIO_RUNTIME_HANDLE.get_or_init(|| multi_thread().handle().clone());
}

/// Ensure that the Tokio Runtime is initialized
///
/// If the runtime has not been initialized already, the current-thread
/// scheduler is used. Calling this function a second time is a no-op.
pub fn init_current_thread_once() {
    let mut initialized = false;
    TOKIO_RUNTIME_HANDLE.get_or_init(|| {
        initialized = true;
        current_thread().handle().clone()
    });

    if initialized {
        start_current_thread();
    }
}

/// Run the event loop until the given Future completes
///
/// The event loop runs until the given future is complete.
///
/// After this function returns, the event loop can be resumed with either [`run_until_complete`] or
/// [`crate::run_forever`]
///
/// # Arguments
/// * `py` - The current PyO3 GIL guard
/// * `fut` - The future to drive to completion
///
/// # Examples
///
/// ```
/// # use std::time::Duration;
/// #
/// # use pyo3::prelude::*;
/// #
/// # #[tokio::main]
/// # async fn main() {
/// #   pyo3_asyncio::tokio::init(tokio::runtime::Handle::current());
/// #
/// #   Python::with_gil(|py| {
/// #       pyo3_asyncio::with_runtime(py, || {
/// #           pyo3_asyncio::tokio::run_until_complete(py, async move {
/// #             tokio::time::sleep(Duration::from_secs(1)).await;
/// #             Ok(())
/// #           })
/// #       }) 
/// #   }).unwrap() 
/// # }
/// ```
pub fn run_until_complete<F>(py: Python, fut: F) -> PyResult<()>
where
    F: Future<Output = PyResult<()>> + Send + 'static,
{
    generic::run_until_complete::<TokioRuntime, _>(py, fut)
}

/// Convert a Rust Future into a Python coroutine
///
/// # Arguments
/// * `py` - The current PyO3 GIL guard
/// * `fut` - The Rust future to be converted
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use pyo3::prelude::*;
///
/// /// Awaitable sleep function
/// #[pyfunction]
/// fn sleep_for(py: Python, secs: &PyAny) -> PyResult<PyObject> {
///     let secs = secs.extract()?;
///
///     pyo3_asyncio::tokio::into_coroutine(py, async move {
///         tokio::time::sleep(Duration::from_secs(secs)).await;
///         Python::with_gil(|py| Ok(py.None()))
///     })
/// }
/// ```
pub fn into_coroutine<F>(py: Python, fut: F) -> PyResult<PyObject>
where
    F: Future<Output = PyResult<PyObject>> + Send + 'static,
{
    generic::into_coroutine::<TokioRuntime, _>(py, fut)
}
