#[cfg(not(feature = "async"))]
mod sync {
    use std::io::{self, BufRead, Read, Stdout, Write};
    use crate::{
        process::unix::{PtyStream, UnixProcess},
        session::{Session, OsProcess},
        stream::{log::LogStream, StreamSink},
        Captures, Needle,
    };
    
    // TODO(tmpfs): fix this type for windows
    type LogSession = Session<UnixProcess, LogStream<PtyStream, Stdout>>;

    /// Wraps a session that may be logged to stdout.
    #[derive(Debug)]
    pub enum PtySession {
        /// Default pty session.
        Default(Session),
        /// Pty session that logs to stdout.
        Logged(LogSession),
    }

    impl PtySession {
        /// Get a reference to a process running program.
        pub fn get_process(&self) -> &OsProcess {
            match self {
                PtySession::Default(s) => s.get_process(),
                PtySession::Logged(s) => s.get_process(),
            }
        }
    }

    impl StreamSink for PtySession {
        fn send<B: AsRef<[u8]>>(&mut self, buf: B) -> io::Result<()> {
            match self {
                PtySession::Default(s) => s.send(buf),
                PtySession::Logged(s) => s.send(buf),
            }
        }

        fn send_line(&mut self, text: &str) -> io::Result<()> {
            match self {
                PtySession::Default(s) => s.send_line(text),
                PtySession::Logged(s) => s.send_line(text),
            }
        }

        fn expect<N>(
            &mut self,
            needle: N,
        ) -> Result<Captures, crate::Error>
        where
            N: Needle,
        {
            match self {
                PtySession::Default(s) => s.expect(needle),
                PtySession::Logged(s) => s.expect(needle),
            }
        }
    }

    impl Write for PtySession {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            match self {
                PtySession::Default(s) => s.write(buf),
                PtySession::Logged(s) => s.write(buf),
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            match self {
                PtySession::Default(s) => s.flush(),
                PtySession::Logged(s) => s.flush(),
            }
        }
    }

    impl BufRead for PtySession {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            match self {
                PtySession::Default(s) => s.fill_buf(),
                PtySession::Logged(s) => s.fill_buf(),
            }
        }

        fn consume(&mut self, amt: usize) {
            match self {
                PtySession::Default(s) => s.consume(amt),
                PtySession::Logged(s) => s.consume(amt),
            }
        }
    }

    impl Read for PtySession {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            match self {
                PtySession::Default(s) => s.read(buf),
                PtySession::Logged(s) => s.read(buf),
            }
        }
    }
}

#[cfg(feature = "async")]
mod async_pty {
    use std::io::{self, BufRead, Read, Stdout, Write, Result};
    use crate::{
        process::unix::{AsyncPtyStream, UnixProcess},
        session::{Session, OsProcess},
        stream::{log::LogStream, StreamSink},
        Captures, Needle,
    };
    use std::{task::{Poll, Context}, pin::Pin};
    use futures_lite::{AsyncRead, AsyncWrite, AsyncBufRead};

    // TODO(tmpfs): fix this type for windows
    type LogSession = Session<UnixProcess, LogStream<AsyncPtyStream, Stdout>>;

    /// Wraps a session that may be logged to stdout.
    #[derive(Debug)]
    pub enum PtySession {
        /// Default pty session.
        Default(Session),
        /// Pty session that logs to stdout.
        Logged(LogSession),
    }

    impl PtySession {
        /// Get a reference to a process running program.
        pub fn get_process(&self) -> &OsProcess {
            match self {
                PtySession::Default(s) => s.get_process(),
                PtySession::Logged(s) => s.get_process(),
            }
        }
    }

    #[async_trait::async_trait(?Send)]
    impl StreamSink for PtySession {
        async fn send<B: AsRef<[u8]>>(&mut self, buf: B) -> Result<()> {
            match self {
                PtySession::Default(s) => s.send(buf).await,
                PtySession::Logged(s) => s.send(buf).await,
            }
        }

        async fn send_line(&mut self, text: &str) -> Result<()> {
            match self {
                PtySession::Default(s) => s.send_line(text).await,
                PtySession::Logged(s) => s.send_line(text).await,
            }
        }

        async fn expect<N>(
            &mut self,
            needle: N,
        ) -> std::result::Result<Captures, crate::Error>
        where
            N: Needle,
        {
            match self {
                PtySession::Default(s) => s.expect(needle).await,
                PtySession::Logged(s) => s.expect(needle).await,
            }
        }
    }

    impl AsyncWrite for PtySession {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize>> {
            match &mut *self {
                PtySession::Default(s) => {
                    Pin::new(s).poll_write(cx, buf)
                },
                PtySession::Logged(s) => {
                    Pin::new(s).poll_write(cx, buf)
                },
            }
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
            match &mut *self {
                PtySession::Default(s) => {
                    Pin::new(s).poll_flush(cx)
                },
                PtySession::Logged(s) => {
                    Pin::new(s).poll_flush(cx)
                },
            }
        }

        fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
            match &mut *self {
                PtySession::Default(s) => {
                    Pin::new(s).poll_close(cx)
                },
                PtySession::Logged(s) => {
                    Pin::new(s).poll_close(cx)
                },
            }
        }
    }

    impl AsyncRead for PtySession {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<Result<usize>> {
            match &mut *self {
                PtySession::Default(s) => {
                    Pin::new(s).poll_read(cx, buf)
                },
                PtySession::Logged(s) => {
                    Pin::new(s).poll_read(cx, buf)
                },
            }
        }
    }

    impl AsyncBufRead for PtySession {
        fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<&[u8]>> {
            let this = self.get_mut();
            match this {
                PtySession::Default(s) => {
                    Pin::new(s).poll_fill_buf(cx)
                },
                PtySession::Logged(s) => {
                    Pin::new(s).poll_fill_buf(cx)
                },
            }
        }

        fn consume(mut self: Pin<&mut Self>, amt: usize) {
            match &mut *self {
                PtySession::Default(s) => {
                    Pin::new(s).consume(amt)
                },
                PtySession::Logged(s) => {
                    Pin::new(s).consume(amt)
                },
            }
        }
    }
}

#[cfg(not(feature = "async"))]
pub use sync::PtySession;

#[cfg(feature = "async")]
pub use async_pty::PtySession;
