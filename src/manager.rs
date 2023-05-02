use bytes::BytesMut;
use std::{io, sync::Arc};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    select,
    sync::{mpsc, oneshot},
};

use tokio_util::sync::CancellationToken;

async fn _send_routine<T: AsyncWriteExt + Unpin>(
    writer: WriteHalf<T>,
    send_rx: oneshot::Receiver<mpsc::Receiver<BytesMut>>,
    cancel_token: CancellationToken,
) -> io::Result<()> {
    let mut writer = writer;
    let mut send_rx = send_rx.await.unwrap();

    loop {
        select! {
            _ = cancel_token.cancelled() => {
                println!("gracefully shut!");
                return Ok(())
            }

            maybe_data = send_rx.recv() => {
                if let Some(data) = maybe_data {
                    writer.write_all(&data).await?;
                }
            }
        }
    }
}

async fn _recv_routine<T: AsyncReadExt + Unpin>(
    reader: oneshot::Receiver<ReadHalf<T>>,
    recv_tx: Arc<mpsc::Sender<BytesMut>>,
    cancel_token: CancellationToken,
) -> io::Result<()> {
    let mut reader = reader.await.unwrap();
    loop {
        let mut buffer = BytesMut::with_capacity(1024);
        select! {
            _ = cancel_token.cancelled() => {
                return Ok(())
            }

            maybe_size = reader.read_buf(&mut buffer) => {
                println!("debug buf: {:?}, size: {:?} ", buffer, maybe_size);

                match maybe_size {

                    Ok(size) => {
                        if size == 0 {
                            println!("Somethibg bad happened!");
                        }
                        buffer.truncate(size);

                        recv_tx.send(buffer).await.unwrap();
                    },
                    Err(error) => return Err(error),
                }
            }
        }
    }
}

pub async fn control_loop<
    T: AsyncReadExt + AsyncWriteExt + Unpin + std::fmt::Debug + std::marker::Send + 'static,
>(
    stream: T,
    keep_alive: bool,

) -> io::Result<()> {
    let cancellation_token = CancellationToken::new();

    let (reader, writer) = tokio::io::split(stream);
    let (recv_tx, mut recv_rx) = mpsc::channel::<BytesMut>(10);

    let (send_tx, send_rx) = mpsc::channel::<BytesMut>(10);

    let recv_tx = Arc::new(recv_tx);

    let (reader_sender, reader_receiver) = oneshot::channel();
    reader_sender.send(reader).unwrap();
    let mut reader_end = tokio::spawn(_recv_routine(
        reader_receiver,
        recv_tx,
        cancellation_token.clone(),
    ));

    let (writer_tx, writer_rx) = oneshot::channel();
    writer_tx.send(send_rx).unwrap();
    let mut writer_end = tokio::spawn(_send_routine(writer, writer_rx, cancellation_token.clone()));

    let mut shutdown = false;

    let mut result = Ok(());
    loop {
        select! {

            reader_end_s = &mut reader_end, if !shutdown =>  {
                match reader_end_s {
                    Ok(res) => {shutdown = true; result = res;},
                    Err(_) => todo!(),
                }
            }

            writer_end_s = &mut  writer_end, if !shutdown => {
                match writer_end_s {
                    Ok(res) => println!("Writer finished: {:?}", res),
                    Err(_) => todo!(),
                }
            }

            _ = tokio::time::sleep(std::time::Duration::from_secs(1)), if !shutdown => {

                let alive_byte = BytesMut::from("00003bit");
                if keep_alive {
                    send_tx.send(alive_byte).await.unwrap();
                }

            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(1)), if shutdown => {
                cancellation_token.cancel();
                break;
            }
        }
    }

    return result;
}