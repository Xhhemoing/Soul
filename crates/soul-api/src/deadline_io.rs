//! A response write deadline starts only when the completed read is sent.
//! Timing out a socket write never cancels or detaches a Session read worker.
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::time::Sleep;

pub(crate) struct DeadlineIo<T> {
    inner: T,
    write_deadline: Option<Pin<Box<Sleep>>>,
}

impl<T> DeadlineIo<T> {
    pub(crate) fn new(inner: T) -> Self {
        Self {
            inner,
            write_deadline: None,
        }
    }

    fn check_write_deadline(&mut self, context: &mut Context<'_>) -> io::Result<()> {
        let deadline = self
            .write_deadline
            .get_or_insert_with(|| Box::pin(tokio::time::sleep(crate::WRITE_TIMEOUT)));
        if deadline.as_mut().poll(context).is_ready() {
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "response write deadline",
            ))
        } else {
            Ok(())
        }
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for DeadlineIo<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_read(context, buffer)
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for DeadlineIo<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        if let Err(error) = this.check_write_deadline(context) {
            return Poll::Ready(Err(error));
        }
        Pin::new(&mut this.inner).poll_write(context, bytes)
    }
    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if this.write_deadline.is_some() {
            if let Err(error) = this.check_write_deadline(context) {
                return Poll::Ready(Err(error));
            }
        }
        Pin::new(&mut this.inner).poll_flush(context)
    }
    fn poll_shutdown(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if this.write_deadline.is_some() {
            if let Err(error) = this.check_write_deadline(context) {
                return Poll::Ready(Err(error));
            }
        }
        Pin::new(&mut this.inner).poll_shutdown(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct PendingWriter;
    impl AsyncWrite for PendingWriter {
        fn poll_write(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            _: &[u8],
        ) -> Poll<io::Result<usize>> {
            Poll::Pending
        }
        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Pending
        }
        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Pending
        }
    }

    #[test]
    fn a_peer_that_never_accepts_response_bytes_has_a_finite_write_lifetime() {
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        executor.block_on(async {
            let mut writer = DeadlineIo::new(PendingWriter);
            let result = std::future::poll_fn(|context| {
                Pin::new(&mut writer).poll_write(context, b"synthetic")
            })
            .await;
            assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
        });
    }
}
