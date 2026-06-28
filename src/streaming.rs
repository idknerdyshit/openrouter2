use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::OpenRouterError;
#[cfg(feature = "async")]
use crate::error::reqwest_error_message;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SseMessage<T> {
    Data(T),
    Done,
    Raw { event: Option<String>, data: Value },
}

#[cfg(feature = "async")]
pub type AsyncSseStream<T> = std::pin::Pin<
    Box<dyn futures_core::Stream<Item = Result<SseMessage<T>, OpenRouterError>> + Send>,
>;

#[cfg(feature = "async")]
pub(crate) fn decode_async_sse<T>(resp: reqwest::Response) -> AsyncSseStream<T>
where
    T: DeserializeOwned + Send + 'static,
{
    use futures_util::StreamExt as _;

    let stream = async_stream::try_stream! {
        let mut bytes = resp.bytes_stream();
        let mut pending = Vec::new();
        while let Some(chunk) = bytes.next().await {
            let chunk = chunk.map_err(|e| OpenRouterError::Transport(reqwest_error_message(&e)))?;
            pending.extend_from_slice(&chunk);

            while let Some((index, delimiter_len)) = find_sse_frame_delimiter(&pending) {
                let frame = pending[..index].to_vec();
                pending.drain(..index + delimiter_len);
                let raw = String::from_utf8(frame).map_err(|e| OpenRouterError::Decode(e.to_string()))?;
                if let Some(event) = parse_sse_event::<T>(&raw)? {
                    yield event;
                }
            }
        }
        if pending.iter().any(|byte| !byte.is_ascii_whitespace()) {
            let raw = String::from_utf8(pending).map_err(|e| OpenRouterError::Decode(e.to_string()))?;
            if let Some(event) = parse_sse_event::<T>(&raw)? {
                yield event;
            }
        }
    };

    Box::pin(stream)
}

#[cfg(feature = "async")]
fn find_sse_frame_delimiter(buf: &[u8]) -> Option<(usize, usize)> {
    let lf = buf
        .windows(2)
        .position(|window| window == b"\n\n")
        .map(|index| (index, 2));
    let crlf = buf
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| (index, 4));

    match (lf, crlf) {
        (Some(lf), Some(crlf)) => Some(if lf.0 <= crlf.0 { lf } else { crlf }),
        (Some(lf), None) => Some(lf),
        (None, Some(crlf)) => Some(crlf),
        (None, None) => None,
    }
}

#[cfg(feature = "blocking")]
pub struct BlockingSseStream<T> {
    reader: std::io::BufReader<reqwest::blocking::Response>,
    pending: String,
    finished: bool,
    marker: std::marker::PhantomData<T>,
}

#[cfg(feature = "blocking")]
impl<T> BlockingSseStream<T> {
    pub(crate) fn new(resp: reqwest::blocking::Response) -> Self {
        Self {
            reader: std::io::BufReader::new(resp),
            pending: String::new(),
            finished: false,
            marker: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "blocking")]
impl<T> Iterator for BlockingSseStream<T>
where
    T: DeserializeOwned,
{
    type Item = Result<SseMessage<T>, OpenRouterError>;

    fn next(&mut self) -> Option<Self::Item> {
        use std::io::BufRead as _;

        if self.finished {
            return None;
        }

        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => {
                    self.finished = true;
                    if self.pending.trim().is_empty() {
                        return None;
                    }
                    let raw = std::mem::take(&mut self.pending);
                    return parse_sse_event::<T>(&raw).transpose();
                }
                Ok(_) => {
                    if line.trim().is_empty() {
                        if self.pending.trim().is_empty() {
                            continue;
                        }
                        let raw = std::mem::take(&mut self.pending);
                        if let Some(event) = match parse_sse_event::<T>(&raw) {
                            Ok(event) => event,
                            Err(err) => return Some(Err(err)),
                        } {
                            return Some(Ok(event));
                        }
                    } else {
                        self.pending.push_str(&line);
                    }
                }
                Err(err) => {
                    self.finished = true;
                    return Some(Err(OpenRouterError::Transport(err.to_string())));
                }
            }
        }
    }
}

fn parse_sse_event<T>(raw: &str) -> Result<Option<SseMessage<T>>, OpenRouterError>
where
    T: DeserializeOwned,
{
    let mut event_name = None;
    let mut data_lines = Vec::new();

    for line in raw.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        if let Some(value) = line.strip_prefix("event:") {
            event_name = Some(value.trim_start().to_owned());
        } else if let Some(value) = line.strip_prefix("data:") {
            data_lines.push(value.trim_start().to_owned());
        }
    }

    if data_lines.is_empty() {
        return Ok(None);
    }

    let data = data_lines.join("\n");
    if data.trim() == "[DONE]" {
        return Ok(Some(SseMessage::Done));
    }

    let value: Value =
        serde_json::from_str(&data).map_err(|e| OpenRouterError::Decode(e.to_string()))?;
    match serde_json::from_value::<T>(value.clone()) {
        Ok(parsed) => Ok(Some(SseMessage::Data(parsed))),
        Err(_) => Ok(Some(SseMessage::Raw {
            event: event_name,
            data: value,
        })),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{SseMessage, parse_sse_event};

    #[cfg(feature = "async")]
    use super::find_sse_frame_delimiter;

    #[test]
    fn parses_done_event() {
        let event = parse_sse_event::<Value>("data: [DONE]\n").unwrap();
        assert_eq!(event, Some(SseMessage::Done));
    }

    #[test]
    fn parses_json_data_event() {
        let event = parse_sse_event::<Value>("event: message\ndata: {\"x\":1}\n")
            .unwrap()
            .unwrap();
        match event {
            SseMessage::Data(value) => assert_eq!(value["x"], 1),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[cfg(feature = "async")]
    #[test]
    fn finds_lf_and_crlf_frame_delimiters() {
        assert_eq!(find_sse_frame_delimiter(b"data: {}\n\n"), Some((8, 2)));
        assert_eq!(find_sse_frame_delimiter(b"data: {}\r\n\r\n"), Some((8, 4)));
    }

    #[test]
    fn parses_crlf_json_data_event() {
        let event = parse_sse_event::<Value>("event: message\r\ndata: {\"text\":\"hi\"}\r\n")
            .unwrap()
            .unwrap();
        match event {
            SseMessage::Data(value) => assert_eq!(value["text"], "hi"),
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
