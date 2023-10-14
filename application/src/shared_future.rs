use std::{future::Future, pin::Pin, rc::Rc, task::Waker, sync::{Arc, Mutex}, ops::Deref, cell::RefCell};

use futures::future::{LocalBoxFuture, FutureExt};

enum SharedFutureSharedState<Output>{
    NotStarted,
    Running(Vec<Waker>),
    Finished(Rc<Output>),
}


struct SharedFutureShared<Output> {
    future: RefCell<LocalBoxFuture<'static, Output>>,
    state: Arc<Mutex<SharedFutureSharedState<Output>>>,
}

enum SharedFutureState<Output> {
    UnPolled,
    Runner,
    Waiter(usize), // the index of this SharedFuture in the Vec<Waker> of the SharedFutureSharedState::Running
    Finished(Rc<Output>),
}

impl<Output> Clone for SharedFutureState<Output> {
    fn clone(&self) -> Self {
        match self {
            Self::UnPolled => Self::UnPolled,
            Self::Runner => Self::Runner,
            Self::Waiter(idx) => Self::Waiter(*idx),
            Self::Finished(output) => Self::Finished(output.clone()),
        }
    }
}

pub struct SharedFuture<Output> {
    inner: Rc<SharedFutureShared<Output>>,
    state: SharedFutureState<Output>,
}

impl<Output> Clone for SharedFuture<Output> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            state: self.state.clone(),
        }
    }
}

impl<Output> SharedFutureShared<Output> {
    fn new(future: LocalBoxFuture<'static, Output>) -> Self {
        Self {
            future: RefCell::new(future),
            state: Arc::new(Mutex::new(SharedFutureSharedState::NotStarted))
        }
    }
}

impl<Output> SharedFuture<Output> {
    fn get_inner(self: &Self) -> &SharedFutureShared<Output> {
        &self.inner
    }

    fn get_state(self: &mut Self) -> &mut SharedFutureState<Output> {
        &mut self.state
    }

    pub fn new(future: LocalBoxFuture<'static, Output>) -> Self {
        Self {
            inner: Rc::new(SharedFutureShared::new(future)),
            state: SharedFutureState::UnPolled,
        }
    }

    fn lock_shared_state(&self) -> std::sync::MutexGuard<'_, SharedFutureSharedState<Output>> {
        self.inner.state.lock().unwrap()
    }
}


enum SharedStateUpdate<Output> {
    Noop,
    AddWaker(Waker),
    SetState(SharedFutureSharedState<Output>),
    SetWaker(Waker, usize),
    Finish(Rc<Output>),
}

struct PollResult<Output> {
    new_state: Option<SharedFutureState<Output>>,
    return_val: std::task::Poll<Rc<Output>>,
    shared_update: SharedStateUpdate<Output>,
}

impl<Output> Future for SharedFuture<Output> {
    type Output = Rc<Output>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {

        let this = self.as_ref().get_ref();

        let result: PollResult<Output> = match this.state {
            SharedFutureState::Finished(ref output) => {
                PollResult {
                    new_state: None,
                    return_val: std::task::Poll::Ready(output.clone()),
                    shared_update: SharedStateUpdate::Noop,
                }
            },
            SharedFutureState::UnPolled => {
                let mut shared_state = this.inner.state.lock().unwrap();
                match &mut *shared_state {
                    SharedFutureSharedState::NotStarted => {
                        PollResult {
                            new_state: Some(SharedFutureState::Runner),
                            return_val: std::task::Poll::Pending,
                            shared_update: SharedStateUpdate::SetState(SharedFutureSharedState::Running(Vec::new())),
                        }
                    },
                    SharedFutureSharedState::Running(wakers) => {
                        PollResult {
                            new_state: Some(SharedFutureState::Waiter(wakers.len())),
                            return_val: std::task::Poll::Pending,
                            shared_update: SharedStateUpdate::AddWaker(cx.waker().clone()),
                        }
                    },
                    SharedFutureSharedState::Finished(output) => {
                        PollResult {
                            new_state: Some(SharedFutureState::Finished(output.clone())),
                            return_val: std::task::Poll::Ready(output.clone()),
                            shared_update: SharedStateUpdate::Noop,
                        }
                    }
                }
            },
            SharedFutureState::Waiter(idx) => {
                let mut shared_state = this.inner.state.lock().unwrap();
                match &mut *shared_state {
                    SharedFutureSharedState::NotStarted => {
                        unreachable!();
                    },
                    SharedFutureSharedState::Running(wakers) => {
                        PollResult {
                            new_state: None,
                            return_val: std::task::Poll::Pending,
                            shared_update: SharedStateUpdate::SetWaker(cx.waker().clone(), idx),
                        }
                    },
                    SharedFutureSharedState::Finished(output) => {
                        PollResult {
                            new_state: Some(SharedFutureState::Finished(output.clone())),
                            return_val: std::task::Poll::Ready(output.clone()),
                            shared_update: SharedStateUpdate::Noop,
                        }
                    }
                }
            },
            SharedFutureState::Runner => {
                let result = this.inner.future.borrow_mut().poll_unpin(cx);
                match result {
                    std::task::Poll::Ready(output) => {
                        let rc = Rc::new(output);
                        PollResult {
                            new_state: Some(SharedFutureState::Finished(rc.clone())),
                            return_val: std::task::Poll::Ready(rc.clone()),
                            shared_update: SharedStateUpdate::Finish(rc),
                        }
                    },
                    std::task::Poll::Pending => {
                        PollResult{
                            new_state: None,
                            return_val: std::task::Poll::Pending,
                            shared_update: SharedStateUpdate::Noop,
                        }
                    },
                }
            }
        };

        
        // Update the shared state
        match result.shared_update {
            SharedStateUpdate::Noop => {},
            SharedStateUpdate::AddWaker(waker) => {
                let mut shared_state = this.lock_shared_state();
                match &mut *shared_state {
                    SharedFutureSharedState::NotStarted => {
                        unreachable!();
                    },
                    SharedFutureSharedState::Running(wakers) => {
                        wakers.push(waker);
                    },
                    SharedFutureSharedState::Finished(_) => {
                        unreachable!();
                    }
                }
            },
            SharedStateUpdate::SetState(state) => {
                let mut shared_state = this.lock_shared_state();
                *shared_state = state;
            },
            SharedStateUpdate::SetWaker(waker, idx) => {
                let mut shared_state = this.lock_shared_state();
                match &mut *shared_state {
                    SharedFutureSharedState::NotStarted => {
                        unreachable!();
                    },
                    SharedFutureSharedState::Running(wakers) => {
                        wakers[idx] = waker;
                    },
                    SharedFutureSharedState::Finished(_) => {
                        unreachable!();
                    }
                }
            },
            SharedStateUpdate::Finish(output) => {
                let mut shared_state = this.lock_shared_state();
                if let SharedFutureSharedState::Running(wakers) = &mut *shared_state {
                    for waker in wakers.drain(..) {
                        waker.wake();
                    }
                }
                *shared_state = SharedFutureSharedState::Finished(output);
            },
        }
        
        // Update this shared future's state
        if let Some(new_state) = result.new_state {
            unsafe {
                self.get_unchecked_mut().state = new_state;
            }
        }
        result.return_val
    }
}
