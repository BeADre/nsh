//! Async tasks of components ([`super::Context::spawn`]): futures run on the
//! engine thread (they may hold the component's `!Send` state) by a small
//! executor of the core, polled inside a tokio runtime's context, so the
//! tokio ecosystem works in them: `tokio::time` for timers, network and
//! file IO, channels; `Send` work goes to the runtime's threads
//! ([`AsyncCx::background`]). Wakers may fire on any thread: they wake the
//! engine. A task changes its component's state with
//! [`WeakComponent::update`], applied by the core between polls (no shared
//! borrows of the component). Dropping a [`Task`] cancels it; a
//! component's tasks end with it.

use std::{
    any::Any,
    cell::RefCell,
    collections::VecDeque,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
    task::{Context as TaskContext, Poll, Wake, Waker},
    time::{Duration, Instant},
};

use rustc_hash::FxHashMap;
use tokio::runtime::Handle;

use super::{Component, Context, element::ErasedCx};
use crate::NodeId;

type LocalFuture = Pin<Box<dyn Future<Output = ()>>>;

/// Called when a task is woken (from any thread): the engine wakes up.
pub(crate) type Notify = Arc<dyn Fn() + Send + Sync>;

/// Tasks woken since the last run.
#[derive(Default)]
struct Ready {
    ids: Mutex<Vec<u64>>,
    notify: Mutex<Option<Notify>>,
}

struct TaskWaker {
    id: u64,
    ready: Arc<Ready>,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.ready.ids.lock().expect("ready lock").push(self.id);
        let notify = self.ready.notify.lock().expect("notify lock").clone();
        if let Some(notify) = notify {
            notify();
        }
    }
}

/// A change of a component's state a task asked for; `None` when the
/// component is gone.
pub(crate) type UpdateFn = Box<dyn for<'a> FnOnce(Option<(&'a mut dyn Any, &'a mut ErasedCx)>)>;

pub(crate) struct Update {
    pub(crate) host: NodeId,
    pub(crate) run: UpdateFn,
}

struct Entry {
    host: NodeId,
    /// Out while it is polled.
    future: Option<LocalFuture>,
}

#[derive(Default)]
struct Inner {
    tasks: FxHashMap<u64, Entry>,
    next_task: u64,
    updates: VecDeque<Update>,
}

/// The executor of one core's component tasks.
#[derive(Default)]
pub(crate) struct Executor {
    inner: RefCell<Inner>,
    ready: Arc<Ready>,
    /// The runtime tasks are polled in the context of.
    runtime: RefCell<Option<Handle>>,
}

impl Executor {
    pub(crate) fn set_runtime(&self, runtime: Handle) {
        *self.runtime.borrow_mut() = Some(runtime);
    }

    pub(crate) fn set_notify(&self, notify: Notify) {
        *self.ready.notify.lock().expect("notify lock") = Some(notify);
    }

    fn spawn(&self, host: NodeId, future: LocalFuture) -> u64 {
        let id = {
            let mut inner = self.inner.borrow_mut();
            inner.next_task += 1;
            let id = inner.next_task;
            inner.tasks.insert(
                id,
                Entry {
                    host,
                    future: Some(future),
                },
            );
            id
        };
        // Polled at the next run, not inside the code that spawned it.
        self.ready.ids.lock().expect("ready lock").push(id);
        let notify = self.ready.notify.lock().expect("notify lock").clone();
        if let Some(notify) = notify {
            notify();
        }
        id
    }

    fn cancel(&self, id: u64) {
        let removed = self.inner.borrow_mut().tasks.remove(&id);
        // Dropped outside the borrow: the future may own other tasks.
        drop(removed);
    }

    /// Cancels the tasks of component `host`.
    pub(crate) fn cancel_host(&self, host: NodeId) {
        let removed: Vec<Entry> = {
            let mut inner = self.inner.borrow_mut();
            let ids: Vec<u64> = inner
                .tasks
                .iter()
                .filter(|(_, e)| e.host == host)
                .map(|(&id, _)| id)
                .collect();
            ids.iter().filter_map(|id| inner.tasks.remove(id)).collect()
        };
        drop(removed);
    }

    /// Polls the woken tasks; returns whether there were any.
    pub(crate) fn poll_ready(&self) -> bool {
        let mut ids = std::mem::take(&mut *self.ready.ids.lock().expect("ready lock"));
        if ids.is_empty() {
            return false;
        }
        ids.sort_unstable();
        ids.dedup();
        // tokio's timers, IO and `spawn` find their runtime here.
        let runtime = self.runtime.borrow().clone();
        let _context = runtime.as_ref().map(Handle::enter);
        for id in ids {
            let future = match self.inner.borrow_mut().tasks.get_mut(&id) {
                Some(entry) => entry.future.take(),
                None => None,
            };
            let Some(mut future) = future else { continue };
            let waker = Waker::from(Arc::new(TaskWaker {
                id,
                ready: self.ready.clone(),
            }));
            let done = future
                .as_mut()
                .poll(&mut TaskContext::from_waker(&waker))
                .is_ready();
            if done {
                self.inner.borrow_mut().tasks.remove(&id);
                drop(future);
            } else if let Some(entry) = self.inner.borrow_mut().tasks.get_mut(&id) {
                entry.future = Some(future);
            }
        }
        true
    }

    #[cfg(test)]
    pub(crate) fn task_count(&self) -> usize {
        self.inner.borrow().tasks.len()
    }

    /// The state changes tasks asked for.
    pub(crate) fn take_updates(&self) -> VecDeque<Update> {
        std::mem::take(&mut self.inner.borrow_mut().updates)
    }
}

