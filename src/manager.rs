use bytes::{BytesMut, BufMut};
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
                log::debug!("gracefully shut!");
                return Ok(())
            }

            maybe_data = send_rx.recv() => {
                if let Some(data) = maybe_data {
                    let mut buf = BytesMut::with_capacity(4 + data.len());
                    buf.put_u32(data.len() as u32);
                    buf.put(data);
                    writer.write_all(&buf).await?;
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
        let mut buf_size = [0u8; 4];
        select! {
            _ = cancel_token.cancelled() => {
                return Ok(())
            }

            maybe_size = reader.read_exact(&mut buf_size) => {
                match maybe_size {
                    Ok(size) => {
                        if size == 0 {
                            log::error!("Somethibg bad happened!");
                        }
                        let size = u32::from_be_bytes(buf_size) as usize;
                        let mut buffer = BytesMut::with_capacity(size);
                        buffer.resize(size, 0u8);
                        reader.read_exact(&mut buffer).await?;
                        if recv_tx.send(buffer).await.is_err() {
                            return Ok(());
                        }
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
    send_back: mpsc::Sender<(mpsc::Receiver<BytesMut>, mpsc::Sender<BytesMut>)>
) -> io::Result<()> {
    let cancellation_token = CancellationToken::new();

    let (reader, writer) = tokio::io::split(stream);
    let (recv_tx, recv_rx) = mpsc::channel::<BytesMut>(10);

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

    send_back.send((recv_rx, send_tx.clone())).await.unwrap();

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
                    Ok(res) => {shutdown = true; result = res },
                    Err(_) => todo!(),
                }
            }

            _ = tokio::time::sleep(std::time::Duration::from_secs(1)), if !shutdown => {

                let alive_byte = BytesMut::from("bit");
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


pub async fn server_control_loop<
    T: AsyncReadExt + AsyncWriteExt + Unpin + std::fmt::Debug + std::marker::Send + 'static,
    >(
    stream: T,
    )
{
    let (tx, mut rx) = mpsc::channel(2);
    tokio::spawn(control_loop(stream, false, tx));

    let (mut recv, send) = rx.recv().await.unwrap();
    
    loop {
        select! {

            _ = tokio::time::sleep(std::time::Duration::from_millis(1000)) => {
                match recv.try_recv(){
                    Ok(d) => log::info!("data: {:?}", d),
                    Err(e) => log::warn!("error in scl: {:?}", e),
                }
                if send.send(BytesMut::from("I got you!")).await.is_err() {
                    break;
                }
            }
        }
    }

}