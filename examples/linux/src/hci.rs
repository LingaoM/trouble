// SPDX-License-Identifier: MIT OR Apache-2.0

use core::convert::Infallible;
use std::io;
use std::path::PathBuf;

use bt_hci_transport::{PacketKind, PacketToController, PacketToHost, ReadHciError, WithIndicator};
use embedded_io_adapters::tokio_1::FromTokio;
use embedded_io_async::Write as _;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;
use tokio::sync::Mutex;

#[derive(Clone)]
pub(crate) enum Target {
    Bluez(u16),
    Unix(PathBuf),
}

impl Target {
    pub(crate) fn parse(value: Option<&str>) -> Result<Self, String> {
        let value = value.unwrap_or("0");
        if value.is_empty() {
            return Err("HCI target must not be empty".to_owned());
        }

        if value.bytes().all(|byte| byte.is_ascii_digit()) {
            return value
                .parse::<u16>()
                .map(Self::Bluez)
                .map_err(|_| format!("invalid BlueZ HCI device number: {value}"));
        }

        Ok(Self::Unix(PathBuf::from(value)))
    }
}

#[derive(Debug)]
pub enum Error {
    Bluez(bt_hci_linux::Error),
    Io(io::Error),
    HciIo(ReadHciError<io::Error>),
    HciData(ReadHciError<Infallible>),
}

impl core::fmt::Display for Error {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Bluez(error) => write!(formatter, "{error}"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::HciIo(error) => write!(formatter, "{error}"),
            Self::HciData(error) => write!(formatter, "{error}"),
        }
    }
}

impl core::error::Error for Error {}

impl embedded_io::Error for Error {
    fn kind(&self) -> embedded_io::ErrorKind {
        match self {
            Self::Bluez(error) => embedded_io::Error::kind(error),
            Self::Io(error) => error.kind().into(),
            Self::HciIo(error) => error.kind(),
            Self::HciData(error) => error.kind(),
        }
    }
}

impl From<ReadHciError<io::Error>> for Error {
    fn from(error: ReadHciError<io::Error>) -> Self {
        Self::HciIo(error)
    }
}

impl From<ReadHciError<Infallible>> for Error {
    fn from(error: ReadHciError<Infallible>) -> Self {
        Self::HciData(error)
    }
}

pub struct Transport(TransportInner);

enum TransportInner {
    Bluez(bt_hci_linux::Transport),
    Unix(UnixTransport),
}

impl Transport {
    pub(crate) async fn connect(target: Target) -> io::Result<Self> {
        match target {
            Target::Bluez(device) => {
                bt_hci_linux::Transport::new(device).map(|transport| Self(TransportInner::Bluez(transport)))
            }
            Target::Unix(path) => UnixTransport::connect(path)
                .await
                .map(|transport| Self(TransportInner::Unix(transport))),
        }
    }
}

impl embedded_io::ErrorType for Transport {
    type Error = Error;
}

impl bt_hci_transport::Transport for Transport {
    async fn read<'a, P: PacketToHost<'a>>(&self, buffer: &'a mut [u8]) -> Result<P, Self::Error> {
        match &self.0 {
            TransportInner::Bluez(transport) => bt_hci_transport::Transport::read(transport, buffer)
                .await
                .map_err(Error::Bluez),
            TransportInner::Unix(transport) => bt_hci_transport::Transport::read(transport, buffer).await,
        }
    }

    async fn write<P: PacketToController>(&self, packet: &P) -> Result<(), Self::Error> {
        match &self.0 {
            TransportInner::Bluez(transport) => bt_hci_transport::Transport::write(transport, packet)
                .await
                .map_err(Error::Bluez),
            TransportInner::Unix(transport) => bt_hci_transport::Transport::write(transport, packet).await,
        }
    }
}

struct UnixTransport {
    reader: Mutex<FromTokio<OwnedReadHalf>>,
    writer: Mutex<FromTokio<OwnedWriteHalf>>,
}

impl UnixTransport {
    async fn connect(path: PathBuf) -> io::Result<Self> {
        let (reader, writer) = UnixStream::connect(path).await?.into_split();
        Ok(Self {
            reader: Mutex::new(FromTokio::new(reader)),
            writer: Mutex::new(FromTokio::new(writer)),
        })
    }
}

impl embedded_io::ErrorType for UnixTransport {
    type Error = Error;
}

impl bt_hci_transport::Transport for UnixTransport {
    async fn read<'a, P: PacketToHost<'a>>(&self, buffer: &'a mut [u8]) -> Result<P, Self::Error> {
        let mut reader = self.reader.lock().await;
        let kind = PacketKind::read_async(&mut *reader).await?;
        P::read_hci_async(kind, &mut *reader, buffer).await.map_err(Into::into)
    }

    async fn write<P: PacketToController>(&self, packet: &P) -> Result<(), Self::Error> {
        let mut writer = self.writer.lock().await;
        WithIndicator::new(packet)
            .write_hci_async(&mut *writer)
            .await
            .map_err(Error::Io)?;
        writer.flush().await.map_err(Error::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::Target;

    #[test]
    fn parses_hci_target() {
        assert!(matches!(Target::parse(None), Ok(Target::Bluez(0))));
        assert!(matches!(Target::parse(Some("2")), Ok(Target::Bluez(2))));
        assert!(matches!(Target::parse(Some("/tmp/hci.sock")), Ok(Target::Unix(_))));
        assert!(Target::parse(Some("65536")).is_err());
    }
}