/// A running task ([`super::Context::spawn`]). Dropping it cancels the
/// task (Solid's `clearTimeout`, for timers); [`Task::detach`] lets it run
/// on its own. Awaited in another task, it gives the task's output.
#[must_use = "a dropped task is cancelled: keep it, or detach it"]
pub struct Task<T> {
    executor: Weak<Executor>,
    id: u64,
    output: Rc<RefCell<Output<T>>>,
    detached: bool,
}

struct Output<T> {
    value: Option<T>,
    waker: Option<Waker>,
}

impl<T> std::fmt::Debug for Task<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl<T> Task<T> {
    /// Lets the task run to its end without this handle (it still ends
    /// with its component).
    pub fn detach(mut self) {
        self.detached = true;
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        if !self.detached
            && let Some(executor) = self.executor.upgrade()
        {
            executor.cancel(self.id);
        }
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<T> {
        let mut output = self.output.borrow_mut();
        match output.value.take() {
            Some(value) => Poll::Ready(value),
            None => {
                output.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// What a task's code gets besides its component: timers and background
/// work, on tokio (any tokio API works in a task as well).
#[derive(Clone, Copy, Debug, Default)]
pub struct AsyncCx {
    _private: (),
}

impl AsyncCx {
    /// Completes after `delay` (Solid: what `setTimeout` waits for):
    /// `tokio::time::sleep`.
    pub fn timer(&self, delay: Duration) -> tokio::time::Sleep {
        tokio::time::sleep(delay)
    }

    /// Completes at `at`.
    pub fn timer_until(&self, at: Instant) -> tokio::time::Sleep {
        tokio::time::sleep_until(at.into())
    }

    /// Runs `future` on the runtime's threads (`tokio::spawn`): `Send`
    /// work that should not hold the engine thread. Await the handle for
    /// its output.
    pub fn background<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        tokio::spawn(future)
    }

    /// Runs blocking `f` on the runtime's blocking threads
    /// (`tokio::task::spawn_blocking`).
    pub fn blocking<R: Send + 'static>(
        &self,
        f: impl FnOnce() -> R + Send + 'static,
    ) -> tokio::task::JoinHandle<R> {
        tokio::task::spawn_blocking(f)
    }
}

/// A component as its tasks see it: it may be gone by the time they run.
pub struct WeakComponent<C> {
    host: NodeId,
    executor: Weak<Executor>,
    _component: PhantomData<fn(&mut C)>,
}

impl<C> std::fmt::Debug for WeakComponent<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeakComponent")
            .field("host", &self.host)
            .finish_non_exhaustive()
    }
}

impl<C> Clone for WeakComponent<C> {
    fn clone(&self) -> Self {
        Self {
            host: self.host,
            executor: self.executor.clone(),
            _component: PhantomData,
        }
    }
}

impl<C: Component> WeakComponent<C> {
    /// Changes the component's state: `f` runs on the engine thread right
    /// after this poll, with the component and a [`Context`]
    /// (`cx.notify()` renders it again). Await it for `f`'s result, `None`
    /// when the component is gone; not awaited, it still runs.
    pub fn update<R: 'static>(
        &self,
        f: impl FnOnce(&mut C, &mut Context<'_, C>) -> R + 'static,
    ) -> Updated<R> {
        let slot = Rc::new(RefCell::new(Output {
            value: None,
            waker: None,
        }));
        let result = slot.clone();
        if let Some(executor) = self.executor.upgrade() {
            executor.inner.borrow_mut().updates.push_back(Update {
                host: self.host,
                run: Box::new(move |target| {
                    let value = target.and_then(|(state, cx)| {
                        state
                            .downcast_mut::<C>()
                            .map(|state| f(state, &mut Context::new(cx)))
                    });
                    let mut slot = result.borrow_mut();
                    slot.value = Some(value);
                    if let Some(waker) = slot.waker.take() {
                        waker.wake();
                    }
                }),
            });
        } else {
            slot.borrow_mut().value = Some(None);
        }
        Updated { slot }
    }
}

/// An update's result ([`WeakComponent::update`]).
#[must_use = "the update runs anyway; await it for its result"]
pub struct Updated<R> {
    slot: Rc<RefCell<Output<Option<R>>>>,
}

impl<R> std::fmt::Debug for Updated<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Updated").finish_non_exhaustive()
    }
}

impl<R> Future for Updated<R> {
    type Output = Option<R>;

    fn poll(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<R>> {
        let mut slot = self.slot.borrow_mut();
        match slot.value.take() {
            Some(value) => Poll::Ready(value),
            None => {
                slot.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// Starts `f(this, cx)` as a task of component `host`.
pub(crate) fn spawn<C, R, Fut>(
    executor: &Rc<Executor>,
    host: NodeId,
    f: impl FnOnce(WeakComponent<C>, AsyncCx) -> Fut,
) -> Task<R>
where
    C: Component,
    R: 'static,
    Fut: Future<Output = R> + 'static,
{
    let weak = Rc::downgrade(executor);
    let this = WeakComponent {
        host,
        executor: weak.clone(),
        _component: PhantomData,
    };
    let future = f(this, AsyncCx::default());
    let output = Rc::new(RefCell::new(Output {
        value: None,
        waker: None,
    }));
    let done = output.clone();
    let id = executor.spawn(
        host,
        Box::pin(async move {
            let value = future.await;
            let mut output = done.borrow_mut();
            output.value = Some(value);
            if let Some(waker) = output.waker.take() {
                waker.wake();
            }
        }),
    );
    Task {
        executor: weak,
        id,
        output,
        detached: false,
    }
}
