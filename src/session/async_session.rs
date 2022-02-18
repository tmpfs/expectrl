use std::{
    convert::TryInto,
    io,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures_lite::{AsyncBufRead, AsyncRead, AsyncWrite, AsyncWriteExt};

use super::async_stream::Stream;
use crate::{error::to_io_error, stream::log::LoggedStream, ControlCode, Error, Found, Needle};

/// Session represents a spawned process and its streams.
/// It controlls process and communication with it.
#[derive(Debug)]
pub struct Session<P, S> {
    process: P,
    stream: Stream<S>,
}

impl<P, S> Session<P, S> {
    /// Set logger.
    pub async fn with_log<W: io::Write>(
        self,
        logger: W,
    ) -> Result<Session<P, LoggedStream<S, W>>, Error> {
        let stream = self.stream.into_inner();
        let stream = LoggedStream::new(stream, logger);
        let session = Session::new(self.process, stream)?;
        Ok(session)
    }
}

// GEt back to the solution where Logger is just dyn Write instead of all these magic with type system.....

impl<P, S> Session<P, S> {
    pub fn new(process: P, stream: S) -> io::Result<Self> {
        Ok(Self {
            process,
            stream: Stream::new(stream),
        })
    }

    /// Set the pty session's expect timeout.
    pub fn set_expect_timeout(&mut self, expect_timeout: Option<Duration>) {
        self.stream.set_expect_timeout(expect_timeout);
    }
}

impl<P, S: AsyncRead + Unpin> Session<P, S> {
    pub async fn expect<N: Needle>(&mut self, needle: N) -> Result<Found, Error> {
        self.stream.expect(needle).await
    }

    /// Is matched checks if a pattern is matched.
    /// It doesn't consumes bytes from stream.
    pub async fn is_matched<E: Needle>(&mut self, needle: E) -> Result<bool, Error> {
        self.stream.is_matched(needle).await
    }

    /// Check checks if a pattern is matched.
    /// Returns empty found structure if nothing found.
    ///
    /// Is a non blocking version of [Session::expect].
    /// But its strategy of matching is different from it.
    /// It makes search agains all bytes available.
    ///
    /// ```
    /// # futures_lite::future::block_on(async {
    /// let mut p = expectrl::spawn("echo 123").unwrap();
    /// // wait to guarantee that check will successed (most likely)
    /// std::thread::sleep(std::time::Duration::from_secs(1));
    /// let m = p.check(expectrl::Regex("\\d+")).await.unwrap();
    /// assert_eq!(m.first(), b"123");
    /// # });
    /// ```
    pub async fn check<E: Needle>(&mut self, needle: E) -> Result<Found, Error> {
        self.stream.check(needle).await
    }

    /// Verifyes if stream is empty or not.
    pub async fn is_empty(&mut self) -> io::Result<bool> {
        self.stream.is_empty().await
    }
}

impl<P, S: AsyncWrite + Unpin> Session<P, S> {
    /// Send text to child's `STDIN`.
    ///
    /// To write bytes you can use a [std::io::Write] operations instead.
    pub async fn send<Str: AsRef<str>>(&mut self, s: Str) -> io::Result<()> {
        self.stream.write_all(s.as_ref().as_bytes()).await
    }

    /// Send a line to child's `STDIN`.
    pub async fn send_line<Str: AsRef<str>>(&mut self, s: Str) -> io::Result<()> {
        #[cfg(windows)]
        const LINE_ENDING: &[u8] = b"\r\n";
        #[cfg(not(windows))]
        const LINE_ENDING: &[u8] = b"\n";

        let buf = s.as_ref().as_bytes();
        let _ = self.stream.write_all(buf).await?;
        let _ = self.stream.write_all(LINE_ENDING).await?;
        self.stream.flush().await?;

        Ok(())
    }

    /// Send controll character to a child process.
    ///
    /// You must be carefull passing a char or &str as an argument.
    /// If you pass an unexpected controll you'll get a error.
    /// So it may be better to use [ControlCode].
    ///
    /// ```no_run
    /// use expectrl::{session::Session, ControlCode};
    /// use std::process::Command;
    ///
    /// # futures_lite::future::block_on(async {
    /// let mut process = Session::spawn(Command::new("cat")).unwrap();
    /// process.send_control(ControlCode::EndOfText).await.unwrap(); // sends CTRL^C
    /// process.send_control('C').await.unwrap(); // sends CTRL^C
    /// process.send_control("^C").await.unwrap(); // sends CTRL^C
    /// # });
    /// ```
    pub async fn send_control(&mut self, code: impl TryInto<ControlCode>) -> io::Result<()> {
        let code = code
            .try_into()
            .map_err(|_| to_io_error("Failed to parse a control character")(""))?;
        self.stream.write_all(&[code.into()]).await
    }
}

impl<P, S> Deref for Session<P, S> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.process
    }
}

impl<P, S> DerefMut for Session<P, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.process
    }
}

impl<P: Unpin, S: AsyncWrite + Unpin> AsyncWrite for Session<P, S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().stream).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_close(cx)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write_vectored(cx, bufs)
    }
}

impl<P: Unpin, S: AsyncRead + Unpin> AsyncRead for Session<P, S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}

impl<P: Unpin, S: AsyncRead + Unpin> AsyncBufRead for Session<P, S> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<&[u8]>> {
        Pin::new(&mut self.get_mut().stream).poll_fill_buf(cx)
    }

    fn consume(mut self: Pin<&mut Self>, amt: usize) {
        Pin::new(&mut self.stream).consume(amt);
    }
}
