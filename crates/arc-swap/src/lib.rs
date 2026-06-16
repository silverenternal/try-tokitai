use std::ops::Deref;
use std::sync::{Arc, RwLock};

/// Minimal compatibility guard for the subset of arc-swap APIs used by FileKV.
pub struct Guard<T> {
    inner: T,
}

impl<T> Guard<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> Deref for Guard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct ArcSwap<T> {
    inner: RwLock<Arc<T>>,
}

impl<T> ArcSwap<T> {
    pub fn new(value: Arc<T>) -> Self {
        Self {
            inner: RwLock::new(value),
        }
    }

    pub fn load(&self) -> Guard<Arc<T>> {
        Guard::new(self.inner.read().unwrap().clone())
    }

    pub fn store(&self, value: Arc<T>) {
        *self.inner.write().unwrap() = value;
    }

    pub fn swap(&self, value: Arc<T>) -> Arc<T> {
        std::mem::replace(&mut *self.inner.write().unwrap(), value)
    }

    pub fn into_inner(self) -> Arc<T> {
        self.inner.into_inner().unwrap()
    }
}

pub struct ArcSwapOption<T> {
    inner: RwLock<Option<Arc<T>>>,
}

impl<T> ArcSwapOption<T> {
    pub fn new(value: Option<Arc<T>>) -> Self {
        Self {
            inner: RwLock::new(value),
        }
    }

    pub fn empty() -> Self {
        Self::new(None)
    }

    pub fn load(&self) -> Guard<Option<Arc<T>>> {
        Guard::new(self.inner.read().unwrap().clone())
    }

    pub fn store(&self, value: Option<Arc<T>>) {
        *self.inner.write().unwrap() = value;
    }

    pub fn swap(&self, value: Option<Arc<T>>) -> Option<Arc<T>> {
        std::mem::replace(&mut *self.inner.write().unwrap(), value)
    }
}
