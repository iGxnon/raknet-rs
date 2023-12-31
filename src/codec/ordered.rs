use std::collections::HashMap;
use std::ops::AddAssign;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Buf;
use futures::{ready, Stream, StreamExt};
use pin_project_lite::pin_project;
use tracing::debug;

use crate::errors::CodecError;
use crate::packet::connected::{self, Frame};

const INITIAL_ORDERING_MAP_CAP: usize = 64;

struct Ordering<B> {
    map: HashMap<u32, Frame<B>>,
    read: u32,
}

impl<B> Default for Ordering<B> {
    fn default() -> Self {
        Self {
            map: HashMap::with_capacity(INITIAL_ORDERING_MAP_CAP),
            read: 0,
        }
    }
}

pin_project! {
    // Ordering layer, ordered the packets based on ordering_frame_index.
    pub(crate) struct Order<F, B> {
        #[pin]
        frame: F,
        // Max ordered channel that will be used in detailed protocol
        max_channels: usize,
        ordering: Vec<Ordering<B>>,
    }
}

pub(super) trait Ordered: Sized {
    fn ordered<B: Buf>(self, max_channels: usize) -> Order<Self, B>;
}

impl<T> Ordered for T {
    fn ordered<B: Buf>(self, max_channels: usize) -> Order<Self, B> {
        assert!(
            max_channels < usize::from(u8::MAX),
            "max channels should not be larger than u8::MAX"
        );
        assert!(max_channels > 0, "max_channels > 0");

        Order {
            frame: self,
            max_channels,
            ordering: std::iter::repeat_with(Ordering::default)
                .take(max_channels)
                .collect(),
        }
    }
}

impl<F, B> Stream for Order<F, B>
where
    F: Stream<Item = Result<connected::Packet<B>, CodecError>>,
{
    type Item = Result<connected::Packet<B>, CodecError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            let Some(packet) = ready!(this.frame.poll_next_unpin(cx)?) else {
                return Poll::Ready(None);
            };

            let connected::Packet::FrameSet(frame_set) = packet else {
                return Poll::Ready(Some(Ok(packet)));
            };

            let mut frames = None;
            let frames_len = frame_set.frames.len();
            for frame in frame_set.frames {
                if let Some(connected::Ordered {
                    frame_index,
                    channel,
                }) = frame.ordered.clone()
                {
                    let channel = usize::from(channel);
                    if channel >= *this.max_channels {
                        return Poll::Ready(Some(Err(CodecError::OrderedFrame(format!(
                            "channel {} >= max_channels {}",
                            channel, *this.max_channels
                        )))));
                    }
                    let ordering = this
                        .ordering
                        .get_mut(channel)
                        .expect("channel < max_channels");

                    match frame_index.0.cmp(&ordering.read) {
                        std::cmp::Ordering::Less => {
                            debug!("ignore old ordered frame index {frame_index}");
                            continue;
                        }
                        std::cmp::Ordering::Greater => {
                            ordering.map.insert(frame_index.0, frame);
                            continue;
                        }
                        std::cmp::Ordering::Equal => {
                            ordering.read.add_assign(1);
                        }
                    }

                    // then we got a frame index equal to read index, we could read it
                    frames
                        .get_or_insert_with(|| Vec::with_capacity(frames_len))
                        .push(frame);

                    // check if we could read more
                    while let Some(next) = ordering.map.remove(&ordering.read) {
                        ordering.read.add_assign(1);
                        frames
                            .get_or_insert_with(|| Vec::with_capacity(frames_len))
                            .push(next);
                    }

                    // we cannot read anymore
                    continue;
                }
                // the frameset which does not require ordered
                frames
                    .get_or_insert_with(|| Vec::with_capacity(frames_len))
                    .push(frame);
            }
            if let Some(frames) = frames {
                return Poll::Ready(Some(Ok(connected::Packet::FrameSet(connected::FrameSet {
                    frames,
                    ..frame_set
                }))));
            }
        }
    }
}

#[cfg(test)]
mod test {
    use bytes::Bytes;
    use futures::StreamExt;
    use futures_async_stream::stream;

    use super::*;
    use crate::errors::CodecError;
    use crate::packet::connected::{self, Flags, Frame, FrameSet, Ordered, Uint24le};

    fn frame_set(idx: impl IntoIterator<Item = (u8, u32)>) -> connected::Packet<Bytes> {
        connected::Packet::FrameSet(FrameSet {
            seq_num: Uint24le(0),
            frames: idx
                .into_iter()
                .map(|(channel, frame_index)| Frame {
                    flags: Flags::parse(0b011_11100),
                    reliable_frame_index: None,
                    seq_frame_index: None,
                    ordered: Some(Ordered {
                        frame_index: Uint24le(frame_index),
                        channel,
                    }),
                    fragment: None,
                    body: Bytes::new(),
                })
                .collect(),
        })
    }

    #[tokio::test]
    async fn test_ordered_works() {
        let frame = {
            #[stream]
            async {
                yield frame_set([(0, 1), (0, 0), (0, 2), (0, 4), (0, 3)]);
                yield frame_set([(1, 1)]);
            }
        };
        tokio::pin!(frame);

        let mut ordered = Order {
            frame: frame.map(Ok),
            max_channels: 10,
            ordering: std::iter::repeat_with(Ordering::default).take(10).collect(),
        };

        assert_eq!(
            ordered.next().await.unwrap().unwrap(),
            frame_set([(0, 0), (0, 1), (0, 2), (0, 3), (0, 4)])
        );
        assert!(ordered.next().await.is_none());
    }

    #[tokio::test]
    async fn test_ordered_channel_exceed() {
        let frame = {
            #[stream]
            async {
                yield frame_set([(10, 1)]);
            }
        };
        tokio::pin!(frame);

        let mut ordered = Order {
            frame: frame.map(Ok),
            max_channels: 10,
            ordering: std::iter::repeat_with(Ordering::default).take(10).collect(),
        };

        assert!(matches!(
            ordered.next().await.unwrap().unwrap_err(),
            CodecError::OrderedFrame(_)
        ));
    }
}
