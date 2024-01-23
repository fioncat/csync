use std::sync::Arc;

use csync_proto::frame::{ClipboardFrame, DataFrame, Frame};

use crate::channel::ChannelHandler;

#[tokio::test]
async fn channel() {
    let ch = ChannelHandler::new().await;

    ch.register(Arc::new("test0".to_string())).await.unwrap();
    ch.register(Arc::new("test1".to_string())).await.unwrap();
    ch.register(Arc::new("test2".to_string())).await.unwrap();

    enum Operation {
        Push(&'static str, &'static str),
        Pull(Option<&'static str>),
    }

    let cases = vec![
        (
            "addr0",
            vec!["test0", "test1"],
            vec![
                Operation::Pull(None),
                Operation::Push("test1", "hello, test1"),
                Operation::Pull(Some("hello, test1")),
                Operation::Pull(None),
                Operation::Push("test0", "hello, test0"),
                Operation::Pull(Some("hello, test0")),
                Operation::Pull(None),
                Operation::Pull(None),
            ],
        ),
        (
            "addr1",
            vec!["test0", "test1", "test2"],
            vec![
                Operation::Pull(Some("hello, test0")),
                Operation::Pull(None),
                Operation::Push("test1", "from test1"),
                Operation::Push("test0", "from test0"),
                Operation::Pull(Some("from test0")),
                Operation::Pull(None),
                Operation::Push("test2", "from test2"),
                Operation::Pull(Some("from test2")),
                Operation::Pull(None),
            ],
        ),
        (
            "addr0",
            vec!["test0", "test1"],
            vec![Operation::Pull(Some("from test0")), Operation::Pull(None)],
        ),
    ];

    for (addr, subs, ops) in cases {
        for op in ops {
            match op {
                Operation::Push(publish, text) => push_channel(&ch, publish, text).await,
                Operation::Pull(expect) => {
                    let result = pull_channel(&ch, addr, subs.clone()).await;
                    assert_eq!(result, expect.map(|s| s.to_string()));
                }
            }
        }
    }

    ch.close(
        Arc::new("addr0".to_string()),
        None,
        Some(Arc::new(vec!["test0".to_string(), "test1".to_string()])),
    )
    .await
    .unwrap();
    ch.close(
        Arc::new("addr1".to_string()),
        None,
        Some(Arc::new(vec![
            "test0".to_string(),
            "test1".to_string(),
            "test2".to_string(),
        ])),
    )
    .await
    .unwrap();

    ch.close(
        Arc::new("".to_string()),
        Some(Arc::new("test0".to_string())),
        None,
    )
    .await
    .unwrap();
    ch.close(
        Arc::new("".to_string()),
        Some(Arc::new("test1".to_string())),
        None,
    )
    .await
    .unwrap();

    // TODO: How to test case that channel register more than once?
    ch.close(
        Arc::new("".to_string()),
        Some(Arc::new("test2".to_string())),
        None,
    )
    .await
    .unwrap();
}

async fn push_channel(ch: &ChannelHandler, publish: &str, text: &str) {
    ch.push(
        Arc::new(publish.to_string()),
        Frame::Push(DataFrame {
            from: None,
            digest: String::new(),
            data: csync_proto::frame::ClipboardFrame::Text(text.to_string()),
        }),
    )
    .await
    .unwrap();
}

async fn pull_channel(ch: &ChannelHandler, addr: &str, subs: Vec<&str>) -> Option<String> {
    let subs = subs.into_iter().map(|s| s.to_string()).collect();
    let frame = ch
        .pull(Arc::new(addr.to_string()), Arc::new(subs))
        .await
        .unwrap();

    match frame {
        Some(frame) => match frame.as_ref() {
            Frame::Push(frame) => match &frame.data {
                ClipboardFrame::Text(text) => Some(text.clone()),
                _ => panic!("unexpect frame type"),
            },
            _ => panic!("unexpect frame"),
        },
        None => None,
    }
}
