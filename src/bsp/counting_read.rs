use std::io::{Read, Result};

pub struct CountingRead<R> {
    inner: R,
    count: usize,
}

impl<R> CountingRead<R>
where
    R: Read,
{
    pub fn new(inner: R) -> Self {
        CountingRead {
            inner: inner,
            count: 0,
        }
    }

    pub fn into_inner(self) -> R {
        self.inner
    }

    pub fn bytes_read(&self) -> usize {
        self.count
    }
}

impl<R> Read for CountingRead<R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let res = self.inner.read(buf);
        if let Ok(size) = res {
            self.count += size
        }
        res
    }
}
